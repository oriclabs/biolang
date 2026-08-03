#[path = "../src/crispr.rs"]
mod crispr;
#[path = "../src/immune.rs"]
mod immune;

use bl_core::value::{Table, Value};

// ── crispr tests ──────────────────────────────────────────────────────

#[test]
fn test_guide_counts_parses_tsv() {
    let tsv = "sgRNA\tgene\tctrl1\tctrl2\ttrt1\ttrt2\n\
               guide_A\tGENE1\t100\t110\t10\t12\n\
               guide_B\tGENE1\t90\t95\t8\t9\n\
               guide_C\tGENE2\t80\t85\t200\t210\n";
    let result =
        crispr::call_crispr_builtin("guide_counts", vec![Value::Str(tsv.to_string())]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    assert_eq!(table.rows.len(), 3);
    assert!(table.columns.contains(&"guide".to_string()));
    assert!(table.columns.contains(&"gene".to_string()));
    assert!(table.columns.contains(&"ctrl1".to_string()));
    // guide name of first row
    assert_eq!(table.rows[0][0], Value::Str("guide_A".to_string()));
    // count
    assert_eq!(table.rows[0][2], Value::Int(100));
}

#[test]
fn test_guide_counts_skips_comments() {
    let tsv = "# comment line\nsgRNA\tgene\ts1\nguide_X\tGENE\t50\n";
    let result =
        crispr::call_crispr_builtin("guide_counts", vec![Value::Str(tsv.to_string())]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0][0], Value::Str("guide_X".to_string()));
}

#[test]
fn test_lfc_guides_computes_correctly() {
    // Build a count table manually: 3 rows, cols: guide, gene, ctrl1, ctrl2, trt1, trt2
    let col_names = vec![
        "guide".to_string(),
        "gene".to_string(),
        "ctrl1".to_string(),
        "ctrl2".to_string(),
        "trt1".to_string(),
        "trt2".to_string(),
    ];
    let rows = vec![
        vec![
            Value::Str("g1".to_string()),
            Value::Str("GENE1".to_string()),
            Value::Int(100),
            Value::Int(100),
            Value::Int(10),
            Value::Int(10),
        ],
        vec![
            Value::Str("g2".to_string()),
            Value::Str("GENE1".to_string()),
            Value::Int(50),
            Value::Int(50),
            Value::Int(200),
            Value::Int(200),
        ],
    ];
    let table = Value::Table(Table::new(col_names, rows));
    // ctrl_cols = [2, 3], trt_cols = [4, 5]
    let result = crispr::call_crispr_builtin(
        "lfc_guides",
        vec![
            table,
            Value::List((vec![Value::Int(2), Value::Int(3)]).into()),
            Value::List((vec![Value::Int(4), Value::Int(5)]).into()),
        ],
    )
    .unwrap();
    let t = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    assert_eq!(t.rows.len(), 2);
    // g1: ctrl=100, trt=10 → lfc = log2(11/101) ≈ -3.2
    let lfc_col = t.columns.iter().position(|c| c == "lfc").unwrap();
    let lfc_g1 = match &t.rows[0][lfc_col] {
        Value::Float(f) => *f,
        _ => panic!("lfc must be Float"),
    };
    assert!(
        lfc_g1 < 0.0,
        "guide depleted in treatment should have negative lfc"
    );
    let lfc_g2 = match &t.rows[1][lfc_col] {
        Value::Float(f) => *f,
        _ => panic!("lfc must be Float"),
    };
    assert!(
        lfc_g2 > 0.0,
        "guide enriched in treatment should have positive lfc"
    );
}

#[test]
fn test_mageck_score_gene_aggregation() {
    let col_names = vec![
        "guide".to_string(),
        "gene".to_string(),
        "ctrl1".to_string(),
        "trt1".to_string(),
    ];
    let rows = vec![
        vec![
            Value::Str("g1".to_string()),
            Value::Str("GENE1".to_string()),
            Value::Int(100),
            Value::Int(5),
        ],
        vec![
            Value::Str("g2".to_string()),
            Value::Str("GENE1".to_string()),
            Value::Int(100),
            Value::Int(6),
        ],
        vec![
            Value::Str("g3".to_string()),
            Value::Str("GENE2".to_string()),
            Value::Int(50),
            Value::Int(200),
        ],
        vec![
            Value::Str("g4".to_string()),
            Value::Str("GENE2".to_string()),
            Value::Int(50),
            Value::Int(180),
        ],
    ];
    let table = Value::Table(Table::new(col_names, rows));
    let result = crispr::call_crispr_builtin(
        "mageck_score",
        vec![
            table,
            Value::List((vec![Value::Int(2)]).into()),
            Value::List((vec![Value::Int(3)]).into()),
        ],
    )
    .unwrap();
    let t = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    // GENE1 should rank lower (more essential = depleted) than GENE2
    assert_eq!(t.rows.len(), 2);
    let gene_col = t.columns.iter().position(|c| c == "gene").unwrap();
    assert_eq!(t.rows[0][gene_col], Value::Str("GENE1".to_string()));
}

#[test]
fn test_guide_gc() {
    // ATGCATGC → 4 GC out of 8 = 0.5
    let result =
        crispr::call_crispr_builtin("guide_gc", vec![Value::Str("ATGCATGC".to_string())]).unwrap();
    match result {
        Value::Float(f) => assert!((f - 0.5).abs() < 1e-9),
        _ => panic!("expected Float"),
    }
    // all GC
    let r2 =
        crispr::call_crispr_builtin("guide_gc", vec![Value::Str("GCGCGC".to_string())]).unwrap();
    match r2 {
        Value::Float(f) => assert!((f - 1.0).abs() < 1e-9),
        _ => panic!("expected Float"),
    }
}

#[test]
fn test_crispr_qc_metrics() {
    let col_names = vec![
        "guide".to_string(),
        "gene".to_string(),
        "s1".to_string(),
        "s2".to_string(),
    ];
    let rows = vec![
        vec![
            Value::Str("g1".to_string()),
            Value::Str("A".to_string()),
            Value::Int(100),
            Value::Int(100),
        ],
        vec![
            Value::Str("g2".to_string()),
            Value::Str("A".to_string()),
            Value::Int(50),
            Value::Int(50),
        ],
        vec![
            Value::Str("g3".to_string()),
            Value::Str("B".to_string()),
            Value::Int(0),
            Value::Int(0),
        ],
    ];
    let result =
        crispr::call_crispr_builtin("crispr_qc", vec![Value::Table(Table::new(col_names, rows))])
            .unwrap();
    let rec = match result {
        Value::Record(r) => r,
        _ => panic!("expected Record"),
    };
    assert_eq!(rec["n_guides"], Value::Int(3));
    assert_eq!(rec["n_genes"], Value::Int(2));
    assert_eq!(rec["zero_count_guides"], Value::Int(1));
    match &rec["gini_coefficient"] {
        Value::Float(f) => assert!(*f >= 0.0 && *f <= 1.0, "Gini must be [0,1]"),
        _ => panic!("expected Float"),
    }
}

// ── immune tests ──────────────────────────────────────────────────────

#[test]
fn test_parse_vdj_csv() {
    let csv = "barcode,raw_clonotype_id,v_gene,j_gene,cdr3,umis\n\
               AAACCC,clonotype1,TRAV1-2,TRAJ33,CAVSLDSNYQLIW,5\n\
               AAAGGG,clonotype1,TRAV1-2,TRAJ33,CAVSLDSNYQLIW,8\n\
               AAATTT,clonotype2,TRAV12-2,TRAJ6,CAVNLDSNYQLIW,3\n";
    let result =
        immune::call_immune_builtin("parse_vdj", vec![Value::Str(csv.to_string())]).unwrap();
    let t = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    assert_eq!(t.rows.len(), 3);
    assert!(t.columns.contains(&"cdr3".to_string()));
    assert!(t.columns.contains(&"v_gene".to_string()));
}

#[test]
fn test_clonotype_diversity_values() {
    // Two clones: one dominant, one rare
    let col_names = vec!["cdr3".to_string()];
    let mut rows = vec![];
    for _ in 0..9 {
        rows.push(vec![Value::Str("CLONE_A".to_string())]);
    }
    rows.push(vec![Value::Str("CLONE_B".to_string())]);

    let result = immune::call_immune_builtin(
        "clonotype_diversity",
        vec![Value::Table(Table::new(col_names, rows))],
    )
    .unwrap();
    let rec = match result {
        Value::Record(r) => r,
        _ => panic!("expected Record"),
    };
    assert_eq!(rec["richness"], Value::Int(2));
    assert_eq!(rec["total_cells"], Value::Int(10));
    // Shannon should be positive
    match &rec["shannon"] {
        Value::Float(f) => assert!(*f > 0.0),
        _ => panic!("expected Float"),
    }
    // Chao1 with f1=1, f2=0 → 2 + 1*(1-1)/2 = 2
    match &rec["chao1"] {
        Value::Float(f) => assert!((*f - 2.0).abs() < 1e-9),
        _ => panic!("expected Float for chao1"),
    }
}

#[test]
fn test_clonal_expansion_threshold() {
    let col_names = vec!["cdr3".to_string()];
    let mut rows = vec![];
    // 50 cells from one clone (50%), 50 from 50 singletons
    for _ in 0..50 {
        rows.push(vec![Value::Str("BIG_CLONE".to_string())]);
    }
    for i in 0..50 {
        rows.push(vec![Value::Str(format!("singleton_{i}"))]);
    }
    let result = immune::call_immune_builtin(
        "clonal_expansion",
        vec![
            Value::Table(Table::new(col_names, rows)),
            Value::Float(0.01), // threshold 1%
        ],
    )
    .unwrap();
    let rec = match result {
        Value::Record(r) => r,
        _ => panic!("expected Record"),
    };
    // Only BIG_CLONE > 1%
    assert_eq!(rec["n_expanded"], Value::Int(1));
    match &rec["top_clone_fraction"] {
        Value::Float(f) => assert!((*f - 0.5).abs() < 1e-9),
        _ => panic!("expected Float"),
    }
}

#[test]
fn test_vj_usage_sorted() {
    let col_names = vec!["v_gene".to_string(), "cdr3".to_string()];
    let rows = vec![
        vec![
            Value::Str("TRAV1".to_string()),
            Value::Str("SEQ1".to_string()),
        ],
        vec![
            Value::Str("TRAV1".to_string()),
            Value::Str("SEQ2".to_string()),
        ],
        vec![
            Value::Str("TRAV1".to_string()),
            Value::Str("SEQ3".to_string()),
        ],
        vec![
            Value::Str("TRAV2".to_string()),
            Value::Str("SEQ4".to_string()),
        ],
    ];
    let result =
        immune::call_immune_builtin("vj_usage", vec![Value::Table(Table::new(col_names, rows))])
            .unwrap();
    let t = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    // TRAV1 should appear first (3 counts)
    assert_eq!(t.rows[0][0], Value::Str("TRAV1".to_string()));
    assert_eq!(t.rows[0][1], Value::Int(3));
    // fraction of TRAV1 = 3/4
    match &t.rows[0][2] {
        Value::Float(f) => assert!((*f - 0.75).abs() < 1e-9),
        _ => panic!("expected Float"),
    }
}
