use bio_core::fastq_ops::*;

#[test]
fn test_trim_adapter_found() {
    let mut seq = Vec::new();
    seq.extend_from_slice(&[b'A'; 40]);
    seq.extend_from_slice(b"AGATCGGAAGAG"); // 12bp adapter
    let adapters: Vec<&[u8]> = vec![b"AGATCGGAAGAG"];
    let pos = trim_adapter(&seq, &adapters, 10);
    assert_eq!(pos, 40);
}

#[test]
fn test_trim_adapter_no_match() {
    let seq = b"ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT";
    let adapters: Vec<&[u8]> = vec![b"AGATCGGAAGAG"];
    let pos = trim_adapter(seq, &adapters, 10);
    assert_eq!(pos, seq.len());
}

#[test]
fn test_trim_adapter_partial() {
    // Adapter starts at position 35, only 10bp overlap at end of read
    let mut seq = vec![b'A'; 35];
    seq.extend_from_slice(b"AGATCGGAAG"); // 10bp partial
    let adapters: Vec<&[u8]> = vec![b"AGATCGGAAGAG"];
    let pos = trim_adapter(&seq, &adapters, 10);
    assert_eq!(pos, 35);
}

#[test]
fn test_trim_quality_mixed() {
    // 10 high-quality (Q40='I') then 5 low-quality (Q2='#')
    let qual = b"IIIIIIIIII#####";
    let pos = trim_quality(qual, 20, 4);
    assert_eq!(pos, 12);
}

#[test]
fn test_trim_quality_all_good() {
    let qual = b"IIIIIIIIII"; // all Q40
    let pos = trim_quality(qual, 20, 4);
    assert_eq!(pos, 10);
}

#[test]
fn test_trim_quality_all_bad() {
    let qual = b"!!!!!!!!!!"; // all Q0
    let pos = trim_quality(qual, 20, 4);
    assert_eq!(pos, 0);
}

#[test]
fn test_trim_quality_empty() {
    let pos = trim_quality(b"", 20, 4);
    assert_eq!(pos, 0);
}

#[test]
fn test_trim_quality_zero_window() {
    let qual = b"IIIII";
    let pos = trim_quality(qual, 20, 0);
    assert_eq!(pos, 5);
}

#[test]
fn test_adapters_constant() {
    assert_eq!(ADAPTERS.len(), 3);
    assert_eq!(ADAPTERS[0], b"AGATCGGAAGAG");
}
