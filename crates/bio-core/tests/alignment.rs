use bio_core::alignment::*;

#[test]
fn test_identical_sequences() {
    let params = AlignParams::default();
    let result = align("ACGT", "ACGT", &params);
    assert_eq!(result.identity, 1.0);
    assert_eq!(result.gaps, 0);
    assert_eq!(result.score, 8); // 4 * match_score(2)
}

#[test]
fn test_global_with_gap() {
    let params = AlignParams {
        match_score: 2,
        mismatch_score: -1,
        gap_open: -5,
        gap_extend: -1,
        mode: AlignMode::Global,
    };
    let result = align("ACGT", "AGT", &params);
    assert!(result.aligned1.contains('-') || result.aligned2.contains('-'));
}

#[test]
fn test_local_alignment() {
    let params = AlignParams {
        match_score: 2,
        mismatch_score: -1,
        gap_open: -5,
        gap_extend: -1,
        mode: AlignMode::Local,
    };
    let result = align("XXXACGTXXX", "ACGT", &params);
    assert!(result.score > 0);
    // Local alignment should find the matching region
    assert!(result.identity > 0.5);
}

#[test]
fn test_edit_distance_known() {
    assert_eq!(edit_distance("kitten", "sitting"), 3);
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("abc", "abc"), 0);
}

#[test]
fn test_hamming_equal_length() {
    assert_eq!(hamming_distance("ACGT", "ACGT").unwrap(), 0);
    assert_eq!(hamming_distance("ACGT", "TGCA").unwrap(), 4);
    assert_eq!(hamming_distance("ACGT", "ACGA").unwrap(), 1);
}

#[test]
fn test_hamming_unequal_length() {
    assert!(hamming_distance("ACGT", "ACG").is_err());
}

#[test]
fn test_blosum62_known_pair() {
    // A-A should be 4 in BLOSUM62
    let a_idx = aa_index('A').unwrap();
    assert_eq!(BLOSUM62[a_idx][a_idx], 4);
    // W-W should be 11
    let w_idx = aa_index('W').unwrap();
    assert_eq!(BLOSUM62[w_idx][w_idx], 11);
}

#[test]
fn test_scoring_matrix_lookup() {
    assert!(scoring_matrix("blosum62").is_some());
    assert!(scoring_matrix("pam250").is_some());
    assert!(scoring_matrix("blosum45").is_some());
    assert!(scoring_matrix("unknown").is_none());
}
