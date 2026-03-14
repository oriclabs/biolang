use bl_core::value::{BioSequence, Table, Value};
use bl_runtime::viz::call_viz_builtin;
use std::collections::HashMap;

fn make_table(cols: Vec<&str>, rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.into_iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

// ── Sparkline tests ─────────────────────────────────────────────

#[test]
fn test_sparkline_basic() {
    let list = Value::List(vec![
        Value::Float(1.0),
        Value::Float(3.0),
        Value::Float(7.0),
        Value::Float(2.0),
        Value::Float(5.0),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 5);
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_all_equal() {
    let list = Value::List(vec![
        Value::Float(5.0),
        Value::Float(5.0),
        Value::Float(5.0),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.chars().all(|c| c == '\u{2584}')); // '▄'
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_empty() {
    let list = Value::List(vec![]);
    let result = call_viz_builtin("sparkline", vec![list]);
    match result {
        Ok(Value::Str(s)) => assert_eq!(s, ""),
        _ => panic!("expected empty Str"),
    }
}

#[test]
fn test_sparkline_negative_values() {
    let list = Value::List(vec![
        Value::Float(-5.0),
        Value::Float(-3.0),
        Value::Float(-1.0),
        Value::Float(-4.0),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 4);
        // -1 is the max, should be the tallest block
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_single_value() {
    let list = Value::List(vec![Value::Float(42.0)]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 1);
        // Single value means span=0, so middle block
        assert!(s.chars().all(|c| c == '\u{2584}')); // '▄'
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_very_large_values() {
    let list = Value::List(vec![
        Value::Float(1e15),
        Value::Float(2e15),
        Value::Float(3e15),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 3);
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_wrong_type() {
    let result = call_viz_builtin("sparkline", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Bar chart tests ─────────────────────────────────────────────

#[test]
fn test_bar_chart_record() {
    let rec = Value::Record(HashMap::from([
        ("gene1".into(), Value::Int(100)),
        ("gene2".into(), Value::Int(50)),
    ]));
    let result = call_viz_builtin("bar_chart", vec![rec]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_bar_chart_table() {
    let table = make_table(
        vec!["name", "count"],
        vec![
            vec![Value::Str("a".into()), Value::Int(10)],
            vec![Value::Str("b".into()), Value::Int(20)],
        ],
    );
    let result = call_viz_builtin("bar_chart", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_bar_chart_wrong_type() {
    let result = call_viz_builtin("bar_chart", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_bar_chart_empty_record() {
    let rec = Value::Record(HashMap::new());
    let result = call_viz_builtin("bar_chart", vec![rec]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_bar_chart_many_items() {
    let mut map = HashMap::new();
    for i in 0..50 {
        map.insert(format!("item_{i}"), Value::Int(i));
    }
    let rec = Value::Record(map);
    // default limit is 20, should truncate
    let result = call_viz_builtin("bar_chart", vec![rec]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Boxplot tests ───────────────────────────────────────────────

#[test]
fn test_boxplot_list() {
    let list = Value::List(vec![
        Value::Float(1.0),
        Value::Float(2.0),
        Value::Float(3.0),
        Value::Float(4.0),
        Value::Float(5.0),
        Value::Float(6.0),
        Value::Float(7.0),
        Value::Float(8.0),
        Value::Float(9.0),
        Value::Float(10.0),
    ]);
    let result = call_viz_builtin("boxplot", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_boxplot_table() {
    let table = make_table(
        vec!["scores", "values"],
        vec![
            vec![Value::Float(1.0), Value::Float(10.0)],
            vec![Value::Float(5.0), Value::Float(20.0)],
            vec![Value::Float(3.0), Value::Float(15.0)],
            vec![Value::Float(7.0), Value::Float(25.0)],
        ],
    );
    let result = call_viz_builtin("boxplot", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_boxplot_all_identical_values() {
    let list = Value::List(vec![
        Value::Float(7.0),
        Value::Float(7.0),
        Value::Float(7.0),
        Value::Float(7.0),
        Value::Float(7.0),
    ]);
    let result = call_viz_builtin("boxplot", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_boxplot_single_value() {
    let list = Value::List(vec![Value::Float(42.0)]);
    let result = call_viz_builtin("boxplot", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_boxplot_empty_list_error() {
    let list = Value::List(vec![]);
    // empty list with no numbers triggers error from nums_from_value returning Ok(vec![])
    // then boxplot checks for empty
    let result = call_viz_builtin("boxplot", vec![list]);
    assert!(result.is_err());
}

#[test]
fn test_boxplot_wrong_type() {
    let result = call_viz_builtin("boxplot", vec![Value::Str("nope".into())]);
    assert!(result.is_err());
}

// ── Heatmap ASCII tests ────────────────────────────────────────

#[test]
fn test_heatmap_ascii_table() {
    let table = make_table(
        vec!["a", "b", "c"],
        vec![
            vec![Value::Float(0.0), Value::Float(5.0), Value::Float(10.0)],
            vec![Value::Float(3.0), Value::Float(1.0), Value::Float(7.0)],
        ],
    );
    let result = call_viz_builtin("heatmap_ascii", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_heatmap_ascii_matrix() {
    let mat = bl_core::matrix::Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let result = call_viz_builtin("heatmap_ascii", vec![Value::Matrix(mat)]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_heatmap_ascii_1x1_table() {
    let table = make_table(
        vec!["x"],
        vec![vec![Value::Float(5.0)]],
    );
    let result = call_viz_builtin("heatmap_ascii", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_heatmap_ascii_wrong_type() {
    let result = call_viz_builtin("heatmap_ascii", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Coverage tests ──────────────────────────────────────────────

#[test]
fn test_coverage_list() {
    let list = Value::List(
        (0..100)
            .map(|i| Value::Float((i as f64 * 0.1).sin().abs() * 10.0))
            .collect(),
    );
    let result = call_viz_builtin("coverage", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_coverage_bedgraph_table() {
    let table = make_table(
        vec!["chrom", "start", "end", "value"],
        vec![
            vec![Value::Str("chr1".into()), Value::Int(0), Value::Int(100), Value::Float(5.0)],
            vec![Value::Str("chr1".into()), Value::Int(100), Value::Int(200), Value::Float(10.0)],
            vec![Value::Str("chr2".into()), Value::Int(0), Value::Int(50), Value::Float(3.0)],
        ],
    );
    let result = call_viz_builtin("coverage", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_coverage_empty_list() {
    let list = Value::List(vec![]);
    // Empty list should still produce a result (empty sparkline)
    let result = call_viz_builtin("coverage", vec![list]);
    // nums_from_value returns Ok(vec![]) for empty list, then bin_values produces zeroes
    assert!(result.is_ok());
}

#[test]
fn test_coverage_wrong_type() {
    let result = call_viz_builtin("coverage", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Dotplot tests ───────────────────────────────────────────────

#[test]
fn test_dotplot_ascii() {
    let s1 = Value::DNA(BioSequence {
        data: "ATCGATCGATCGATCG".to_string(),
    });
    let s2 = Value::DNA(BioSequence {
        data: "ATCGATCGATCGATCG".to_string(),
    });
    let result = call_viz_builtin("dotplot", vec![s1, s2]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_dotplot_svg() {
    let s1 = Value::DNA(BioSequence {
        data: "ATCGATCGATCGATCG".to_string(),
    });
    let s2 = Value::DNA(BioSequence {
        data: "GATCGATCGATCGATC".to_string(),
    });
    let opts = Value::Record(HashMap::from([("format".into(), Value::Str("svg".into()))]));
    let result = call_viz_builtin("dotplot", vec![s1, s2, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<circle"));
    } else {
        panic!("expected Str with SVG");
    }
}

#[test]
fn test_dotplot_identical_sequences() {
    let seq = "ATCGATCGATCGATCGATCGATCG";
    let s1 = Value::DNA(BioSequence { data: seq.to_string() });
    let s2 = Value::DNA(BioSequence { data: seq.to_string() });
    // Identical sequences should produce a diagonal of matches
    let result = call_viz_builtin("dotplot", vec![s1, s2]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_dotplot_no_similarity() {
    let s1 = Value::DNA(BioSequence {
        data: "AAAAAAAAAAAAAAA".to_string(),
    });
    let s2 = Value::DNA(BioSequence {
        data: "TTTTTTTTTTTTTTT".to_string(),
    });
    // No k-mer matches between all-A and all-T
    let result = call_viz_builtin("dotplot", vec![s1, s2]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_dotplot_too_short_error() {
    let s1 = Value::DNA(BioSequence { data: "ATC".to_string() });
    let s2 = Value::DNA(BioSequence { data: "ATC".to_string() });
    // Default window=5, sequences of length 3 < 5
    let result = call_viz_builtin("dotplot", vec![s1, s2]);
    assert!(result.is_err());
}

#[test]
fn test_dotplot_wrong_type() {
    let result = call_viz_builtin("dotplot", vec![Value::Int(1), Value::Int(2)]);
    assert!(result.is_err());
}

// ── Alignment view tests ────────────────────────────────────────

#[test]
fn test_alignment_view_ascii() {
    let table = make_table(
        vec!["qname", "flag", "rname", "pos", "mapq", "cigar"],
        vec![
            vec![
                Value::Str("read1".into()),
                Value::Int(0),
                Value::Str("chr1".into()),
                Value::Int(100),
                Value::Int(60),
                Value::Str("50M".into()),
            ],
            vec![
                Value::Str("read2".into()),
                Value::Int(16),
                Value::Str("chr1".into()),
                Value::Int(120),
                Value::Int(60),
                Value::Str("30M".into()),
            ],
        ],
    );
    let result = call_viz_builtin("alignment_view", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_alignment_view_svg() {
    let table = make_table(
        vec!["qname", "flag", "rname", "pos", "mapq", "cigar"],
        vec![
            vec![
                Value::Str("read1".into()),
                Value::Int(0),
                Value::Str("chr1".into()),
                Value::Int(100),
                Value::Int(60),
                Value::Str("50M".into()),
            ],
            vec![
                Value::Str("read2".into()),
                Value::Int(16),
                Value::Str("chr1".into()),
                Value::Int(120),
                Value::Int(60),
                Value::Str("30M".into()),
            ],
        ],
    );
    let opts = Value::Record(HashMap::from([("format".into(), Value::Str("svg".into()))]));
    let result = call_viz_builtin("alignment_view", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str with SVG");
    }
}

#[test]
fn test_alignment_view_no_reads() {
    let table = make_table(
        vec!["qname", "flag", "rname", "pos", "mapq", "cigar"],
        vec![],
    );
    let result = call_viz_builtin("alignment_view", vec![table]).unwrap();
    // Empty table means no reads, should output "(no reads)"
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_alignment_view_wrong_type() {
    let result = call_viz_builtin("alignment_view", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Quality plot tests ──────────────────────────────────────────

#[test]
fn test_quality_plot_single() {
    let quals = Value::List(vec![
        Value::Int(35),
        Value::Int(30),
        Value::Int(25),
        Value::Int(15),
        Value::Int(40),
    ]);
    let result = call_viz_builtin("quality_plot", vec![quals]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_quality_plot_multi() {
    let read1 = Value::List(vec![Value::Int(35), Value::Int(30), Value::Int(25)]);
    let read2 = Value::List(vec![Value::Int(38), Value::Int(28), Value::Int(20)]);
    let read3 = Value::List(vec![Value::Int(40), Value::Int(32), Value::Int(10)]);
    let quals = Value::List(vec![read1, read2, read3]);
    let result = call_viz_builtin("quality_plot", vec![quals]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_quality_plot_svg() {
    let quals = Value::List(vec![
        Value::Int(35),
        Value::Int(30),
        Value::Int(25),
        Value::Int(15),
        Value::Int(40),
    ]);
    let opts = Value::Record(HashMap::from([("format".into(), Value::Str("svg".into()))]));
    let result = call_viz_builtin("quality_plot", vec![quals, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str with SVG");
    }
}

#[test]
fn test_quality_plot_edge_scores_zero() {
    let quals = Value::List(vec![
        Value::Int(0),
        Value::Int(0),
        Value::Int(0),
    ]);
    let result = call_viz_builtin("quality_plot", vec![quals]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_quality_plot_edge_scores_high() {
    let quals = Value::List(vec![
        Value::Int(41),
        Value::Int(42),
        Value::Int(40),
    ]);
    let result = call_viz_builtin("quality_plot", vec![quals]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_quality_plot_empty_list_error() {
    let quals = Value::List(vec![]);
    let result = call_viz_builtin("quality_plot", vec![quals]);
    assert!(result.is_err());
}

#[test]
fn test_quality_plot_wrong_type() {
    let result = call_viz_builtin("quality_plot", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Unknown builtin ─────────────────────────────────────────────

// ── Quality plot with PHRED string ─────────────────────────────

#[test]
fn test_quality_plot_phred_string() {
    // PHRED+33 encoded quality string
    let qual_str = Value::Str("IIIIIHHHGGFFE".into());
    let result = call_viz_builtin("quality_plot", vec![qual_str]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_quality_plot_phred_string_svg() {
    let qual_str = Value::Str("IIIIIHHHGGFFE".into());
    let opts = Value::Record(HashMap::from([("format".into(), Value::Str("svg".into()))]));
    let result = call_viz_builtin("quality_plot", vec![qual_str, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected SVG string");
    }
}

// ── Dotplot with protein sequences ────────────────────────────

#[test]
fn test_dotplot_protein() {
    let s1 = Value::Protein(BioSequence {
        data: "MVLSPADKTNVKAAWGKVGAHAG".to_string(),
    });
    let s2 = Value::Protein(BioSequence {
        data: "MVLSPADKTNVKAAWGKVGAHAG".to_string(),
    });
    let result = call_viz_builtin("dotplot", vec![s1, s2]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_dotplot_protein_svg() {
    let s1 = Value::Protein(BioSequence {
        data: "MVLSPADKTNVKAAWGKVGAHAG".to_string(),
    });
    let s2 = Value::Protein(BioSequence {
        data: "MVLSAADKTNVKAAWGKVGAHAG".to_string(),
    });
    let opts = Value::Record(HashMap::from([("format".into(), Value::Str("svg".into()))]));
    let result = call_viz_builtin("dotplot", vec![s1, s2, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected SVG");
    }
}

// ── Dotplot with custom options ───────────────────────────────

#[test]
fn test_dotplot_custom_window() {
    let s1 = Value::DNA(BioSequence {
        data: "ATCGATCGATCGATCGATCGATCGATCG".to_string(),
    });
    let s2 = Value::DNA(BioSequence {
        data: "ATCGATCGATCGATCGATCGATCGATCG".to_string(),
    });
    let opts = Value::Record(HashMap::from([
        ("window".into(), Value::Int(3)),
        ("threshold".into(), Value::Int(2)),
    ]));
    let result = call_viz_builtin("dotplot", vec![s1, s2, opts]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Alignment view MSA input ──────────────────────────────────

#[test]
fn test_alignment_view_msa_list() {
    // MSA as list of aligned strings
    let msa = Value::List(vec![
        Value::Str("ATCG-ATCG".into()),
        Value::Str("ATCGAATCG".into()),
        Value::Str("AT-GAATCG".into()),
    ]);
    let result = call_viz_builtin("alignment_view", vec![msa]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_alignment_view_msa_many() {
    // MSA with more sequences
    let msa = Value::List(vec![
        Value::Str("ATCG-ATCG".into()),
        Value::Str("ATCGAATCG".into()),
        Value::Str("AT-GAATCG".into()),
        Value::Str("ATCG-ATCG".into()),
    ]);
    let result = call_viz_builtin("alignment_view", vec![msa]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Alignment view with complex CIGAR ─────────────────────────

#[test]
fn test_alignment_view_complex_cigar() {
    let table = make_table(
        vec!["qname", "flag", "rname", "pos", "mapq", "cigar"],
        vec![
            vec![
                Value::Str("read1".into()),
                Value::Int(0),
                Value::Str("chr1".into()),
                Value::Int(100),
                Value::Int(60),
                Value::Str("10M2I20M3D15M".into()),
            ],
            vec![
                Value::Str("read2".into()),
                Value::Int(0),
                Value::Str("chr1".into()),
                Value::Int(110),
                Value::Int(40),
                Value::Str("25M5S".into()),
            ],
            vec![
                Value::Str("read3".into()),
                Value::Int(16),
                Value::Str("chr1".into()),
                Value::Int(95),
                Value::Int(60),
                Value::Str("60M".into()),
            ],
        ],
    );
    let result = call_viz_builtin("alignment_view", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Bar chart with options ────────────────────────────────────

#[test]
fn test_bar_chart_with_sort() {
    let rec = Value::Record(HashMap::from([
        ("alpha".into(), Value::Int(30)),
        ("beta".into(), Value::Int(10)),
        ("gamma".into(), Value::Int(50)),
        ("delta".into(), Value::Int(20)),
    ]));
    let opts = Value::Record(HashMap::from([
        ("sort".into(), Value::Bool(true)),
    ]));
    let result = call_viz_builtin("bar_chart", vec![rec, opts]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_bar_chart_with_limit() {
    let mut map = HashMap::new();
    for i in 0..50 {
        map.insert(format!("item_{i}"), Value::Int(i));
    }
    let rec = Value::Record(map);
    let opts = Value::Record(HashMap::from([
        ("limit".into(), Value::Int(10)),
    ]));
    let result = call_viz_builtin("bar_chart", vec![rec, opts]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_bar_chart_list_input() {
    let list = Value::List(vec![
        Value::Float(10.0),
        Value::Float(25.0),
        Value::Float(15.0),
        Value::Float(30.0),
    ]);
    let result = call_viz_builtin("bar_chart", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Boxplot with options ──────────────────────────────────────

#[test]
fn test_boxplot_many_values() {
    let list = Value::List(
        (0..1000).map(|i| Value::Float((i as f64 * 0.7).sin())).collect(),
    );
    let result = call_viz_builtin("boxplot", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_boxplot_two_values() {
    let list = Value::List(vec![Value::Float(1.0), Value::Float(10.0)]);
    let result = call_viz_builtin("boxplot", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Heatmap ASCII with options ────────────────────────────────

#[test]
fn test_heatmap_ascii_large_table() {
    let mut rows = Vec::new();
    for i in 0..10 {
        let mut row = Vec::new();
        for j in 0..10 {
            row.push(Value::Float((i * 10 + j) as f64));
        }
        rows.push(row);
    }
    let cols: Vec<&str> = vec!["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9"];
    let table = make_table(cols, rows);
    let result = call_viz_builtin("heatmap_ascii", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

#[test]
fn test_heatmap_ascii_negative_and_positive() {
    let table = make_table(
        vec!["a", "b"],
        vec![
            vec![Value::Float(-10.0), Value::Float(10.0)],
            vec![Value::Float(0.0), Value::Float(-5.0)],
        ],
    );
    let result = call_viz_builtin("heatmap_ascii", vec![table]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Coverage with interval pairs ──────────────────────────────

#[test]
fn test_coverage_interval_pairs() {
    // List of [start, end] pairs
    let list = Value::List(vec![
        Value::List(vec![Value::Int(0), Value::Int(100)]),
        Value::List(vec![Value::Int(50), Value::Int(150)]),
        Value::List(vec![Value::Int(200), Value::Int(300)]),
    ]);
    let result = call_viz_builtin("coverage", vec![list]).unwrap();
    assert!(matches!(result, Value::Nil));
}

// ── Sparkline edge cases ──────────────────────────────────────

#[test]
fn test_sparkline_with_integers() {
    let list = Value::List(vec![
        Value::Int(1),
        Value::Int(5),
        Value::Int(3),
        Value::Int(8),
        Value::Int(2),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 5);
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_sparkline_mixed_int_float() {
    let list = Value::List(vec![
        Value::Int(1),
        Value::Float(2.5),
        Value::Int(4),
        Value::Float(0.5),
    ]);
    let result = call_viz_builtin("sparkline", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert_eq!(s.chars().count(), 4);
    } else {
        panic!("expected Str");
    }
}

// ── Unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_viz_builtin() {
    let result = call_viz_builtin("nonexistent", vec![]);
    assert!(result.is_err());
}
