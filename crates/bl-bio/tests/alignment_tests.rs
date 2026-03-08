use bl_bio::alignment::call_alignment_builtin;
use bl_core::value::{BioSequence, Value};
use std::collections::HashMap;

#[test]
fn test_align_identical() {
    let result = call_alignment_builtin(
        "align",
        vec![Value::Str("ACGT".into()), Value::Str("ACGT".into())],
    )
    .unwrap();
    if let Value::Record(r) = result {
        assert_eq!(r.get("identity"), Some(&Value::Float(1.0)));
        assert_eq!(r.get("gaps"), Some(&Value::Int(0)));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_align_with_gap() {
    let result = call_alignment_builtin(
        "align",
        vec![Value::Str("ACGT".into()), Value::Str("AGT".into())],
    )
    .unwrap();
    if let Value::Record(r) = result {
        let a1 = r.get("aligned1").unwrap();
        let a2 = r.get("aligned2").unwrap();
        let has_gap = matches!(a1, Value::Str(s) if s.contains('-'))
            || matches!(a2, Value::Str(s) if s.contains('-'));
        assert!(has_gap, "expected gap in alignment");
    }
}

#[test]
fn test_align_local() {
    let mut opts = HashMap::new();
    opts.insert("type".to_string(), Value::Str("local".into()));
    let result = call_alignment_builtin(
        "align",
        vec![
            Value::Str("XXXACGTXXX".into()),
            Value::Str("ACGT".into()),
            Value::Record(opts),
        ],
    )
    .unwrap();
    if let Value::Record(r) = result {
        if let Some(Value::Int(score)) = r.get("score") {
            assert!(*score > 0);
        }
    }
}

#[test]
fn test_align_dna_values() {
    let result = call_alignment_builtin(
        "align",
        vec![
            Value::DNA(BioSequence { data: "ACGT".into() }),
            Value::DNA(BioSequence { data: "ACGT".into() }),
        ],
    )
    .unwrap();
    assert!(matches!(result, Value::Record(_)));
}

#[test]
fn test_edit_distance() {
    let result = call_alignment_builtin(
        "edit_distance",
        vec![Value::Str("kitten".into()), Value::Str("sitting".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_hamming_distance_equal() {
    let result = call_alignment_builtin(
        "hamming_distance",
        vec![Value::Str("ACGT".into()), Value::Str("ACGA".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_hamming_distance_unequal() {
    let result = call_alignment_builtin(
        "hamming_distance",
        vec![Value::Str("ACGT".into()), Value::Str("ACG".into())],
    );
    assert!(result.is_err());
}

#[test]
fn test_score_matrix_blosum62() {
    let result = call_alignment_builtin(
        "score_matrix",
        vec![Value::Str("blosum62".into())],
    )
    .unwrap();
    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 20);
        assert_eq!(m.ncol, 20);
        assert_eq!(m.get(0, 0), 4.0);
    } else {
        panic!("expected Matrix");
    }
}
