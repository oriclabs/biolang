use bio_core::aligned_read::{parse_cigar_ref_len, parse_cigar_query_len};
use bio_core::AlignedRead;

fn test_read() -> AlignedRead {
    AlignedRead {
        qname: "read1".into(),
        flag: 99, // paired, proper_pair, mate_reverse, read1
        rname: "chr1".into(),
        pos: 100,
        mapq: 60,
        cigar: "50M2I10M5D20M".into(),
        rnext: "=".into(),
        pnext: 300,
        tlen: 250,
        seq: "ATCG".repeat(20),
        qual: "IIII".repeat(20),
    }
}

#[test]
fn test_flag_queries() {
    let r = test_read();
    assert!(r.is_paired());
    assert!(r.is_proper_pair());
    assert!(!r.is_unmapped());
    assert!(r.is_mapped());
    assert!(r.is_read1());
    assert!(!r.is_read2());
    assert!(!r.is_duplicate());
    assert!(!r.is_secondary());
    assert!(!r.is_supplementary());
    assert!(r.is_primary());
}

#[test]
fn test_cigar_ref_len() {
    // 50M + 10M + 5D + 20M = 85 (I doesn't consume ref)
    assert_eq!(parse_cigar_ref_len("50M2I10M5D20M"), 85);
}

#[test]
fn test_cigar_query_len() {
    // 50M + 2I + 10M + 20M = 82 (D doesn't consume query, but S would)
    assert_eq!(parse_cigar_query_len("50M2I10M5D20M"), 82);
}

#[test]
fn test_end_pos() {
    let r = test_read();
    assert_eq!(r.end_pos(), 100 + 85); // 185
}

#[test]
fn test_to_interval() {
    let r = test_read();
    let iv = r.to_interval();
    assert_eq!(iv.chrom, "chr1");
    assert_eq!(iv.start, 100);
    assert_eq!(iv.end, 185);
}

#[test]
fn test_unmapped_read() {
    let r = AlignedRead {
        qname: "unmapped".into(),
        flag: 4, // unmapped
        rname: "*".into(),
        pos: 0,
        mapq: 0,
        cigar: "*".into(),
        rnext: "*".into(),
        pnext: 0,
        tlen: 0,
        seq: "ATCG".into(),
        qual: "IIII".into(),
    };
    assert!(r.is_unmapped());
    assert!(!r.is_mapped());
}
