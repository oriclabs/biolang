use bio_core::{GenomicInterval, Strand};
use bio_core::interval_ops::*;

fn iv(chrom: &str, start: i64, end: i64) -> GenomicInterval {
    GenomicInterval {
        chrom: chrom.to_string(),
        start,
        end,
        strand: Strand::Unknown,
    }
}

#[test]
fn test_sort_intervals() {
    let mut ivs = vec![iv("chr2", 100, 200), iv("chr1", 300, 400), iv("chr1", 100, 200)];
    sort_intervals(&mut ivs);
    assert_eq!(ivs[0], iv("chr1", 100, 200));
    assert_eq!(ivs[1], iv("chr1", 300, 400));
    assert_eq!(ivs[2], iv("chr2", 100, 200));
}

#[test]
fn test_overlaps() {
    assert!(overlaps(&iv("chr1", 100, 200), &iv("chr1", 150, 250)));
    assert!(!overlaps(&iv("chr1", 100, 200), &iv("chr1", 200, 300))); // adjacent = no overlap
    assert!(!overlaps(&iv("chr1", 100, 200), &iv("chr2", 100, 200))); // different chrom
}

#[test]
fn test_overlap_bp() {
    assert_eq!(overlap_bp(&iv("chr1", 100, 200), &iv("chr1", 150, 250)), 50);
    assert_eq!(overlap_bp(&iv("chr1", 100, 200), &iv("chr1", 200, 300)), 0);
    assert_eq!(overlap_bp(&iv("chr1", 100, 200), &iv("chr2", 100, 200)), 0);
}

#[test]
fn test_merge() {
    let ivs = vec![
        iv("chr1", 100, 200),
        iv("chr1", 150, 300),
        iv("chr1", 500, 600),
        iv("chr2", 100, 200),
    ];
    let merged = merge(&ivs);
    assert_eq!(merged.len(), 3);
    assert_eq!(merged[0], iv("chr1", 100, 300));
    assert_eq!(merged[1], iv("chr1", 500, 600));
    assert_eq!(merged[2], iv("chr2", 100, 200));
}

#[test]
fn test_merge_empty() {
    assert!(merge(&[]).is_empty());
}

#[test]
fn test_intersect() {
    let a = vec![iv("chr1", 100, 200), iv("chr1", 300, 400), iv("chr2", 100, 200)];
    let b = vec![iv("chr1", 150, 250), iv("chr2", 500, 600)];
    let result = intersect(&a, &b);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], iv("chr1", 100, 200));
}

#[test]
fn test_subtract() {
    let a = vec![iv("chr1", 100, 200), iv("chr1", 300, 400)];
    let b = vec![iv("chr1", 150, 250)];
    let result = subtract(&a, &b);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], iv("chr1", 300, 400));
}

#[test]
fn test_closest_distance() {
    let a = vec![iv("chr1", 100, 200)];
    let b = vec![iv("chr1", 500, 600), iv("chr1", 300, 400)];
    let result = closest_distance(&a, &b);
    assert_eq!(result.len(), 1);
    // a midpoint = 150, closest b midpoint = 350 (300-400), distance = 200
    assert_eq!(result[0], (0, Some(200)));
}

#[test]
fn test_closest_distance_no_match() {
    let a = vec![iv("chr1", 100, 200)];
    let b = vec![iv("chr2", 100, 200)];
    let result = closest_distance(&a, &b);
    assert_eq!(result[0], (0, None));
}

#[test]
fn test_flanks() {
    let ivs = vec![iv("chr1", 1000, 2000)];
    let result = flanks(&ivs, 500, 500);
    assert_eq!(result.len(), 1);
    let (up, down) = &result[0];
    assert_eq!(up.start, 500);
    assert_eq!(up.end, 1000);
    assert_eq!(down.start, 2000);
    assert_eq!(down.end, 2500);
}

#[test]
fn test_flanks_clamp() {
    let ivs = vec![iv("chr1", 100, 200)];
    let result = flanks(&ivs, 500, 300);
    let (up, _) = &result[0];
    assert_eq!(up.start, 0); // clamped
}

#[test]
fn test_extend() {
    let ivs = vec![iv("chr1", 1000, 2000)];
    let result = extend(&ivs, 100);
    assert_eq!(result[0], iv("chr1", 900, 2100));
}

#[test]
fn test_extend_clamp() {
    let ivs = vec![iv("chr1", 50, 100)];
    let result = extend(&ivs, 100);
    assert_eq!(result[0].start, 0); // clamped
}

#[test]
fn test_complement() {
    let ivs = vec![iv("chr1", 100, 200), iv("chr1", 300, 400)];
    let result = complement(&ivs);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], iv("chr1", 0, 100));
    assert_eq!(result[1], iv("chr1", 200, 300));
}
