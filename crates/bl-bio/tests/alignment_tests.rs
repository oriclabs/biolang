use bl_bio::alignment::call_alignment_builtin;
use bl_core::value::{BioSequence, Value};
use std::collections::HashMap;

// ── Helper ──────────────────────────────────────────────────────

fn get_record_int(val: &Value, field: &str) -> i64 {
    match val {
        Value::Record(r) => match r.get(field) {
            Some(Value::Int(n)) => *n,
            other => panic!("expected Int for '{field}', got {other:?}"),
        },
        _ => panic!("expected Record"),
    }
}

fn get_record_list(val: &Value, field: &str) -> Vec<Value> {
    match val {
        Value::Record(r) => match r.get(field) {
            Some(Value::List(l)) => l.clone(),
            other => panic!("expected List for '{field}', got {other:?}"),
        },
        _ => panic!("expected Record"),
    }
}

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

// ── MSA Tests ────────────────────────────────────────────────────

#[test]
fn test_msa_basic_three_sequences() {
    let result = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![
            Value::Str("ACGTACGT".into()),
            Value::Str("ACGTACGT".into()),
            Value::Str("ACGTACGT".into()),
        ])],
    )
    .unwrap();

    let n_seqs = get_record_int(&result, "n_seqs");
    assert_eq!(n_seqs, 3);

    let sequences = get_record_list(&result, "sequences");
    assert_eq!(sequences.len(), 3);

    let names = get_record_list(&result, "names");
    assert_eq!(names.len(), 3);

    // All same length
    let length = get_record_int(&result, "length");
    assert!(length >= 8); // at least as long as input
}

#[test]
fn test_msa_with_divergent_sequences() {
    let result = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![
            Value::Str("ACGTACGT".into()),
            Value::Str("ACGACGT".into()),  // one deletion
            Value::Str("ACGTACG".into()),  // shorter
        ])],
    )
    .unwrap();

    let n_seqs = get_record_int(&result, "n_seqs");
    assert_eq!(n_seqs, 3);

    let sequences = get_record_list(&result, "sequences");
    // All aligned sequences should be the same length
    let lens: Vec<usize> = sequences
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.len(),
            _ => panic!("expected Str"),
        })
        .collect();
    assert!(lens.windows(2).all(|w| w[0] == w[1]), "aligned sequences must be equal length: {:?}", lens);
}

#[test]
fn test_msa_with_dna_values() {
    let result = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![
            Value::DNA(BioSequence { data: "ACGT".into() }),
            Value::DNA(BioSequence { data: "ACGT".into() }),
        ])],
    )
    .unwrap();

    assert_eq!(get_record_int(&result, "n_seqs"), 2);
}

#[test]
fn test_msa_requires_at_least_two() {
    let result = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![Value::Str("ACGT".into())])],
    );
    assert!(result.is_err());
}

#[test]
fn test_msa_rejects_non_list() {
    let result = call_alignment_builtin("msa", vec![Value::Str("ACGT".into())]);
    assert!(result.is_err());
}

// ── Distance Matrix Tests ────────────────────────────────────────

#[test]
fn test_distance_matrix_identical() {
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![Value::List(vec![
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
        ])],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 2);
        assert_eq!(m.ncol, 2);
        // Diagonal should be 0
        assert_eq!(m.get(0, 0), 0.0);
        assert_eq!(m.get(1, 1), 0.0);
        // Off-diagonal should be 0 (identical sequences)
        assert_eq!(m.get(0, 1), 0.0);
        assert_eq!(m.get(1, 0), 0.0);
        // Should have names
        assert!(m.row_names.is_some());
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_different() {
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![Value::List(vec![
            Value::Str("AAAA".into()),
            Value::Str("TTTT".into()),
        ])],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        // p_distance should be 1.0 (all positions differ)
        assert_eq!(m.get(0, 1), 1.0);
        // Symmetric
        assert_eq!(m.get(0, 1), m.get(1, 0));
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_p_distance_half() {
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![Value::List(vec![
            Value::Str("AATT".into()),
            Value::Str("AAAA".into()),
        ])],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        // 2 out of 4 differ = 0.5
        assert_eq!(m.get(0, 1), 0.5);
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_hamming_model() {
    let mut opts = HashMap::new();
    opts.insert("model".to_string(), Value::Str("hamming".into()));
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![
            Value::List(vec![
                Value::Str("AATT".into()),
                Value::Str("AAAA".into()),
            ]),
            Value::Record(opts),
        ],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        // 2 raw differences
        assert_eq!(m.get(0, 1), 2.0);
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_jc_model() {
    let mut opts = HashMap::new();
    opts.insert("model".to_string(), Value::Str("jc".into()));
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![
            Value::List(vec![
                Value::Str("AACC".into()),
                Value::Str("AATT".into()),
            ]),
            Value::Record(opts),
        ],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        let d = m.get(0, 1);
        // JC distance should be > p_distance for moderate divergence
        assert!(d > 0.5, "JC distance {d} should exceed p_distance 0.5");
        assert!(d.is_finite());
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_three_seqs() {
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![Value::List(vec![
            Value::Str("ACGT".into()),
            Value::Str("ACGA".into()),
            Value::Str("TTTT".into()),
        ])],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 3);
        assert_eq!(m.ncol, 3);
        // Diagonal is 0
        assert_eq!(m.get(0, 0), 0.0);
        assert_eq!(m.get(1, 1), 0.0);
        assert_eq!(m.get(2, 2), 0.0);
        // seq1 vs seq2 should be less than seq1 vs seq3
        assert!(m.get(0, 1) < m.get(0, 2));
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_from_msa_record() {
    // First run MSA, then pass result to distance_matrix
    let msa = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![
            Value::Str("ACGTACGT".into()),
            Value::Str("ACGTACGT".into()),
        ])],
    )
    .unwrap();

    let result = call_alignment_builtin("distance_matrix", vec![msa]).unwrap();
    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 2);
        assert_eq!(m.get(0, 1), 0.0); // identical sequences
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_distance_matrix_unknown_model() {
    let mut opts = HashMap::new();
    opts.insert("model".to_string(), Value::Str("kimura".into()));
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![
            Value::List(vec![
                Value::Str("ACGT".into()),
                Value::Str("ACGA".into()),
            ]),
            Value::Record(opts),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_distance_matrix_gaps_ignored() {
    let result = call_alignment_builtin(
        "distance_matrix",
        vec![Value::List(vec![
            Value::Str("AC-GT".into()),
            Value::Str("AC-GT".into()),
        ])],
    )
    .unwrap();

    if let Value::Matrix(m) = result {
        assert_eq!(m.get(0, 1), 0.0); // identical non-gap positions
    } else {
        panic!("expected Matrix");
    }
}

// ── Conservation Scores Tests ────────────────────────────────────

#[test]
fn test_conservation_perfectly_conserved() {
    let result = call_alignment_builtin(
        "conservation_scores",
        vec![Value::List(vec![
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
        ])],
    )
    .unwrap();

    if let Value::List(scores) = result {
        assert_eq!(scores.len(), 4);
        for s in &scores {
            if let Value::Float(f) = s {
                assert_eq!(*f, 1.0, "perfectly conserved column should score 1.0");
            } else {
                panic!("expected Float");
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_conservation_no_conservation() {
    let result = call_alignment_builtin(
        "conservation_scores",
        vec![Value::List(vec![
            Value::Str("AAAA".into()),
            Value::Str("CCCC".into()),
            Value::Str("GGGG".into()),
            Value::Str("TTTT".into()),
        ])],
    )
    .unwrap();

    if let Value::List(scores) = result {
        assert_eq!(scores.len(), 4);
        for s in &scores {
            if let Value::Float(f) = s {
                assert!(
                    *f < 0.01,
                    "column with all different bases should have near-zero conservation, got {f}"
                );
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_conservation_partial() {
    // 3 out of 4 sequences match at each position
    let result = call_alignment_builtin(
        "conservation_scores",
        vec![Value::List(vec![
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
            Value::Str("TTTT".into()),
        ])],
    )
    .unwrap();

    if let Value::List(scores) = result {
        assert_eq!(scores.len(), 4);
        for s in &scores {
            if let Value::Float(f) = s {
                assert!(*f > 0.0 && *f <= 1.0, "partial conservation should be in (0, 1], got {f}");
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_conservation_with_gaps() {
    let result = call_alignment_builtin(
        "conservation_scores",
        vec![Value::List(vec![
            Value::Str("A-GT".into()),
            Value::Str("A-GT".into()),
        ])],
    )
    .unwrap();

    if let Value::List(scores) = result {
        assert_eq!(scores.len(), 4);
        // Position 0 (A/A): perfectly conserved
        assert_eq!(scores[0], Value::Float(1.0));
        // Position 1 (gap/gap): all gaps => 0.0
        assert_eq!(scores[1], Value::Float(0.0));
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_conservation_from_msa_record() {
    let msa = call_alignment_builtin(
        "msa",
        vec![Value::List(vec![
            Value::Str("ACGTACGT".into()),
            Value::Str("ACGTACGT".into()),
        ])],
    )
    .unwrap();

    let result = call_alignment_builtin("conservation_scores", vec![msa]).unwrap();
    if let Value::List(scores) = result {
        assert!(!scores.is_empty());
        // Identical sequences: all positions should be 1.0
        for s in &scores {
            if let Value::Float(f) = s {
                assert_eq!(*f, 1.0);
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_conservation_requires_at_least_two() {
    let result = call_alignment_builtin(
        "conservation_scores",
        vec![Value::List(vec![Value::Str("ACGT".into())])],
    );
    assert!(result.is_err());
}

// ── End-to-end pipeline test ─────────────────────────────────────

#[test]
fn test_msa_to_distance_matrix_to_conservation_pipeline() {
    // This is the pipeline that makes the multispecies tutorial work:
    // msa → distance_matrix, msa → conservation_scores
    let seqs = vec![
        Value::Str("ACGTACGTAA".into()),
        Value::Str("ACGAACGTAA".into()),
        Value::Str("TTGTACGTAA".into()),
    ];

    // Step 1: MSA
    let msa = call_alignment_builtin("msa", vec![Value::List(seqs)]).unwrap();
    let n_seqs = get_record_int(&msa, "n_seqs");
    assert_eq!(n_seqs, 3);

    // Step 2: Distance matrix from MSA
    let dist = call_alignment_builtin("distance_matrix", vec![msa.clone()]).unwrap();
    if let Value::Matrix(m) = &dist {
        assert_eq!(m.nrow, 3);
        assert_eq!(m.ncol, 3);
        // Symmetric
        assert_eq!(m.get(0, 1), m.get(1, 0));
    } else {
        panic!("expected Matrix from distance_matrix");
    }

    // Step 3: Conservation scores from MSA
    let cons = call_alignment_builtin("conservation_scores", vec![msa]).unwrap();
    if let Value::List(scores) = cons {
        assert!(!scores.is_empty());
        for s in &scores {
            if let Value::Float(f) = s {
                assert!(*f >= 0.0 && *f <= 1.0, "conservation must be in [0,1]");
            }
        }
    } else {
        panic!("expected List from conservation_scores");
    }
}
