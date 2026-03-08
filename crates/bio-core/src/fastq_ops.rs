//! Pure FASTQ trimming algorithms. No framework dependencies.

/// Common Illumina adapter sequences.
pub const ADAPTERS: &[&[u8]] = &[
    b"AGATCGGAAGAG",  // Illumina Universal Adapter
    b"CTGTCTCTTATAC", // Nextera Transposase
    b"TGGAATTCTCGG",  // Illumina Small RNA 3' Adapter
];

/// Scan from 3' end for adapter match. Returns trim position (length of kept sequence).
///
/// For each adapter, checks all possible start positions in the read where at least
/// `min_overlap` bases match the adapter prefix. Returns the earliest match position.
pub fn trim_adapter(seq: &[u8], adapters: &[&[u8]], min_overlap: usize) -> usize {
    let seq_len = seq.len();
    for adapter in adapters {
        let adapter_len = adapter.len();
        let max_start = seq_len.saturating_sub(min_overlap);
        for start in 0..=max_start {
            let overlap = (seq_len - start).min(adapter_len);
            if overlap < min_overlap {
                continue;
            }
            if seq[start..start + overlap] == adapter[..overlap] {
                return start;
            }
        }
    }
    seq_len
}

/// Sliding window quality trim from 3' end. Returns trim position.
///
/// Scans backwards from the 3' end using a window of `window` bases. If the mean
/// Phred quality in the window falls below `threshold`, trims at that position.
/// Quality bytes are Phred+33 encoded (subtract 33 to get Phred score).
pub fn trim_quality(qual: &[u8], threshold: u8, window: usize) -> usize {
    let len = qual.len();
    if len == 0 || window == 0 {
        return len;
    }
    let mut pos = len;
    while pos > 0 {
        let win_start = pos.saturating_sub(window);
        let win_end = pos;
        let win_len = win_end - win_start;
        let sum: u64 = qual[win_start..win_end]
            .iter()
            .map(|&q| q.saturating_sub(33) as u64)
            .sum();
        let mean = sum / win_len as u64;
        if mean >= threshold as u64 {
            return pos;
        }
        pos -= 1;
    }
    0
}
