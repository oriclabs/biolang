use bio_core::Kmer;

#[test]
fn test_encode_decode() {
    let kmer = Kmer::from_str("ACGT", 4).unwrap();
    assert_eq!(kmer.decode(), "ACGT");
}

#[test]
fn test_reverse_complement() {
    let kmer = Kmer::from_str("ACGT", 4).unwrap();
    let rc = kmer.reverse_complement();
    assert_eq!(rc.decode(), "ACGT"); // ACGT is its own reverse complement
}

#[test]
fn test_reverse_complement_asym() {
    let kmer = Kmer::from_str("AACG", 4).unwrap();
    let rc = kmer.reverse_complement();
    assert_eq!(rc.decode(), "CGTT");
}

#[test]
fn test_canonical() {
    let k1 = Kmer::from_str("AACG", 4).unwrap();
    let k2 = Kmer::from_str("CGTT", 4).unwrap();
    assert_eq!(k1.canonical(), k2.canonical());
}

#[test]
fn test_extract_all() {
    let kmers = Kmer::extract_all("ACGTACGT", 4);
    assert_eq!(kmers.len(), 5);
    assert_eq!(kmers[0].decode(), "ACGT");
    assert_eq!(kmers[4].decode(), "ACGT");
}

#[test]
fn test_count() {
    let counts = Kmer::count("ACGTACGT", 4);
    // ACGT is its own RC, so canonical = ACGT
    assert!(!counts.is_empty());
}

#[test]
fn test_spectrum() {
    let counts = Kmer::count("AAAAAAAAAA", 3);
    let spectrum = Kmer::spectrum(&counts);
    assert!(!spectrum.is_empty());
}

#[test]
fn test_minimizers() {
    let mins = Kmer::minimizers("ACGTACGTACGT", 3, 3);
    assert!(!mins.is_empty());
}

#[test]
fn test_invalid_k() {
    assert!(Kmer::from_str("ACG", 4).is_none()); // length mismatch
    assert!(Kmer::from_str("ACGN", 4).is_none()); // invalid base
}
