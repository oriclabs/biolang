use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::fmt;

/// A k-mer stored as 2-bit encoded u64.
///
/// Encoding: A=00, C=01, G=10, T=11
/// Supports k up to 32 (64 bits / 2 bits per base).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Kmer {
    pub encoded: u64,
    pub k: u8,
}

impl Kmer {
    /// Encode a single base to 2 bits.
    fn encode_base(b: u8) -> Option<u64> {
        match b {
            b'A' | b'a' => Some(0b00),
            b'C' | b'c' => Some(0b01),
            b'G' | b'g' => Some(0b10),
            b'T' | b't' => Some(0b11),
            _ => None,
        }
    }

    /// Decode 2 bits to a base character.
    fn decode_base(bits: u64) -> char {
        match bits & 0b11 {
            0b00 => 'A',
            0b01 => 'C',
            0b10 => 'G',
            0b11 => 'T',
            _ => unreachable!(),
        }
    }

    /// Create a new Kmer from a DNA sequence string.
    /// Returns None if sequence contains non-ACGT characters or k > 32.
    pub fn from_str(seq: &str, k: u8) -> Option<Self> {
        if k == 0 || k > 32 || seq.len() != k as usize {
            return None;
        }
        let mut encoded = 0u64;
        for (i, b) in seq.bytes().enumerate() {
            let bits = Self::encode_base(b)?;
            encoded |= bits << (2 * (k as usize - 1 - i));
        }
        Some(Kmer { encoded, k })
    }

    /// Decode this k-mer back to a DNA string.
    pub fn decode(&self) -> String {
        let mut result = String::with_capacity(self.k as usize);
        for i in 0..self.k as usize {
            let bits = (self.encoded >> (2 * (self.k as usize - 1 - i))) & 0b11;
            result.push(Self::decode_base(bits));
        }
        result
    }

    /// Compute the reverse complement.
    /// Complement: A↔T (00↔11), C↔G (01↔10) — just XOR with 0b11
    /// Then reverse the 2-bit pairs.
    pub fn reverse_complement(&self) -> Kmer {
        let k = self.k as usize;
        // Complement all bits
        let complemented = self.encoded ^ ((1u64 << (2 * k)) - 1);
        // Reverse 2-bit pairs
        let mut rc = 0u64;
        for i in 0..k {
            let bits = (complemented >> (2 * i)) & 0b11;
            rc |= bits << (2 * (k - 1 - i));
        }
        Kmer { encoded: rc, k: self.k }
    }

    /// Canonical form: min(self, reverse_complement).
    /// Strand-agnostic representation.
    pub fn canonical(&self) -> Kmer {
        let rc = self.reverse_complement();
        if self.encoded <= rc.encoded { *self } else { rc }
    }

    /// Extract all k-mers from a sequence using a sliding window.
    pub fn extract_all(seq: &str, k: u8) -> Vec<Kmer> {
        if k == 0 || k > 32 || (seq.len() as u8) < k {
            return Vec::new();
        }
        let k_usize = k as usize;
        let mask = if k == 32 { u64::MAX } else { (1u64 << (2 * k_usize)) - 1 };
        let bytes = seq.as_bytes();

        let mut kmers = Vec::with_capacity(seq.len() - k_usize + 1);
        let mut encoded = 0u64;
        let mut valid_run = 0usize;

        for &b in bytes.iter() {
            if let Some(bits) = Self::encode_base(b) {
                encoded = ((encoded << 2) | bits) & mask;
                valid_run += 1;
                if valid_run >= k_usize {
                    kmers.push(Kmer { encoded, k });
                }
            } else {
                // Non-ACGT base: reset window
                valid_run = 0;
                encoded = 0;
            }
        }
        kmers
    }

    /// Count k-mers in a sequence. Returns HashMap of canonical kmer → count.
    pub fn count(seq: &str, k: u8) -> HashMap<Kmer, u64> {
        let mut counts = HashMap::new();
        for kmer in Self::extract_all(seq, k) {
            *counts.entry(kmer.canonical()).or_insert(0) += 1;
        }
        counts
    }

    /// Count canonical k-mers directly into a pre-existing u64→i64 counter.
    /// Avoids per-sequence HashMap allocation and String decoding.
    /// Generic over hasher so callers can use FxHashMap for speed.
    pub fn count_into<S: BuildHasher>(seq: &str, k: u8, counter: &mut HashMap<u64, i64, S>) {
        if k == 0 || k > 32 || (seq.len()) < k as usize {
            return;
        }
        let k_usize = k as usize;
        let mask = if k == 32 { u64::MAX } else { (1u64 << (2 * k_usize)) - 1 };
        let bytes = seq.as_bytes();
        let mut encoded = 0u64;
        let mut valid_run = 0usize;

        for &b in bytes {
            if let Some(bits) = Self::encode_base(b) {
                encoded = ((encoded << 2) | bits) & mask;
                valid_run += 1;
                if valid_run >= k_usize {
                    let kmer = Kmer { encoded, k };
                    let canonical = kmer.canonical().encoded;
                    *counter.entry(canonical).or_insert(0) += 1;
                }
            } else {
                valid_run = 0;
                encoded = 0;
            }
        }
    }

    /// Compute k-mer spectrum: frequency → how many k-mers have that frequency.
    pub fn spectrum(counts: &HashMap<Kmer, u64>) -> HashMap<u64, u64> {
        let mut spectrum = HashMap::new();
        for &count in counts.values() {
            *spectrum.entry(count).or_insert(0) += 1;
        }
        spectrum
    }

    /// Compute minimizers: the smallest k-mer in each window of w consecutive k-mers.
    pub fn minimizers(seq: &str, k: u8, w: usize) -> Vec<(Kmer, usize)> {
        let kmers = Self::extract_all(seq, k);
        if kmers.is_empty() || w == 0 || w > kmers.len() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut last_pos = usize::MAX;

        for i in 0..=(kmers.len() - w) {
            let window = &kmers[i..i + w];
            let (min_idx, min_kmer) = window
                .iter()
                .enumerate()
                .min_by_key(|(_, km)| km.canonical().encoded)
                .unwrap();
            let abs_pos = i + min_idx;
            if abs_pos != last_pos {
                result.push((min_kmer.canonical(), abs_pos));
                last_pos = abs_pos;
            }
        }
        result
    }
}

impl fmt::Display for Kmer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Kmer({})", self.decode())
    }
}
