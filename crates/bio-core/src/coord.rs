use serde::{Deserialize, Serialize};

/// Coordinate system conventions used in genomics.
///
/// Different file formats use different conventions:
/// - BED: 0-based, half-open [start, end)
/// - VCF: 1-based, closed [start, end]
/// - GFF/GTF: 1-based, closed [start, end]
/// - SAM/BAM: 1-based, closed [start, end]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordSystem {
    Bed,
    Vcf,
    Gff,
    Sam,
    Unknown,
}

impl CoordSystem {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bed" => CoordSystem::Bed,
            "vcf" => CoordSystem::Vcf,
            "gff" | "gtf" => CoordSystem::Gff,
            "sam" | "bam" => CoordSystem::Sam,
            _ => CoordSystem::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CoordSystem::Bed => "bed",
            CoordSystem::Vcf => "vcf",
            CoordSystem::Gff => "gff",
            CoordSystem::Sam => "sam",
            CoordSystem::Unknown => "unknown",
        }
    }

    /// Whether this system is 0-based.
    pub fn is_zero_based(&self) -> bool {
        matches!(self, CoordSystem::Bed)
    }

    /// Convert a (start, end) from this system to 0-based half-open (BED-style).
    pub fn to_bed(&self, start: i64, end: i64) -> (i64, i64) {
        match self {
            CoordSystem::Bed | CoordSystem::Unknown => (start, end),
            // 1-based closed → 0-based half-open: start-1, end stays
            CoordSystem::Vcf | CoordSystem::Gff | CoordSystem::Sam => (start - 1, end),
        }
    }

    /// Convert a (start, end) from 0-based half-open (BED-style) to this system.
    pub fn from_bed(&self, start: i64, end: i64) -> (i64, i64) {
        match self {
            CoordSystem::Bed | CoordSystem::Unknown => (start, end),
            CoordSystem::Vcf | CoordSystem::Gff | CoordSystem::Sam => (start + 1, end),
        }
    }

    /// Convert coordinates from one system to another.
    pub fn convert(from: CoordSystem, to: CoordSystem, start: i64, end: i64) -> (i64, i64) {
        let (bed_start, bed_end) = from.to_bed(start, end);
        to.from_bed(bed_start, bed_end)
    }

    /// Check if two coordinate systems are compatible (same convention).
    pub fn is_compatible(&self, other: &CoordSystem) -> bool {
        match (self, other) {
            (CoordSystem::Unknown, _) | (_, CoordSystem::Unknown) => true,
            (a, b) => a.is_zero_based() == b.is_zero_based(),
        }
    }
}

impl std::fmt::Display for CoordSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
