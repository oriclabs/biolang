// Both modules are `pub` in lib.rs, and the two entry points this file uses are
// public, so it links against the library like every other integration test.
//
// It used to pull both sources in a second time with `#[path]`, compiling them
// as modules of the test binary. In that copy `crate::` means the test crate,
// so `atac.rs`'s reference to `crate::singlecell` had nothing to resolve
// against and the whole suite failed to build -- eleven tests that had stopped
// running while the library itself compiled clean.
use bl_runtime::atac;
use bl_runtime::drug;

use bl_core::value::{Table, Value};

fn make_list_int(vals: &[i64]) -> Value {
    Value::List(
        vals.iter()
            .map(|&n| Value::Int(n))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn make_list_float(vals: &[f64]) -> Value {
    Value::List(
        vals.iter()
            .map(|&f| Value::Float(f))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn get_float(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => panic!("expected Float, got {:?}", v),
    }
}

fn get_int(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => panic!("expected Int"),
    }
}

fn get_record_float(v: &Value, key: &str) -> f64 {
    match v {
        Value::Record(m) => get_float(m.get(key).expect(key)),
        _ => panic!("expected Record"),
    }
}

fn get_record_int(v: &Value, key: &str) -> i64 {
    match v {
        Value::Record(m) => get_int(m.get(key).expect(key)),
        _ => panic!("expected Record"),
    }
}

// ── atac tests ───────────────────────────────────────────────────────

#[test]
fn test_fragment_size_dist_bins() {
    // 5 fragments in [0,10), 3 in [150,160)
    let mut lengths = vec![5i64; 5];
    lengths.extend(vec![155i64; 3]);
    let result =
        atac::call_atac_builtin("fragment_size_dist", vec![make_list_int(&lengths)]).unwrap();

    let table = match result {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    };
    // bin 0: [0,10) should have count 5
    let count_bin0 = get_int(&table.rows[0][2]);
    assert_eq!(count_bin0, 5);
    // bin 15: [150,160) should have count 3
    let count_bin15 = get_int(&table.rows[15][2]);
    assert_eq!(count_bin15, 3);
    // fractions sum to 1.0
    let frac_sum: f64 = table.rows.iter().map(|r| get_float(&r[3])).sum();
    assert!((frac_sum - 1.0).abs() < 1e-9);
}

#[test]
fn test_nfr_enrichment_ratio() {
    // 80 NFR (<150) and 40 mono (150-300) → ratio = 2.0
    let mut lengths: Vec<i64> = vec![100; 80]; // NFR
    lengths.extend(vec![200i64; 40]); // mono
    let result = atac::call_atac_builtin("nfr_enrichment", vec![make_list_int(&lengths)]).unwrap();
    let ratio = get_float(&result);
    assert!((ratio - 2.0).abs() < 1e-9);
}

#[test]
fn test_nfr_enrichment_zero_mono() {
    // No mono-nucleosome reads → return 0.0
    let lengths = vec![100i64; 10];
    let result = atac::call_atac_builtin("nfr_enrichment", vec![make_list_int(&lengths)]).unwrap();
    assert_eq!(get_float(&result), 0.0);
}

#[test]
fn test_nucleosome_fractions_sum_to_one() {
    let mut lengths: Vec<i64> = vec![50; 10]; // sub_nfr
    lengths.extend(vec![120i64; 10]); // nfr
    lengths.extend(vec![200i64; 10]); // mono
    lengths.extend(vec![400i64; 10]); // di
    lengths.extend(vec![600i64; 10]); // tri
    lengths.extend(vec![900i64; 10]); // higher

    let result =
        atac::call_atac_builtin("nucleosome_fractions", vec![make_list_int(&lengths)]).unwrap();
    let total = match &result {
        Value::Record(m) => ["sub_nfr", "nfr", "mono", "di", "tri", "higher"]
            .iter()
            .map(|k| get_float(m.get(*k).unwrap()))
            .sum::<f64>(),
        _ => panic!("expected Record"),
    };
    assert!((total - 1.0).abs() < 1e-9);
}

#[test]
fn test_atac_qc_metrics() {
    let mut lengths: Vec<i64> = vec![120; 60]; // NFR
    lengths.extend(vec![200i64; 30]); // mono
    lengths.extend(vec![600i64; 10]); // large

    let result = atac::call_atac_builtin("atac_qc", vec![make_list_int(&lengths)]).unwrap();
    assert_eq!(get_record_int(&result, "n_fragments"), 100);
    let nfr_frac = get_record_float(&result, "nfr_fraction");
    assert!((nfr_frac - 0.6).abs() < 1e-9);
    let large_frac = get_record_float(&result, "fraction_large");
    assert!((large_frac - 0.1).abs() < 1e-9);
    // all required keys present
    let keys = [
        "n_fragments",
        "nfr_fraction",
        "mono_fraction",
        "nfr_enrichment",
        "median_fragment_size",
        "fraction_large",
    ];
    match &result {
        Value::Record(m) => {
            for k in &keys {
                assert!(m.contains_key(*k), "missing key {k}");
            }
        }
        _ => panic!("expected Record"),
    }
}

// ── drug tests ───────────────────────────────────────────────────────

#[test]
fn test_fit_ic50_sigmoid() {
    // Generate clean sigmoid data with IC50=1.0, slope=1.0, top=100, bottom=0
    let concs = vec![0.01f64, 0.03, 0.1, 0.3, 1.0, 3.0, 10.0, 30.0, 100.0];
    let viabs: Vec<f64> = concs
        .iter()
        .map(|&c| 100.0 / (1.0 + (1.0 / c).powi(1)))
        .collect();

    let result = drug::call_drug_builtin(
        "fit_ic50",
        vec![make_list_float(&concs), make_list_float(&viabs)],
    )
    .unwrap();

    let r2 = get_record_float(&result, "r2");
    assert!(r2 > 0.9, "R² = {r2} should be > 0.9");
    let ic50 = get_record_float(&result, "ic50");
    assert!(ic50 > 0.0, "IC50 should be positive");
}

#[test]
fn test_dose_response_curve_formula() {
    // At conc = ic50, viability should be (top+bottom)/2 = 50
    let concs = vec![0.5f64, 1.0, 2.0];
    let result = drug::call_drug_builtin(
        "dose_response_curve",
        vec![
            make_list_float(&concs),
            Value::Float(1.0),   // ic50
            Value::Float(1.0),   // slope
            Value::Float(100.0), // top
            Value::Float(0.0),   // bottom
        ],
    )
    .unwrap();

    match result {
        Value::List(vals) => {
            // At conc=1.0 (index 1), viability = 100/(1+1) = 50
            let mid = get_float(&vals[1]);
            assert!(
                (mid - 50.0).abs() < 1e-6,
                "at IC50 viability should be 50, got {mid}"
            );
            // At conc=0.5 < IC50, viability should be < 50
            assert!(get_float(&vals[0]) < 50.0);
            // At conc=2.0 > IC50, viability should be > 50
            assert!(get_float(&vals[2]) > 50.0);
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn test_auc_monotone_decreasing() {
    // Monotone decreasing: higher concentration → lower viability
    let concs = vec![0.1f64, 1.0, 10.0, 100.0];
    let viabs = vec![90.0f64, 70.0, 30.0, 5.0];
    let result = drug::call_drug_builtin(
        "auc_response",
        vec![make_list_float(&concs), make_list_float(&viabs)],
    )
    .unwrap();
    let auc = get_float(&result);
    assert!(auc > 0.0, "AUC should be positive for viable cells");
}

#[test]
fn test_bliss_synergy_known() {
    // Drug A: 80% viab, Drug B: 50% viab
    // Bliss expected: 0.8 * 0.5 * 100 = 40%
    // Observed combo: 30% → synergy = 30 - 40 = -10 (antagonism)
    let result = drug::call_drug_builtin(
        "bliss_synergy",
        vec![Value::Float(80.0), Value::Float(50.0), Value::Float(30.0)],
    )
    .unwrap();
    let synergy = get_float(&result);
    assert!(
        (synergy - (-10.0)).abs() < 1e-6,
        "expected -10, got {synergy}"
    );
}

#[test]
fn test_loewe_synergy_additivity() {
    // When conc_a = 0.5*ic50_a and conc_b = 0.5*ic50_b → CI = 1.0 → synergy = 0
    let result = drug::call_drug_builtin(
        "loewe_synergy",
        vec![
            Value::Float(2.0),  // ic50_a
            Value::Float(4.0),  // ic50_b
            Value::Float(1.0),  // conc_a = 0.5 * ic50_a
            Value::Float(2.0),  // conc_b = 0.5 * ic50_b
            Value::Float(50.0), // observed (unused in formula)
        ],
    )
    .unwrap();
    let synergy = get_float(&result);
    assert!(
        (synergy - 0.0).abs() < 1e-9,
        "Loewe additive CI=1 → synergy=0, got {synergy}"
    );
}

#[test]
fn test_drug_rank_ascending() {
    let cols = vec!["drug".to_string(), "ic50".to_string()];
    let rows = vec![
        vec![Value::Str("DrugC".into()), Value::Float(10.0)],
        vec![Value::Str("DrugA".into()), Value::Float(1.0)],
        vec![Value::Str("DrugB".into()), Value::Float(5.0)],
    ];
    let table = Value::Table(Table::new(cols, rows));

    let result = drug::call_drug_builtin("drug_rank", vec![table]).unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.columns.last().unwrap(), "rank");
            // First row should be DrugA (lowest IC50) with rank 1
            let drug_name = match &t.rows[0][0] {
                Value::Str(s) => s.as_str(),
                _ => panic!(),
            };
            assert_eq!(drug_name, "DrugA");
            let rank = get_int(&t.rows[0][2]);
            assert_eq!(rank, 1);
        }
        _ => panic!("expected Table"),
    }
}
