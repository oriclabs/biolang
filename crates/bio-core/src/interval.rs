use std::fmt;

/// Strand orientation for genomic intervals.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Strand {
    Plus,
    Minus,
    Unknown,
}

impl Strand {
    /// Parse strand from a string: `"+"` → Plus, `"-"` → Minus, anything else → Unknown.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "+" => Strand::Plus,
            "-" => Strand::Minus,
            _ => Strand::Unknown,
        }
    }
}

impl fmt::Display for Strand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Strand::Plus => write!(f, "+"),
            Strand::Minus => write!(f, "-"),
            Strand::Unknown => write!(f, "."),
        }
    }
}

/// A single genomic interval.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GenomicInterval {
    pub chrom: String,
    pub start: i64,
    pub end: i64,
    pub strand: Strand,
}

impl fmt::Display for GenomicInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}-{}({})", self.chrom, self.start, self.end, self.strand)
    }
}
