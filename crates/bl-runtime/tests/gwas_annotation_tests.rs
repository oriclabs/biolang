#[path = "../src/gwas.rs"]
mod gwas;
#[path = "../src/annotation.rs"]
mod annotation;

use bl_core::value::{Table, Value};

fn str(s: &str) -> Value { Value::Str(s.to_string()) }
fn int(n: i64) -> Value { Value::Int(n) }
fn float(f: f64) -> Value { Value::Float(f) }

fn table(cols: &[&str], rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(cols.iter().map(|s| s.to_string()).collect(), rows))
}

fn call_gwas(name: &str, args: Vec<Value>) -> Value {
    gwas::call_gwas_builtin(name, args).expect(name)
}

fn call_ann(name: &str, args: Vec<Value>) -> Value {
    annotation::call_annotation_builtin(name, args).expect(name)
}

fn get_col(t: &Table, col: &str) -> Vec<Value> {
    let ci = t.columns.iter().position(|c| c == col).unwrap();
    t.rows.iter().map(|r| r[ci].clone()).collect()
}

fn as_table(v: Value) -> Table {
    match v { Value::Table(t) => t, _ => panic!("expected Table") }
}

fn as_float(v: &Value) -> f64 {
    match v { Value::Float(f) => *f, Value::Int(n) => *n as f64, _ => panic!("not float") }
}

// ── GWAS tests ────────────────────────────────────────────────────────

#[test]
fn parse_sumstats_tab_delimited() {
    let text = "CHR\tBP\tSNP\tP\tBETA\n1\t1000\trs1\t0.001\t0.1\n1\t2000\trs2\t0.5\t-0.05";
    let t = as_table(call_gwas("parse_sumstats", vec![str(text)]));
    assert_eq!(t.rows.len(), 2);
    assert!(t.columns.contains(&"chrom".to_string()));
    assert!(t.columns.contains(&"pval".to_string()));
    assert!(t.columns.contains(&"snp".to_string()));
    let pvals = get_col(&t, "pval");
    assert!((as_float(&pvals[0]) - 0.001).abs() < 1e-10);
}

#[test]
fn parse_sumstats_detects_columns() {
    let text = "CHROM\tPOS\tRSID\tP_VALUE\n1\t100\trs99\t1e-10";
    let t = as_table(call_gwas("parse_sumstats", vec![str(text)]));
    assert!(t.columns.contains(&"chrom".to_string()));
    assert!(t.columns.contains(&"pos".to_string()));
    assert!(t.columns.contains(&"snp".to_string()));
    assert!(t.columns.contains(&"pval".to_string()));
}

#[test]
fn manhattan_data_cumulative_positions() {
    let rows = vec![
        vec![str("1"), int(1000), float(0.01)],
        vec![str("1"), int(2000), float(0.001)],
        vec![str("2"), int(500),  float(0.05)],
    ];
    let sumstats = table(&["chrom","pos","pval"], rows);
    let t = as_table(call_gwas("manhattan_data", vec![sumstats]));
    let cum: Vec<i64> = get_col(&t, "cumulative_pos").iter().map(|v| match v { Value::Int(n) => *n, _ => -1 }).collect();
    // chrom 1: offset 0, so pos 1000→1000, pos 2000→2000
    // chrom 2: offset > 2000
    assert!(cum[2] > cum[1], "chrom 2 cumpos should exceed chrom 1 max");
    let nlp = get_col(&t, "neg_log10_p");
    assert!(as_float(&nlp[1]) > as_float(&nlp[0]), "lower p → higher -log10p");
}

#[test]
fn qq_data_length_and_order() {
    let pvals = vec![float(0.1), float(0.5), float(0.01), float(0.9)];
    let t = as_table(call_gwas("qq_data", vec![Value::List((pvals).into())]));
    assert_eq!(t.rows.len(), 4);
    assert!(t.columns.contains(&"expected".to_string()));
    assert!(t.columns.contains(&"observed".to_string()));
    // Expected values should be in increasing order (sorted by p ascending)
    let obs: Vec<f64> = get_col(&t, "observed").iter().map(|v| as_float(v)).collect();
    // observed -log10p: smallest p has largest observed
    assert!(obs[0] > obs[obs.len()-1]);
}

#[test]
fn clump_removes_nearby_snps() {
    let rows = vec![
        vec![str("1"), int(1000), float(1e-10)],  // index SNP
        vec![str("1"), int(1100), float(1e-8)],   // within 250kb — should be excluded
        vec![str("1"), int(5_000_000), float(1e-9)], // far away — should be kept
    ];
    let sumstats = table(&["chrom","pos","pval"], rows);
    let t = as_table(call_gwas("clump", vec![sumstats, float(5e-8), int(250)]));
    assert_eq!(t.rows.len(), 2, "second SNP within 250kb should be clumped out");
}

#[test]
fn top_loci_filter() {
    let rows = vec![
        vec![str("1"), int(100), float(1e-10)],
        vec![str("1"), int(200), float(0.01)],
        vec![str("2"), int(300), float(5e-8)],
    ];
    let sumstats = table(&["chrom","pos","pval"], rows);
    let t = as_table(call_gwas("top_loci", vec![sumstats, float(5e-8)]));
    assert_eq!(t.rows.len(), 2); // 1e-10 and 5e-8 pass threshold
}

#[test]
fn lambda_gc_near_one_for_uniform() {
    // Under the null (uniform p-values), lambda_gc ≈ 1.0
    let pvals: Vec<Value> = (1..=100).map(|i| float(i as f64 / 101.0)).collect();
    let result = call_gwas("lambda_gc", vec![Value::List((pvals).into())]);
    let lam = as_float(&result);
    assert!((lam - 1.0).abs() < 0.15, "lambda_gc for uniform pvals should be ~1, got {lam}");
}

// ── Annotation tests ──────────────────────────────────────────────────

#[test]
fn parse_gtf_basic() {
    let gtf = r#"##gff-version 2
chr1	HAVANA	gene	11869	14409	.	+	.	gene_id "ENSG00000223972"; gene_name "DDX11L1"; gene_type "transcribed_unprocessed_pseudogene";
chr1	HAVANA	transcript	11869	14409	.	+	.	gene_id "ENSG00000223972"; transcript_id "ENST00000456328"; gene_name "DDX11L1";
"#;
    let t = as_table(call_ann("parse_gtf", vec![str(gtf)]));
    assert_eq!(t.rows.len(), 2);
    assert!(t.columns.contains(&"gene_id".to_string()));
    assert!(t.columns.contains(&"gene_name".to_string()));
    let gene_ids = get_col(&t, "gene_id");
    assert_eq!(match &gene_ids[0] { Value::Str(s) => s.as_str(), _ => "" }, "ENSG00000223972");
    let gene_names = get_col(&t, "gene_name");
    assert_eq!(match &gene_names[0] { Value::Str(s) => s.as_str(), _ => "" }, "DDX11L1");
}

#[test]
fn gene_bodies_collapses_transcripts() {
    // Two transcripts of same gene at different coordinates
    let rows = vec![
        vec![str("ENSG1"), str("ENST1"), str("chr1"), int(100), int(500), str("+"), str("GeneA"), str("")],
        vec![str("ENSG1"), str("ENST2"), str("chr1"), int(200), int(800), str("+"), str("GeneA"), str("")],
    ];
    let gtf = Value::Table(Table::new(
        vec!["gene_id","transcript_id","chrom","start","end","strand","gene_name","gene_type"].iter().map(|s| s.to_string()).collect(),
        rows,
    ));
    let t = as_table(call_ann("gene_bodies", vec![gtf]));
    assert_eq!(t.rows.len(), 1); // one gene
    let starts = get_col(&t, "start");
    let ends   = get_col(&t, "end");
    assert_eq!(match &starts[0] { Value::Int(n) => *n, _ => -1 }, 100); // min start
    assert_eq!(match &ends[0]   { Value::Int(n) => *n, _ => -1 }, 800); // max end
}

#[test]
fn interval_overlap_finds_pairs() {
    let query = table(&["chrom","start","end","name"], vec![
        vec![str("chr1"), int(100), int(300), str("peak1")],
        vec![str("chr1"), int(500), int(700), str("peak2")],
    ]);
    let subject = table(&["chrom","start","end","gene"], vec![
        vec![str("chr1"), int(200), int(400), str("GeneA")],  // overlaps peak1
        vec![str("chr1"), int(600), int(800), str("GeneB")],  // overlaps peak2
        vec![str("chr2"), int(100), int(300), str("GeneC")],  // different chrom
    ]);
    let t = as_table(call_ann("interval_overlap", vec![query, subject]));
    assert_eq!(t.rows.len(), 2); // peak1∩GeneA + peak2∩GeneB
    // Check columns include both query and subject
    assert!(t.columns.contains(&"name".to_string()));
    assert!(t.columns.contains(&"gene_subject".to_string()));
}

#[test]
fn gene_id_map_deduplicates() {
    let rows = vec![
        vec![str("ENSG1"), str(""), str("chr1"), int(1), int(100), str("+"), str("GeneA"), str("")],
        vec![str("ENSG1"), str(""), str("chr1"), int(1), int(100), str("+"), str("GeneA"), str("")], // duplicate
        vec![str("ENSG2"), str(""), str("chr1"), int(200), int(300), str("+"), str("GeneB"), str("")],
    ];
    let gtf = Value::Table(Table::new(
        vec!["gene_id","transcript_id","chrom","start","end","strand","gene_name","gene_type"].iter().map(|s| s.to_string()).collect(),
        rows,
    ));
    let t = as_table(call_ann("gene_id_map", vec![gtf]));
    assert_eq!(t.rows.len(), 2); // ENSG1 and ENSG2, deduplicated
}
