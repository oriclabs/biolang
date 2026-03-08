use serde::{Deserialize, Serialize};

use crate::interval::{GenomicInterval, Strand};

/// A single aligned sequencing read (SAM/BAM record).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignedRead {
    pub qname: String,
    pub flag: u16,
    pub rname: String,
    pub pos: i64,
    pub mapq: u8,
    pub cigar: String,
    pub rnext: String,
    pub pnext: i64,
    pub tlen: i64,
    pub seq: String,
    pub qual: String,
}

// SAM flag bit constants
const FLAG_PAIRED: u16 = 0x1;
const FLAG_PROPER_PAIR: u16 = 0x2;
const FLAG_UNMAPPED: u16 = 0x4;
const FLAG_REVERSE: u16 = 0x10;
const FLAG_READ1: u16 = 0x40;
const FLAG_READ2: u16 = 0x80;
const FLAG_SECONDARY: u16 = 0x100;
const FLAG_DUPLICATE: u16 = 0x400;
const FLAG_SUPPLEMENTARY: u16 = 0x800;

impl AlignedRead {
    // ── Flag queries ──

    pub fn is_paired(&self) -> bool {
        self.flag & FLAG_PAIRED != 0
    }

    pub fn is_proper_pair(&self) -> bool {
        self.flag & FLAG_PROPER_PAIR != 0
    }

    pub fn is_unmapped(&self) -> bool {
        self.flag & FLAG_UNMAPPED != 0
    }

    pub fn is_mapped(&self) -> bool {
        !self.is_unmapped()
    }

    pub fn is_reverse(&self) -> bool {
        self.flag & FLAG_REVERSE != 0
    }

    pub fn is_read1(&self) -> bool {
        self.flag & FLAG_READ1 != 0
    }

    pub fn is_read2(&self) -> bool {
        self.flag & FLAG_READ2 != 0
    }

    pub fn is_duplicate(&self) -> bool {
        self.flag & FLAG_DUPLICATE != 0
    }

    pub fn is_secondary(&self) -> bool {
        self.flag & FLAG_SECONDARY != 0
    }

    pub fn is_supplementary(&self) -> bool {
        self.flag & FLAG_SUPPLEMENTARY != 0
    }

    pub fn is_primary(&self) -> bool {
        !self.is_secondary() && !self.is_supplementary()
    }

    // ── CIGAR operations ──

    /// Aligned length on the reference (sum of M/D/N/=/X ops).
    pub fn aligned_length(&self) -> i64 {
        parse_cigar_ref_len(&self.cigar)
    }

    /// Query (read) consumed length (sum of M/I/S/=/X ops).
    pub fn query_length(&self) -> i64 {
        parse_cigar_query_len(&self.cigar)
    }

    /// End position on the reference (pos + aligned_length).
    pub fn end_pos(&self) -> i64 {
        self.pos + self.aligned_length()
    }

    /// Convert to a GenomicInterval spanning the aligned region.
    pub fn to_interval(&self) -> GenomicInterval {
        let strand = if self.is_reverse() { Strand::Minus } else { Strand::Plus };
        GenomicInterval {
            chrom: self.rname.clone(),
            start: self.pos,
            end: self.end_pos(),
            strand,
        }
    }
}

/// Parse CIGAR string and return reference-consuming length (M/D/N/=/X).
pub fn parse_cigar_ref_len(cigar: &str) -> i64 {
    let mut len: i64 = 0;
    let mut num = 0i64;
    for c in cigar.chars() {
        if c.is_ascii_digit() {
            num = num * 10 + (c as i64 - '0' as i64);
        } else {
            match c {
                'M' | 'D' | 'N' | '=' | 'X' => len += num,
                _ => {}
            }
            num = 0;
        }
    }
    len
}

/// Parse CIGAR string and return query-consuming length (M/I/S/=/X).
pub fn parse_cigar_query_len(cigar: &str) -> i64 {
    let mut len: i64 = 0;
    let mut num = 0i64;
    for c in cigar.chars() {
        if c.is_ascii_digit() {
            num = num * 10 + (c as i64 - '0' as i64);
        } else {
            match c {
                'M' | 'I' | 'S' | '=' | 'X' => len += num,
                _ => {}
            }
            num = 0;
        }
    }
    len
}
