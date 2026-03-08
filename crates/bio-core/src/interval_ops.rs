//! Pure interval algorithms operating on `GenomicInterval`. No framework dependencies.

use crate::{GenomicInterval, Strand};

/// Sort intervals by chromosome (lexicographic) then by start position.
pub fn sort_intervals(intervals: &mut [GenomicInterval]) {
    intervals.sort_by(|a, b| a.chrom.cmp(&b.chrom).then(a.start.cmp(&b.start)));
}

/// Check whether two intervals overlap (same chromosome, overlapping coordinates).
pub fn overlaps(a: &GenomicInterval, b: &GenomicInterval) -> bool {
    a.chrom == b.chrom && a.start < b.end && b.start < a.end
}

/// Number of overlapping base pairs. Returns 0 if no overlap or different chromosomes.
pub fn overlap_bp(a: &GenomicInterval, b: &GenomicInterval) -> i64 {
    if a.chrom != b.chrom {
        return 0;
    }
    let ov = a.end.min(b.end) - a.start.max(b.start);
    ov.max(0)
}

/// Merge overlapping/touching intervals. Returns sorted, non-overlapping intervals.
pub fn merge(intervals: &[GenomicInterval]) -> Vec<GenomicInterval> {
    if intervals.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<GenomicInterval> = intervals.to_vec();
    sort_intervals(&mut sorted);

    let mut merged: Vec<GenomicInterval> = Vec::new();
    for iv in sorted {
        if let Some(last) = merged.last_mut() {
            if last.chrom == iv.chrom && iv.start <= last.end {
                last.end = last.end.max(iv.end);
                continue;
            }
        }
        merged.push(iv);
    }
    merged
}

/// Intersect: return intervals from `a` that overlap with any interval in `b`.
/// Each interval from `a` is included at most once (whole interval, not clipped).
pub fn intersect(a: &[GenomicInterval], b: &[GenomicInterval]) -> Vec<GenomicInterval> {
    let mut result = Vec::new();
    for ai in a {
        for bi in b {
            if overlaps(ai, bi) {
                result.push(ai.clone());
                break;
            }
        }
    }
    result
}

/// Subtract: return intervals from `a` that do NOT overlap with any interval in `b`.
pub fn subtract(a: &[GenomicInterval], b: &[GenomicInterval]) -> Vec<GenomicInterval> {
    let mut result = Vec::new();
    for ai in a {
        let has_overlap = b.iter().any(|bi| overlaps(ai, bi));
        if !has_overlap {
            result.push(ai.clone());
        }
    }
    result
}

/// For each interval in `a`, find the distance to the closest interval in `b` (same chromosome).
/// Distance is measured between midpoints. Returns `(index_in_a, Option<distance>)`.
/// `None` if no interval in `b` shares the same chromosome.
pub fn closest_distance(a: &[GenomicInterval], b: &[GenomicInterval]) -> Vec<(usize, Option<i64>)> {
    a.iter()
        .enumerate()
        .map(|(idx, ai)| {
            let a_mid = (ai.start + ai.end) / 2;
            let best = b
                .iter()
                .filter(|bi| bi.chrom == ai.chrom)
                .map(|bi| {
                    let b_mid = (bi.start + bi.end) / 2;
                    (a_mid - b_mid).abs()
                })
                .min();
            (idx, best)
        })
        .collect()
}

/// Generate upstream and downstream flanking regions for each interval.
/// Returns `(upstream_flank, downstream_flank)` pairs. Upstream start is clamped to 0.
pub fn flanks(
    intervals: &[GenomicInterval],
    upstream: i64,
    downstream: i64,
) -> Vec<(GenomicInterval, GenomicInterval)> {
    intervals
        .iter()
        .map(|iv| {
            let up = GenomicInterval {
                chrom: iv.chrom.clone(),
                start: (iv.start - upstream).max(0),
                end: iv.start,
                strand: iv.strand.clone(),
            };
            let down = GenomicInterval {
                chrom: iv.chrom.clone(),
                start: iv.end,
                end: iv.end + downstream,
                strand: iv.strand.clone(),
            };
            (up, down)
        })
        .collect()
}

/// Extend each interval by `distance` in both directions. Start is clamped to 0.
pub fn extend(intervals: &[GenomicInterval], distance: i64) -> Vec<GenomicInterval> {
    intervals
        .iter()
        .map(|iv| GenomicInterval {
            chrom: iv.chrom.clone(),
            start: (iv.start - distance).max(0),
            end: iv.end + distance,
            strand: iv.strand.clone(),
        })
        .collect()
}

/// Complement: given a set of intervals, return the gaps between them (per chromosome).
/// Input is sorted first. Gaps start at coordinate 0 on each chromosome.
pub fn complement(intervals: &[GenomicInterval]) -> Vec<GenomicInterval> {
    if intervals.is_empty() {
        return Vec::new();
    }
    let merged = merge(intervals);

    let mut result = Vec::new();
    let mut prev_end: i64 = 0;
    let mut prev_chrom = String::new();

    for iv in &merged {
        if iv.chrom != prev_chrom {
            prev_end = 0;
            prev_chrom = iv.chrom.clone();
        }
        if iv.start > prev_end {
            result.push(GenomicInterval {
                chrom: iv.chrom.clone(),
                start: prev_end,
                end: iv.start,
                strand: Strand::Unknown,
            });
        }
        prev_end = iv.end;
    }

    result
}
