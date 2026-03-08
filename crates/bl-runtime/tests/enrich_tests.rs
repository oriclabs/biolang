use bl_core::value::{Table, Value};
use bl_runtime::enrich::call_enrich_builtin;
use std::collections::HashMap;

// ── Helpers ──────────────────────────────────────────────────────

fn gene_list(genes: &[&str]) -> Value {
    Value::List(genes.iter().map(|g| Value::Str(g.to_string())).collect())
}

fn gene_sets(sets: Vec<(&str, Vec<&str>)>) -> Value {
    let mut map = HashMap::new();
    for (name, genes) in sets {
        map.insert(
            name.to_string(),
            Value::List(genes.iter().map(|g| Value::Str(g.to_string())).collect()),
        );
    }
    Value::Map(map)
}

fn ranked_table(genes_scores: Vec<(&str, f64)>) -> Value {
    let rows: Vec<Vec<Value>> = genes_scores
        .into_iter()
        .map(|(g, s)| vec![Value::Str(g.to_string()), Value::Float(s)])
        .collect();
    Value::Table(Table::new(
        vec!["gene".to_string(), "score".to_string()],
        rows,
    ))
}

// ── enrich (ORA) ────────────────────────────────────────────────

#[test]
fn test_enrich_known_overlap() {
    let genes = gene_list(&["A", "B", "C"]);
    let sets = gene_sets(vec![
        ("set1", vec!["A", "B", "D"]),
        ("set2", vec!["X", "Y"]),
        ("set3", vec!["A", "B", "C"]),
    ]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(20000)]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.columns[0], "term");
        // Check p_values are in [0, 1]
        let p_idx = t.col_index("p_value").unwrap();
        for row in &t.rows {
            if let Value::Float(p) = &row[p_idx] {
                assert!(*p >= 0.0 && *p <= 1.0, "p={p}");
            }
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_enrich_no_overlap() {
    let genes = gene_list(&["X", "Y"]);
    let sets = gene_sets(vec![("set1", vec!["A", "B"])]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(20000)]).unwrap();
    if let Value::Table(t) = result {
        let p_idx = t.col_index("p_value").unwrap();
        if let Value::Float(p) = &t.rows[0][p_idx] {
            assert!(
                (*p - 1.0).abs() < 1e-10,
                "p={p} should be 1.0 for no overlap"
            );
        }
    }
}

#[test]
fn test_enrich_empty_gene_list() {
    let genes = gene_list(&[]);
    let sets = gene_sets(vec![("set1", vec!["A", "B"])]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(20000)]).unwrap();
    if let Value::Table(t) = result {
        // No genes means no overlap for any set
        let overlap_idx = t.col_index("overlap").unwrap();
        for row in &t.rows {
            if let Value::Int(o) = &row[overlap_idx] {
                assert_eq!(*o, 0, "empty gene list should have 0 overlap");
            }
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_enrich_all_genes_matching() {
    // All query genes are in the gene set => strong enrichment (small p-value)
    let genes = gene_list(&["A", "B", "C", "D", "E"]);
    let sets = gene_sets(vec![("perfect_set", vec!["A", "B", "C", "D", "E"])]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(20000)]).unwrap();
    if let Value::Table(t) = result {
        let p_idx = t.col_index("p_value").unwrap();
        let overlap_idx = t.col_index("overlap").unwrap();
        if let Value::Float(p) = &t.rows[0][p_idx] {
            assert!(*p < 0.01, "perfect overlap should have very small p-value, got {p}");
        }
        if let Value::Int(o) = &t.rows[0][overlap_idx] {
            assert_eq!(*o, 5, "all 5 genes should overlap");
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_enrich_gene_sets_no_overlap() {
    // Query genes exist but none match any gene set
    let genes = gene_list(&["X", "Y", "Z"]);
    let sets = gene_sets(vec![
        ("s1", vec!["A", "B"]),
        ("s2", vec!["C", "D"]),
    ]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(1000)]).unwrap();
    if let Value::Table(t) = result {
        let p_idx = t.col_index("p_value").unwrap();
        for row in &t.rows {
            if let Value::Float(p) = &row[p_idx] {
                assert!(
                    (*p - 1.0).abs() < 1e-10,
                    "no overlap should give p=1.0, got {p}"
                );
            }
        }
    }
}

#[test]
fn test_enrich_very_small_gene_set() {
    let genes = gene_list(&["A", "B", "C"]);
    let sets = gene_sets(vec![("tiny", vec!["A"])]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(20000)]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        let overlap_idx = t.col_index("overlap").unwrap();
        if let Value::Int(o) = &t.rows[0][overlap_idx] {
            assert_eq!(*o, 1);
        }
    }
}

#[test]
fn test_enrich_result_structure() {
    let genes = gene_list(&["A", "B"]);
    let sets = gene_sets(vec![("s1", vec!["A", "C"])]);
    let result = call_enrich_builtin("enrich", vec![genes, sets, Value::Int(100)]).unwrap();
    if let Value::Table(t) = result {
        // Check all expected columns exist
        assert_eq!(t.columns.len(), 5);
        assert!(t.col_index("term").is_some());
        assert!(t.col_index("overlap").is_some());
        assert!(t.col_index("p_value").is_some());
        assert!(t.col_index("fdr").is_some());
        assert!(t.col_index("genes").is_some());

        // Check types per row
        for row in &t.rows {
            assert!(matches!(row[0], Value::Str(_)), "term should be Str");
            assert!(matches!(row[1], Value::Int(_)), "overlap should be Int");
            assert!(matches!(row[2], Value::Float(_)), "p_value should be Float");
            assert!(matches!(row[3], Value::Float(_)), "fdr should be Float");
            assert!(matches!(row[4], Value::Str(_)), "genes should be Str");
        }
    } else {
        panic!("expected Table");
    }
}

// ── ora alias ───────────────────────────────────────────────────

#[test]
fn test_ora_alias() {
    let genes = gene_list(&["A"]);
    let sets = gene_sets(vec![("s1", vec!["A"])]);
    let result = call_enrich_builtin("ora", vec![genes, sets, Value::Int(100)]);
    assert!(result.is_ok());
}

// ── Type errors ─────────────────────────────────────────────────

#[test]
fn test_ora_wrong_genes_type() {
    let result = call_enrich_builtin(
        "ora",
        vec![
            Value::Int(42),
            gene_sets(vec![("s1", vec!["A"])]),
            Value::Int(100),
        ],
    );
    assert!(result.is_err(), "non-List genes should error");
}

#[test]
fn test_ora_wrong_gene_sets_type() {
    let result = call_enrich_builtin(
        "ora",
        vec![gene_list(&["A"]), Value::Int(42), Value::Int(100)],
    );
    assert!(result.is_err(), "non-Map gene sets should error");
}

#[test]
fn test_ora_wrong_bg_size_type() {
    let result = call_enrich_builtin(
        "ora",
        vec![
            gene_list(&["A"]),
            gene_sets(vec![("s1", vec!["A"])]),
            Value::Str("bad".into()),
        ],
    );
    assert!(result.is_err(), "non-Int background size should error");
}

#[test]
fn test_ora_genes_with_non_string_elements() {
    let genes = Value::List(vec![Value::Int(1), Value::Int(2)]);
    let sets = gene_sets(vec![("s1", vec!["A"])]);
    let result = call_enrich_builtin("ora", vec![genes, sets, Value::Int(100)]);
    assert!(result.is_err(), "non-Str gene names should error");
}

// ── GSEA ────────────────────────────────────────────────────────

#[test]
fn test_gsea_enriched_set() {
    // Create ranked list where top genes are all in set1
    let mut genes_scores = Vec::new();
    for i in 0..100 {
        genes_scores.push((format!("G{i}"), 100.0 - i as f64));
    }
    let table = ranked_table(
        genes_scores
            .iter()
            .map(|(g, s)| (g.as_str(), *s))
            .collect(),
    );
    let sets = gene_sets(vec![(
        "top_set",
        (0..10).map(|i| format!("G{i}")).collect::<Vec<_>>().iter().map(|s| s.as_str()).collect(),
    )]);
    let result = call_enrich_builtin("gsea", vec![table, sets]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        let es_idx = t.col_index("es").unwrap();
        if let Value::Float(es) = &t.rows[0][es_idx] {
            assert!(*es > 0.0, "ES={es} should be positive for top-ranked set");
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gsea_reversed_ranking() {
    // Top genes are NOT in the set (worst genes first in ranking)
    let mut genes_scores = Vec::new();
    for i in 0..100 {
        genes_scores.push((format!("G{i}"), 100.0 - i as f64));
    }
    let table = ranked_table(
        genes_scores
            .iter()
            .map(|(g, s)| (g.as_str(), *s))
            .collect(),
    );
    // Set contains genes at the BOTTOM of the ranking
    let bottom_genes: Vec<String> = (90..100).map(|i| format!("G{i}")).collect();
    let sets = gene_sets(vec![(
        "bottom_set",
        bottom_genes.iter().map(|s| s.as_str()).collect(),
    )]);
    let result = call_enrich_builtin("gsea", vec![table, sets]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        let es_idx = t.col_index("es").unwrap();
        if let Value::Float(es) = &t.rows[0][es_idx] {
            assert!(
                *es < 0.0,
                "ES={es} should be negative for bottom-ranked set"
            );
        }
    }
}

#[test]
fn test_gsea_empty_gene_set_no_enrichment() {
    // Gene set contains genes not present in the ranking at all
    let table = ranked_table(vec![("G1", 10.0), ("G2", 5.0), ("G3", 1.0)]);
    let sets = gene_sets(vec![("absent_set", vec!["X", "Y", "Z"])]);
    let result = call_enrich_builtin("gsea", vec![table, sets]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        let es_idx = t.col_index("es").unwrap();
        if let Value::Float(es) = &t.rows[0][es_idx] {
            // ES should be 0 when no genes from the set are in the ranking
            assert!(
                es.abs() < 1e-10,
                "ES should be 0 for absent gene set, got {es}"
            );
        }
    }
}

#[test]
fn test_gsea_result_structure() {
    let table = ranked_table(vec![("A", 10.0), ("B", 5.0), ("C", 1.0)]);
    let sets = gene_sets(vec![("s1", vec!["A"])]);
    let result = call_enrich_builtin("gsea", vec![table, sets]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.columns.len(), 6);
        assert!(t.col_index("term").is_some());
        assert!(t.col_index("es").is_some());
        assert!(t.col_index("nes").is_some());
        assert!(t.col_index("p_value").is_some());
        assert!(t.col_index("fdr").is_some());
        assert!(t.col_index("leading_edge").is_some());
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_gsea_wrong_table_type() {
    let result = call_enrich_builtin(
        "gsea",
        vec![Value::Int(42), gene_sets(vec![("s1", vec!["A"])])],
    );
    assert!(result.is_err());
}

#[test]
fn test_gsea_wrong_gene_sets_type() {
    let table = ranked_table(vec![("A", 1.0)]);
    let result = call_enrich_builtin("gsea", vec![table, Value::Int(42)]);
    assert!(result.is_err());
}

// ── Unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_enrich_builtin() {
    let r = call_enrich_builtin("nonexistent", vec![]);
    assert!(r.is_err());
}
