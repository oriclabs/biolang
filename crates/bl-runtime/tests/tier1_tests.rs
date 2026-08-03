use bl_core::value::{Table, Value};
use bl_runtime::chipseq::call_chipseq_builtin;
use bl_runtime::microbiome::call_microbiome_builtin;
use bl_runtime::phylo::call_phylo_builtin;
use bl_runtime::rnaseq::call_rnaseq_builtin;
use bl_runtime::variants::call_variants_builtin;

// ── Helpers ──────────────────────────────────────────────────────────

fn int(n: i64) -> Value {
    Value::Int(n)
}
fn float(f: f64) -> Value {
    Value::Float(f)
}
fn str_(s: &str) -> Value {
    Value::Str(s.to_string())
}
fn list(v: Vec<Value>) -> Value {
    Value::List((v).into())
}

fn table(cols: &[&str], rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

fn call_ok(module: &str, name: &str, args: Vec<Value>) -> Value {
    match module {
        "variants" => call_variants_builtin(name, args).expect(name),
        "rnaseq" => call_rnaseq_builtin(name, args).expect(name),
        "phylo" => call_phylo_builtin(name, args).expect(name),
        "chipseq" => call_chipseq_builtin(name, args).expect(name),
        "microbiome" => call_microbiome_builtin(name, args).expect(name),
        _ => panic!("unknown module {module}"),
    }
}

// ── variants ─────────────────────────────────────────────────────────

#[test]
fn test_vcf_parse_basic() {
    let vcf = "##fileformat=VCFv4.2\nchr1\t100\t.\tA\tG\t50.0\tPASS\tAF=0.5\n";
    let result = call_ok("variants", "vcf_parse", vec![str_(vcf)]);
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 1);
            assert_eq!(t.columns[0], "chrom");
            assert_eq!(t.rows[0][0], str_("chr1"));
            assert_eq!(t.rows[0][1], int(100));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn test_titv_ratio() {
    let t = table(
        &[
            "chrom", "pos", "id", "ref_", "alt", "qual", "filter", "info",
        ],
        vec![
            vec![
                str_("chr1"),
                int(100),
                str_("."),
                str_("A"),
                str_("G"),
                float(50.0),
                str_("PASS"),
                str_("."),
            ],
            vec![
                str_("chr1"),
                int(200),
                str_("."),
                str_("A"),
                str_("C"),
                float(40.0),
                str_("PASS"),
                str_("."),
            ],
            vec![
                str_("chr1"),
                int(300),
                str_("."),
                str_("C"),
                str_("T"),
                float(45.0),
                str_("PASS"),
                str_("."),
            ],
        ],
    );
    let ratio = call_ok("variants", "titv_ratio", vec![t]);
    match ratio {
        Value::Float(f) => assert!((f - 2.0).abs() < 1e-9, "expected Ti/Tv=2, got {f}"),
        _ => panic!("expected Float"),
    }
}

#[test]
fn test_variant_summary() {
    let t = table(
        &[
            "chrom", "pos", "id", "ref_", "alt", "qual", "filter", "info",
        ],
        vec![
            vec![
                str_("chr1"),
                int(1),
                str_("."),
                str_("A"),
                str_("G"),
                float(50.0),
                str_("PASS"),
                str_("."),
            ],
            vec![
                str_("chr1"),
                int(2),
                str_("."),
                str_("A"),
                str_("AT"),
                float(50.0),
                str_("PASS"),
                str_("."),
            ],
        ],
    );
    let result = call_ok("variants", "variant_summary", vec![t]);
    match result {
        Value::Table(t) => {
            let snp_row = t.rows.iter().find(|r| r[0] == str_("SNP"));
            assert!(snp_row.is_some(), "expected SNP row");
            assert_eq!(snp_row.unwrap()[1], int(1));
        }
        _ => panic!("expected Table"),
    }
}

// ── rnaseq ───────────────────────────────────────────────────────────

#[test]
fn test_parse_salmon() {
    let text = "Name\tLength\tEffectiveLength\tTPM\tNumReads\ngene1\t1000\t900.0\t12.5\t150.0\n";
    let result = call_ok("rnaseq", "parse_salmon", vec![str_(text)]);
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 1);
            assert_eq!(t.rows[0][0], str_("gene1"));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn test_size_factors() {
    // 2 genes × 2 samples; each sample gets factor 1.0 with symmetric matrix
    let t = table(
        &["sample_a", "sample_b"],
        vec![vec![float(4.0), float(4.0)], vec![float(16.0), float(16.0)]],
    );
    let result = call_ok("rnaseq", "size_factors", vec![t]);
    match result {
        Value::List(v) => {
            assert_eq!(v.len(), 2);
            for val in v.iter() {
                match val {
                    Value::Float(f) => assert!((*f - 1.0).abs() < 1e-9, "factor={f}"),
                    _ => panic!("expected Float"),
                }
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_filter_low_counts() {
    let t = table(
        &["s1", "s2"],
        vec![
            vec![int(5), int(5)],   // both below min_count=10
            vec![int(20), int(30)], // both above
        ],
    );
    let result = call_ok("rnaseq", "filter_low_counts", vec![t, int(10), int(1)]);
    match result {
        Value::Table(t) => assert_eq!(t.rows.len(), 1, "only 1 row should pass"),
        _ => panic!("expected Table"),
    }
}

// ── phylo ────────────────────────────────────────────────────────────

#[test]
fn test_nw_parse() {
    let nw = "(A:0.1,B:0.2,(C:0.3,D:0.4):0.5);";
    let result = call_ok("phylo", "nw_parse", vec![str_(nw)]);
    match result {
        Value::Table(t) => {
            assert!(t.rows.len() >= 4, "expected at least 4 nodes");
            assert!(t.columns.contains(&"label".to_string()));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn test_tree_leaves() {
    let nw = "(A:0.1,B:0.2);";
    let tree = call_ok("phylo", "nw_parse", vec![str_(nw)]);
    let leaves = call_ok("phylo", "tree_leaves", vec![tree]);
    match leaves {
        Value::List(l) => {
            assert_eq!(l.len(), 2);
            let labels: Vec<&str> = l
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.as_str(),
                    _ => "",
                })
                .collect();
            assert!(labels.contains(&"A") && labels.contains(&"B"));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_patristic_distance() {
    // Simple star: (A:1.0,B:2.0); — distance A→B = 1+2 = 3
    let nw = "(A:1.0,B:2.0);";
    let tree = call_ok("phylo", "nw_parse", vec![str_(nw)]);
    let dist = call_ok(
        "phylo",
        "patristic_distance",
        vec![tree, str_("A"), str_("B")],
    );
    match dist {
        Value::Float(f) => assert!((f - 3.0).abs() < 1e-9, "expected 3.0, got {f}"),
        _ => panic!("expected Float"),
    }
}

// ── chipseq ──────────────────────────────────────────────────────────

#[test]
fn test_merge_peaks() {
    let peaks = table(
        &["chrom", "start", "end"],
        vec![
            vec![str_("chr1"), int(100), int(200)],
            vec![str_("chr1"), int(150), int(300)], // overlaps previous
            vec![str_("chr1"), int(500), int(600)], // separate
        ],
    );
    let result = call_ok("chipseq", "merge_peaks", vec![peaks]);
    match result {
        Value::Table(t) => assert_eq!(t.rows.len(), 2, "expected 2 merged peaks"),
        _ => panic!("expected Table"),
    }
}

#[test]
fn test_frip_score() {
    let peaks = table(&["chrom", "start", "end"], vec![]);
    let result = call_ok("chipseq", "frip_score", vec![peaks, int(1000), int(250)]);
    match result {
        Value::Float(f) => assert!((f - 0.25).abs() < 1e-9, "expected 0.25"),
        _ => panic!("expected Float"),
    }
}

#[test]
fn test_tss_enrichment() {
    // Signal: 21 positions, flanks=1.0, center=10 with value 100.0
    // Enrichment = center_mean / flank_mean >> 1
    let mut signal = vec![float(1.0); 21];
    signal[10] = float(100.0);
    let result = call_ok("chipseq", "tss_enrichment", vec![list(signal), int(10)]);
    match result {
        Value::Float(f) => assert!(f > 1.0, "TSS enrichment should be > 1, got {f}"),
        _ => panic!("expected Float"),
    }
}

// ── microbiome ───────────────────────────────────────────────────────

#[test]
fn test_alpha_diversity_shannon() {
    // Equal counts → max Shannon diversity
    let counts = list(vec![int(10), int(10), int(10), int(10)]);
    let result = call_ok(
        "microbiome",
        "alpha_diversity",
        vec![counts, str_("shannon")],
    );
    match result {
        Value::Float(f) => assert!(f > 1.3, "Shannon H > ln(4)≈1.386 for equal probs, got {f}"),
        _ => panic!("expected Float"),
    }
}

#[test]
fn test_relative_abundance() {
    let counts = list(vec![int(25), int(75)]);
    let result = call_ok("microbiome", "relative_abundance", vec![counts]);
    match result {
        Value::List(v) => {
            assert_eq!(v.len(), 2);
            match (&v[0], &v[1]) {
                (Value::Float(a), Value::Float(b)) => {
                    assert!((a - 0.25).abs() < 1e-9);
                    assert!((b - 0.75).abs() < 1e-9);
                }
                _ => panic!("expected floats"),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_rarefaction() {
    let counts = list(vec![int(50), int(30), int(20)]);
    let result = call_ok("microbiome", "rarefaction", vec![counts, int(50)]);
    match result {
        Value::List(v) => {
            let total: i64 = v
                .iter()
                .map(|x| match x {
                    Value::Int(n) => *n,
                    _ => 0,
                })
                .sum();
            assert_eq!(total, 50, "rarefied total should equal depth");
        }
        _ => panic!("expected List"),
    }
}
