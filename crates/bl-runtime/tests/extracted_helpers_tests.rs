//! Builtins extracted from helpers the example packs wrote by hand.
//!
//! Each of these existed as a `fn` in one or more `.bl` files before it existed
//! in the runtime: factorial in five, choose in three, a permutation generator
//! and a subsequence test in two each, and the pair of helpers that read a
//! residue score out of a BLOSUM matrix in four.

use bl_core::value::Value;
use bl_runtime::stats::call_stats_builtin;

fn call(name: &str, args: Vec<Value>) -> Value {
    call_stats_builtin(name, args).unwrap_or_else(|e| panic!("{name} failed: {e}"))
}

/// A three-residue corner of BLOSUM62, enough to check the lookup arithmetic.
/// The full matrix comes from `score_matrix()`, which lives in another module.
fn blosum_fragment() -> Value {
    Value::Matrix(
        bl_core::matrix::Matrix {
            //      A     W     P
            data: vec![
                4.0, -3.0, -1.0, // A
                -3.0, 11.0, -4.0, // W
                -1.0, -4.0, 7.0, // P
            ],
            nrow: 3,
            ncol: 3,
            row_names: Some(vec!["A".into(), "W".into(), "P".into()]),
            col_names: Some(vec!["A".into(), "W".into(), "P".into()]),
        }
        .into(),
    )
}

fn ints(vals: &[i64]) -> Value {
    Value::List(
        vals.iter()
            .map(|v| Value::Int(*v))
            .collect::<Vec<_>>()
            .into(),
    )
}

#[test]
fn factorial_is_exact_while_it_fits() {
    assert_eq!(call("factorial", vec![Value::Int(0)]), Value::Int(1));
    assert_eq!(
        call("factorial", vec![Value::Int(10)]),
        Value::Int(3_628_800)
    );
    // 20! is the largest that fits in an i64.
    assert_eq!(
        call("factorial", vec![Value::Int(20)]),
        Value::Int(2_432_902_008_176_640_000)
    );
}

#[test]
fn factorial_past_the_integer_limit_becomes_float() {
    match call("factorial", vec![Value::Int(21)]) {
        Value::Float(f) => assert!((f - 5.109_094_217_170_944e19).abs() < 1e6, "got {f}"),
        other => panic!("expected Float past 20!, got {other:?}"),
    }
}

#[test]
fn factorial_rejects_a_negative() {
    assert!(call_stats_builtin("factorial", vec![Value::Int(-1)]).is_err());
}

#[test]
fn choose_counts_combinations() {
    assert_eq!(
        call("choose", vec![Value::Int(5), Value::Int(2)]),
        Value::Int(10)
    );
    // Five-card hands from a deck.
    assert_eq!(
        call("choose", vec![Value::Int(52), Value::Int(5)]),
        Value::Int(2_598_960)
    );
    // Taking more than there are.
    assert_eq!(
        call("choose", vec![Value::Int(3), Value::Int(5)]),
        Value::Int(0)
    );
}

#[test]
fn choose_does_not_overflow_on_a_small_answer() {
    // Building 40! first would overflow long before reaching this.
    assert_eq!(
        call("choose", vec![Value::Int(40), Value::Int(20)]),
        Value::Int(137_846_528_820)
    );
}

#[test]
fn permutations_are_lexicographic_and_complete() {
    match call("permutations", vec![ints(&[1, 2, 3])]) {
        Value::List(rows) => {
            assert_eq!(rows.len(), 6);
            assert_eq!(rows[0], ints(&[1, 2, 3]));
            assert_eq!(rows[5], ints(&[3, 2, 1]));
        }
        other => panic!("expected a List, got {other:?}"),
    }
}

#[test]
fn permutations_refuses_a_size_that_would_not_finish() {
    let big: Vec<Value> = (0..15).map(Value::Int).collect();
    assert!(call_stats_builtin("permutations", vec![Value::List(big.into())]).is_err());
}

#[test]
fn is_subsequence_allows_gaps_but_not_reordering() {
    let yes = call(
        "is_subsequence",
        vec![Value::Str("ACG".into()), Value::Str("AXCXG".into())],
    );
    assert_eq!(yes, Value::Bool(true));
    let no = call(
        "is_subsequence",
        vec![Value::Str("GCA".into()), Value::Str("AXCXG".into())],
    );
    assert_eq!(no, Value::Bool(false));
}

#[test]
fn lcs_returns_a_longest_common_subsequence() {
    let got = call(
        "lcs",
        vec![
            Value::Str("AACCTTGG".into()),
            Value::Str("ACACTGTGA".into()),
        ],
    );
    match got {
        // More than one is correct; all of them are six long.
        Value::Str(s) => assert_eq!(s.len(), 6, "got {s}"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn binary_search_reports_minus_one_when_absent() {
    let sorted = ints(&[10, 20, 30, 40, 50]);
    assert_eq!(
        call("binary_search", vec![sorted.clone(), Value::Int(40)]),
        Value::Int(3)
    );
    assert_eq!(
        call("binary_search", vec![sorted, Value::Int(35)]),
        Value::Int(-1)
    );
}

#[test]
fn argmin_and_argmax_break_ties_towards_the_front() {
    assert_eq!(call("argmin", vec![ints(&[5, 2, 9, 2])]), Value::Int(1));
    assert_eq!(call("argmax", vec![ints(&[5, 2, 9, 9])]), Value::Int(2));
    // An empty list is an error, as it is for the rest of this module.
    assert!(call_stats_builtin("argmin", vec![ints(&[])]).is_err());
}

#[test]
fn swap_exchanges_two_positions() {
    assert_eq!(
        call(
            "swap",
            vec![ints(&[1, 2, 3, 4]), Value::Int(0), Value::Int(3)]
        ),
        ints(&[4, 2, 3, 1])
    );
    assert!(call_stats_builtin("swap", vec![ints(&[1, 2]), Value::Int(0), Value::Int(9)]).is_err());
}

#[test]
fn substitution_score_reads_a_blosum_pair() {
    let matrix = blosum_fragment();
    let pair = |a: &str, b: &str| {
        call(
            "substitution_score",
            vec![matrix.clone(), Value::Str(a.into()), Value::Str(b.into())],
        )
    };
    assert_eq!(pair("W", "W"), Value::Float(11.0));
    assert_eq!(pair("A", "A"), Value::Float(4.0));
    assert_eq!(pair("A", "W"), Value::Float(-3.0));
    // Symmetric, as a substitution matrix should be.
    assert_eq!(pair("P", "W"), pair("W", "P"));
    // Case does not matter for a residue name.
    assert_eq!(pair("w", "w"), Value::Float(11.0));
}

#[test]
fn substitution_score_rejects_a_residue_it_does_not_know() {
    let matrix = blosum_fragment();
    assert!(call_stats_builtin(
        "substitution_score",
        vec![matrix, Value::Str("?".into()), Value::Str("A".into())]
    )
    .is_err());
}

#[test]
fn unique_keeps_first_appearance_and_separates_lookalikes() {
    // Bucketed by display form, decided by equality: 1, 1.0 and "1" all render
    // as "1" and must still be three distinct values.
    match call("unique", vec![ints(&[3, 1, 3, 2, 1, 3])]) {
        Value::List(v) => assert_eq!(v.as_ref(), &[Value::Int(3), Value::Int(1), Value::Int(2)]),
        other => panic!("expected List, got {other:?}"),
    }
    let mixed = Value::List(vec![Value::Int(1), Value::Float(1.0), Value::Str("1".into())].into());
    match call("unique", vec![mixed]) {
        Value::List(v) => assert_eq!(v.len(), 3, "lookalikes collapsed: {v:?}"),
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn unique_stays_linear() {
    // Was a scan of everything kept so far, per element: the distinct 8-mers of
    // a megabase took nearly four minutes. Four times the input must not be
    // sixteen times the work.
    let build = |n: i64| {
        let values: Vec<Value> = (0..n).map(|i| Value::Int(i % (n / 2))).collect();
        let start = std::time::Instant::now();
        call("unique", vec![Value::List(values.into())]);
        start.elapsed().as_secs_f64()
    };
    let small = build(4_000);
    let large = build(16_000);
    assert!(
        large < small * 40.0 + 0.5,
        "unique looks superlinear: 4k took {small:.4}s, 16k took {large:.4}s"
    );
}
