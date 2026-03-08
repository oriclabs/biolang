use bio_core::seq_ops::*;

// -- Validation --

#[test]
fn test_is_valid_dna() {
    assert!(is_valid_dna("ATGCN"));
    assert!(is_valid_dna("atgcn"));
    assert!(!is_valid_dna("AUGC")); // U is RNA
    assert!(!is_valid_dna("ATGCX"));
}

#[test]
fn test_is_valid_rna() {
    assert!(is_valid_rna("AUGCN"));
    assert!(is_valid_rna("augcn"));
    assert!(!is_valid_rna("ATGC")); // T is DNA
}

#[test]
fn test_is_valid_nucleotide() {
    assert!(is_valid_nucleotide("ATCG"));
    assert!(is_valid_nucleotide("AUGC"));
    assert!(is_valid_nucleotide("ATCGN-"));
    assert!(!is_valid_nucleotide("ATCGX"));
    assert!(!is_valid_nucleotide("ATCG 123"));
}

#[test]
fn test_detect_molecule() {
    assert_eq!(detect_molecule("ATGC"), MoleculeType::Dna);
    assert_eq!(detect_molecule("AUGC"), MoleculeType::Rna);
    assert_eq!(detect_molecule("AGCC"), MoleculeType::Ambiguous); // no T or U
    assert_eq!(detect_molecule("ATUGC"), MoleculeType::Ambiguous); // both T and U
}

// -- Transcription --

#[test]
fn test_transcribe() {
    assert_eq!(transcribe("ATGATCG"), "AUGAUCG");
    assert_eq!(transcribe("atgatcg"), "augaucg");
}

#[test]
fn test_back_transcribe() {
    assert_eq!(back_transcribe("AUGAUCG"), "ATGATCG");
    assert_eq!(back_transcribe("augaucg"), "atgatcg");
}

// -- Complement --

#[test]
fn test_complement_dna() {
    assert_eq!(complement_dna("ATCG"), "TAGC");
    assert_eq!(complement_dna("atcg"), "tagc");
}

#[test]
fn test_complement_rna() {
    assert_eq!(complement_rna("AUGC"), "UACG");
}

#[test]
fn test_complement_auto() {
    assert_eq!(complement("ATCG"), "TAGC");
    assert_eq!(complement("AUGC"), "UACG");
}

#[test]
fn test_reverse_complement_dna() {
    assert_eq!(reverse_complement_dna("ATCG"), "CGAT");
    assert_eq!(reverse_complement_dna("GATTACA"), "TGTAATC");
}

#[test]
fn test_reverse_complement_rna() {
    assert_eq!(reverse_complement_rna("AUGC"), "GCAU");
}

#[test]
fn test_reverse_complement_auto() {
    assert_eq!(reverse_complement("ATCG"), "CGAT");
    assert_eq!(reverse_complement("AUGC"), "GCAU");
    assert_eq!(reverse_complement("ATCGN"), "NCGAT");
}

// -- Metrics --

#[test]
fn test_gc_content() {
    assert_eq!(gc_content("ATGC"), 0.5);
    assert_eq!(gc_content("GGGG"), 1.0);
    assert_eq!(gc_content("AAAA"), 0.0);
    assert_eq!(gc_content(""), 0.0);
    assert_eq!(gc_content("gcgc"), 1.0);
}

#[test]
fn test_n_content() {
    assert_eq!(n_content("NNNN"), 1.0);
    assert_eq!(n_content("ATGC"), 0.0);
    assert_eq!(n_content("ANNG"), 0.5);
    assert_eq!(n_content(""), 0.0);
}

#[test]
fn test_effective_length() {
    assert_eq!(effective_length("ATCG"), 4);
    assert_eq!(effective_length("AT CG"), 4);
    assert_eq!(effective_length("AT-CG"), 4);
    assert_eq!(effective_length("AT CG-"), 4);
}

// -- Translation --

#[test]
fn test_codon_to_amino_acid() {
    assert_eq!(codon_to_amino_acid("ATG"), 'M');
    assert_eq!(codon_to_amino_acid("AUG"), 'M');
    assert_eq!(codon_to_amino_acid("UAA"), '*');
    assert_eq!(codon_to_amino_acid("TAA"), '*');
    assert_eq!(codon_to_amino_acid("UAG"), '*');
    assert_eq!(codon_to_amino_acid("UGA"), '*');
    assert_eq!(codon_to_amino_acid("UGG"), 'W');
    assert_eq!(codon_to_amino_acid("GGG"), 'G');
    assert_eq!(codon_to_amino_acid("XYZ"), 'X');
}

#[test]
fn test_translate() {
    assert_eq!(translate("ATGTAA"), "M*");
    assert_eq!(translate("TTTTTT"), "FF");
    assert_eq!(translate("ATG"), "M");
    assert_eq!(translate("AUGAUCUAA"), "MI*");
}

#[test]
fn test_translate_to_stop() {
    assert_eq!(translate_to_stop("AUGAUCUAA"), "MI");
    assert_eq!(translate_to_stop("ATGATCGATCG"), "MID"); // last CG ignored
}

#[test]
fn test_translate_dna_full() {
    let rna = transcribe("ATGATCGATCG");
    assert_eq!(translate_to_stop(&rna), "MID");
}

// -- Search --

#[test]
fn test_find_motif() {
    assert_eq!(find_motif("ATGATGATG", "ATG"), vec![0, 3, 6]);
    assert_eq!(find_motif("ATGATGATG", "atg"), vec![0, 3, 6]);
    assert_eq!(find_motif("ATCG", "GGG"), Vec::<usize>::new());
    assert_eq!(find_motif("", "ATG"), Vec::<usize>::new());
}

#[test]
fn test_kmers() {
    assert_eq!(kmers("ATCG", 2), vec!["AT", "TC", "CG"]);
    assert_eq!(kmers("ATCG", 4), vec!["ATCG"]);
    assert!(kmers("ATCG", 5).is_empty());
    assert!(kmers("ATCG", 0).is_empty());
}

// -- ORF finding --

#[test]
fn test_find_orfs_basic() {
    // ATG (start) ... TAA (stop) = ORF
    let seq = "ATGATCGATTAA"; // ATG ATC GAT TAA = M I D *
    let orfs = find_orfs(seq, 0);
    assert_eq!(orfs.len(), 1);
    assert_eq!(orfs[0].start, 0);
    assert_eq!(orfs[0].end, 12);
    assert_eq!(orfs[0].frame, 0);
    assert_eq!(orfs[0].protein, "MID");
}

#[test]
fn test_find_orfs_min_length() {
    let seq = "ATGATCGATTAA"; // 12 nt ORF
    assert_eq!(find_orfs(seq, 12).len(), 1);
    assert_eq!(find_orfs(seq, 13).len(), 0); // too short
}

#[test]
fn test_find_orfs_rna() {
    let seq = "AUGAUCGAUUAA"; // AUG AUC GAU UAA = M I D *
    let orfs = find_orfs(seq, 0);
    assert_eq!(orfs.len(), 1);
    assert_eq!(orfs[0].protein, "MID");
}

#[test]
fn test_find_orfs_in_frame() {
    let seq = "XATGATCTAAXXX"; // frame 1: ATG ATC TAA
    let orfs = find_orfs_in_frame(seq, 1, 0);
    assert_eq!(orfs.len(), 1);
    assert_eq!(orfs[0].start, 1);
    assert_eq!(orfs[0].end, 10);
    assert_eq!(orfs[0].frame, 1);
}
