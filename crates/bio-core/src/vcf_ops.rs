//! Pure VCF helper algorithms. No framework dependencies.

use serde::Serialize;

// ── Types ───────────────────────────────────────────────────────────────────

/// Classification of a variant by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VariantType {
    Snp,
    Indel,
    Mnp,
    Other,
}

/// Summary statistics for a set of VCF variants.
#[derive(Debug, Clone, Serialize)]
pub struct VariantSummary {
    pub snp: u64,
    pub indel: u64,
    pub mnp: u64,
    pub other: u64,
    pub transitions: u64,
    pub transversions: u64,
    pub ts_tv_ratio: f64,
    pub multiallelic: u64,
}

// ── Genotype parsing ────────────────────────────────────────────────────────

/// Parse a genotype field from a VCF FORMAT+sample column pair.
///
/// The GT sub-field is located by position in the FORMAT string.
/// Returns `None` if GT is not present or can't be parsed.
/// Each allele is `None` for missing (`.`) or `Some(index)`.
///
/// # Examples
/// ```
/// use bio_core::vcf_ops::parse_gt;
/// assert_eq!(parse_gt("GT:DP", "0/1:30"), Some(vec![Some(0), Some(1)]));
/// assert_eq!(parse_gt("GT", "./."), Some(vec![None, None]));
/// assert_eq!(parse_gt("GT", "0|1"), Some(vec![Some(0), Some(1)]));
/// ```
pub fn parse_gt(format: &str, genotype: &str) -> Option<Vec<Option<u8>>> {
    let fmt_fields: Vec<&str> = format.split(':').collect();
    let gt_idx = fmt_fields.iter().position(|&f| f == "GT")?;
    let gt_fields: Vec<&str> = genotype.split(':').collect();
    let gt = gt_fields.get(gt_idx)?;
    let sep = if gt.contains('|') { '|' } else { '/' };
    let alleles: Vec<Option<u8>> = gt
        .split(sep)
        .map(|a| {
            if a == "." {
                None
            } else {
                a.parse::<u8>().ok()
            }
        })
        .collect();
    Some(alleles)
}

// ── Variant classification ──────────────────────────────────────────────────

/// Check if a single-nucleotide substitution is a transition (purine↔purine or pyrimidine↔pyrimidine).
///
/// Returns `true` for A↔G and C↔T changes, `false` for transversions.
pub fn is_transition(ref_base: &str, alt_base: &str) -> bool {
    matches!(
        (ref_base, alt_base),
        ("A", "G") | ("G", "A") | ("C", "T") | ("T", "C")
    )
}

/// Classify a single variant allele by comparing REF and ALT.
pub fn classify_variant(ref_allele: &str, alt_allele: &str) -> VariantType {
    if alt_allele == "*" || alt_allele == "." {
        VariantType::Other
    } else if ref_allele.len() == 1 && alt_allele.len() == 1 {
        VariantType::Snp
    } else if ref_allele.len() == alt_allele.len() && ref_allele.len() > 1 {
        VariantType::Mnp
    } else if ref_allele.len() != alt_allele.len() {
        VariantType::Indel
    } else {
        VariantType::Other
    }
}

/// Compute variant summary statistics from parallel slices of REF alleles and ALT allele lists.
///
/// Each entry in `variants` is `(ref_allele, alt_alleles)`.
pub fn summarize_variants(variants: &[(&str, &[&str])]) -> VariantSummary {
    let mut snp = 0u64;
    let mut indel = 0u64;
    let mut mnp = 0u64;
    let mut other = 0u64;
    let mut transitions = 0u64;
    let mut transversions = 0u64;
    let mut multiallelic = 0u64;

    for &(ref_allele, alt_alleles) in variants {
        if alt_alleles.len() > 1 {
            multiallelic += 1;
        }
        for alt in alt_alleles {
            match classify_variant(ref_allele, alt) {
                VariantType::Snp => {
                    snp += 1;
                    if is_transition(ref_allele, alt) {
                        transitions += 1;
                    } else {
                        transversions += 1;
                    }
                }
                VariantType::Indel => indel += 1,
                VariantType::Mnp => mnp += 1,
                VariantType::Other => other += 1,
            }
        }
    }

    let ts_tv = if transversions > 0 {
        transitions as f64 / transversions as f64
    } else {
        0.0
    };

    VariantSummary {
        snp,
        indel,
        mnp,
        other,
        transitions,
        transversions,
        ts_tv_ratio: ts_tv,
        multiallelic,
    }
}
