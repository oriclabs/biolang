//! `align` scored on match/mismatch only, and knew two modes. Four Stronghold
//! problems therefore hand-wrote a Needleman-Wunsch purely to reach BLOSUM62,
//! and overlap alignment could not be expressed at all.
//!
//! Each expectation here is the answer Rosalind publishes for that problem's
//! sample dataset, so these fail if the scoring drifts.

use bl_core::value::Value;
use bl_runtime::seq::call_seq_builtin;

fn score(args: Vec<Value>) -> i64 {
    match call_seq_builtin("align", args).expect("align") {
        Value::Record(fields) => match fields.get("score") {
            Some(Value::Int(n)) => *n,
            other => panic!("expected an Int score, got {other:?}"),
        },
        other => panic!("expected a Record, got {other:?}"),
    }
}

fn s(text: &str) -> Value {
    Value::Str(text.into())
}

#[test]
fn glob_global_alignment_with_blosum62() {
    // https://rosalind.info/problems/glob/ — BLOSUM62, constant gap 5.
    let got = score(vec![
        s("PLEASANTLY"),
        s("MEANLY"),
        s("global"),
        Value::Int(0),
        Value::Int(0),
        Value::Int(-5),
        Value::Int(0),
        s("blosum62"),
    ]);
    assert_eq!(got, 8);
}

#[test]
fn loca_local_alignment_with_pam250() {
    // https://rosalind.info/problems/loca/ — PAM250, not BLOSUM62, gap 5.
    let got = score(vec![
        s("MEANLYPRTEINSTRING"),
        s("PLEASANTLYEINSTEIN"),
        s("local"),
        Value::Int(0),
        Value::Int(0),
        Value::Int(-5),
        Value::Int(0),
        s("pam250"),
    ]);
    assert_eq!(got, 23);
}

#[test]
fn gaff_affine_gaps_with_blosum62() {
    // https://rosalind.info/problems/gaff/ — BLOSUM62, open 11, extend 1.
    let got = score(vec![
        s("PRTEINS"),
        s("PRTWPSEIN"),
        s("global"),
        Value::Int(0),
        Value::Int(0),
        Value::Int(-1),
        Value::Int(-10),
        s("blosum62"),
    ]);
    assert_eq!(got, 8);
}

#[test]
fn oap_overlap_alignment() {
    // https://rosalind.info/problems/oap/ — match 1, mismatch -2, gap 2.
    // A suffix of the first against a prefix of the second.
    let got = score(vec![
        s("CTAAGGGATTCCGGTAATTAGACAG"),
        s("ATAGACCATATGTCAGTGACTGTGTAA"),
        s("overlap"),
        Value::Int(1),
        Value::Int(-2),
        Value::Int(-2),
        Value::Int(0),
    ]);
    assert_eq!(got, 1);
}

#[test]
fn overlap_is_not_the_same_as_global() {
    // Global has to pay for both overhangs; overlap forgives them, so it can
    // never score lower on the same input.
    let args = |mode: &str| {
        vec![
            s("AAAAGGGG"),
            s("GGGGTTTT"),
            s(mode),
            Value::Int(1),
            Value::Int(-1),
            Value::Int(-1),
            Value::Int(0),
        ]
    };
    assert!(score(args("overlap")) > score(args("global")));
}

#[test]
fn semiglobal_forgives_both_overhangs() {
    // A short sequence buried in a long one: semiglobal should not be charged
    // for the flanks, so it beats global.
    let args = |mode: &str| {
        vec![
            s("TTTTTTACGTACGTTTTTTT"),
            s("ACGTACGT"),
            s(mode),
            Value::Int(1),
            Value::Int(-1),
            Value::Int(-1),
            Value::Int(0),
        ]
    };
    assert!(score(args("semiglobal")) > score(args("global")));
}

#[test]
fn an_unknown_matrix_name_is_an_error() {
    let result = call_seq_builtin(
        "align",
        vec![
            s("ACGT"),
            s("ACGT"),
            s("global"),
            Value::Int(1),
            Value::Int(-1),
            Value::Int(-1),
            Value::Int(0),
            s("nonesuch62"),
        ],
    );
    assert!(result.is_err(), "an unknown matrix name should be refused");
}

#[test]
fn without_a_matrix_the_match_scores_still_apply() {
    let got = score(vec![
        s("ACGT"),
        s("ACGT"),
        s("global"),
        Value::Int(2),
        Value::Int(-1),
        Value::Int(-2),
    ]);
    assert_eq!(got, 8);
}
