//! Tests for longread.rs and motif.rs builtins.
//! Uses #[path] trick to compile without lib.rs registration.

#[path = "../src/longread.rs"]
mod longread;

#[path = "../src/motif.rs"]
mod motif;

use bl_core::value::{Table, Value};

fn str_val(s: &str) -> Value {
    Value::Str(s.to_string())
}
fn int_val(n: i64) -> Value {
    Value::Int(n)
}
fn float_val(f: f64) -> Value {
    Value::Float(f)
}
fn list_val(v: Vec<Value>) -> Value {
    Value::List((v).into())
}

fn sample_fastq() -> &'static str {
    // 3 reads: lengths 10, 20, 30; quality all '5' (Phred 20)
    "@read1\nACGTACGTAC\n+\n5555555555\n\
     @read2\nACGTACGTACGTACGTACGT\n+\n55555555555555555555\n\
     @read3\nACGTACGTACGTACGTACGTACGTACGTAC\n+\n555555555555555555555555555555"
}

// ─── longread tests ───────────────────────────────────────────────────

#[test]
fn test_fastq_stats_basic() {
    let result =
        longread::call_longread_builtin("fastq_stats", vec![str_val(sample_fastq())]).unwrap();
    if let Value::Record(rec) = result {
        assert_eq!(rec["n_reads"], Value::Int(3));
        assert_eq!(rec["total_bases"], Value::Int(60));
        assert_eq!(rec["max_length"], Value::Int(30));
        assert_eq!(rec["min_length"], Value::Int(10));
        // mean_length = 60/3 = 20
        if let Value::Float(ml) = rec["mean_length"] {
            assert!((ml - 20.0).abs() < 0.01, "mean_length = {ml}");
        }
        // N50 for lengths [10, 20, 30]: total=60, half=30; sorted desc [30,20,10], cumsum: 30≥30 → N50=30
        assert_eq!(rec["n50"], Value::Int(30));
        // mean quality: ASCII '5' = 53, 53-33=20
        if let Value::Float(mq) = rec["mean_quality"] {
            assert!((mq - 20.0).abs() < 0.01, "mean_quality = {mq}");
        }
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_n50_known() {
    // lengths [10, 8, 6, 5, 3] total=32, half=16
    // sorted desc: [10,8,6,5,3], cumsum: 10, 18 → N50=8
    let result = longread::call_longread_builtin(
        "n50",
        vec![list_val(vec![
            int_val(10),
            int_val(8),
            int_val(6),
            int_val(5),
            int_val(3),
        ])],
    )
    .unwrap();
    assert_eq!(result, Value::Int(8));
}

#[test]
fn test_n50_single() {
    let result = longread::call_longread_builtin("n50", vec![list_val(vec![int_val(42)])]).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_read_length_hist_bins() {
    let lengths = list_val(vec![int_val(100), int_val(200), int_val(300), int_val(400)]);
    let result =
        longread::call_longread_builtin("read_length_hist", vec![lengths, int_val(2)]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.columns, vec!["bin_start", "bin_end", "count"]);
        // bin 0: 100–250, count 2; bin 1: 250–400, count 2
        assert_eq!(t.rows[0][2], Value::Int(2));
        assert_eq!(t.rows[1][2], Value::Int(2));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_quality_filter_drops_short() {
    let fq = "@r1\nACGT\n+\n5555\n@r2\nACGTACGTACGT\n+\n555555555555\n";
    // min_length=10 should drop r1 (len=4), keep r2 (len=12)
    let result = longread::call_longread_builtin(
        "quality_filter",
        vec![str_val(fq), int_val(10), float_val(0.0)],
    )
    .unwrap();
    if let Value::Str(out) = result {
        assert!(!out.contains("@r1"), "r1 should be filtered");
        assert!(out.contains("@r2"), "r2 should pass");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_gc_per_read() {
    // read1: AAAA (GC=0), read2: GCGC (GC=1.0)
    let fq = "@r1\nAAAA\n+\n5555\n@r2\nGCGC\n+\n5555\n";
    let result = longread::call_longread_builtin("gc_per_read", vec![str_val(fq)]).unwrap();
    if let Value::List(v) = result {
        assert_eq!(v.len(), 2);
        if let Value::Float(gc1) = v[0] {
            assert!((gc1 - 0.0).abs() < 0.01);
        }
        if let Value::Float(gc2) = v[1] {
            assert!((gc2 - 1.0).abs() < 0.01);
        }
    } else {
        panic!("expected List");
    }
}

// ─── motif tests ──────────────────────────────────────────────────────

#[test]
fn test_iupac_scan_exact() {
    // Pattern "ATG" in "CCATGCC" at position 2
    let result =
        motif::call_motif_builtin("iupac_scan", vec![str_val("CCATGCC"), str_val("ATG")]).unwrap();
    assert_eq!(result, Value::List((vec![Value::Int(2)]).into()));
}

#[test]
fn test_iupac_scan_degenerate() {
    // R = A or G; scan "ATCAGC" for "R" should find positions 0 (A) and 3 (A) and... let's check
    // seq: A T C A G C
    // pos: 0 1 2 3 4 5
    // R matches A or G: positions 0, 3, 4
    let result =
        motif::call_motif_builtin("iupac_scan", vec![str_val("ATCAGC"), str_val("R")]).unwrap();
    assert_eq!(
        result,
        Value::List((vec![Value::Int(0), Value::Int(3), Value::Int(4)]).into())
    );
}

#[test]
fn test_pwm_from_seqs_basic() {
    // 2 seqs of length 4, both "ACGT"
    let seqs = list_val(vec![str_val("ACGT"), str_val("ACGT")]);
    let result = motif::call_motif_builtin("pwm_from_seqs", vec![seqs]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.rows.len(), 4);
        assert_eq!(t.columns[0], "pos");
        // At pos 0, A should have the highest weight (all seqs have A there)
        let row0 = &t.rows[0];
        let wa = if let Value::Float(f) = row0[1] {
            f
        } else {
            0.0
        };
        let wc = if let Value::Float(f) = row0[2] {
            f
        } else {
            0.0
        };
        assert!(
            wa > wc,
            "A weight {wa} should exceed C weight {wc} at pos 0"
        );
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_pwm_scan_finds_match() {
    // Build a trivial PWM from two identical "ACG" seqs, then scan for it in "XACGX"
    let seqs = list_val(vec![str_val("ACG"), str_val("ACG")]);
    let pwm = motif::call_motif_builtin("pwm_from_seqs", vec![seqs]).unwrap();

    let result =
        motif::call_motif_builtin("pwm_scan", vec![str_val("XACGX"), pwm, float_val(0.5)]).unwrap();
    if let Value::Table(t) = result {
        assert!(!t.rows.is_empty(), "expected at least one match");
        // First match should be at start=1 (0-based, after X)
        // X is not a standard nucleotide so position 0 won't score well
        let starts: Vec<i64> = t
            .rows
            .iter()
            .map(|r| if let Value::Int(s) = r[0] { s } else { -1 })
            .collect();
        assert!(starts.contains(&1), "ACG should be found at position 1");
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_motif_consensus_acgt() {
    // PWM built from "ACGT" × 4 should yield consensus "ACGT"
    let seqs = list_val(vec![
        str_val("ACGT"),
        str_val("ACGT"),
        str_val("ACGT"),
        str_val("ACGT"),
    ]);
    let pwm = motif::call_motif_builtin("pwm_from_seqs", vec![seqs]).unwrap();
    let cons = motif::call_motif_builtin("motif_consensus", vec![pwm]).unwrap();
    assert_eq!(cons, Value::Str("ACGT".to_string()));
}

#[test]
fn test_gc_bias_window() {
    // AAAA GCGC → first half GC=0, second half GC=1
    let result =
        motif::call_motif_builtin("gc_bias", vec![str_val("AAAAGCGC"), int_val(4)]).unwrap();
    if let Value::Table(t) = result {
        assert!(!t.rows.is_empty());
        // First window AAAA: gc_fraction = 0
        if let Value::Float(gc) = t.rows[0][2] {
            assert!((gc - 0.0).abs() < 0.01, "first window gc = {gc}");
        }
    } else {
        panic!("expected Table");
    }
}
