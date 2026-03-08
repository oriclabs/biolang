use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::interval::GenomicInterval;
use crate::vcf_ops::{self, VariantType};

/// A gene annotation with location and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gene {
    pub symbol: String,
    pub gene_id: String,
    pub chrom: String,
    pub start: i64,
    pub end: i64,
    pub strand: String,
    pub biotype: String,
    pub description: String,
}

/// A genetic variant (VCF-like record).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub chrom: String,
    pub pos: i64,
    pub id: String,
    pub ref_allele: String,
    pub alt_allele: String,
    pub quality: f64,
    pub filter: String,
    #[serde(default)]
    pub info: HashMap<String, String>,
    #[serde(default)]
    pub genotypes: Vec<Vec<Option<u8>>>,
    #[serde(default)]
    pub phased: bool,
}

impl Variant {
    /// Classify the variant type (SNP, Indel, MNP, Other).
    pub fn variant_type(&self) -> VariantType {
        // Use first alt allele for classification
        let first_alt = self.alt_alleles().into_iter().next().unwrap_or("");
        vcf_ops::classify_variant(&self.ref_allele, first_alt)
    }

    /// True if this is a single-nucleotide polymorphism.
    pub fn is_snp(&self) -> bool {
        self.variant_type() == VariantType::Snp
    }

    /// True if this is an insertion or deletion.
    pub fn is_indel(&self) -> bool {
        self.variant_type() == VariantType::Indel
    }

    /// True if this is a multi-nucleotide polymorphism.
    pub fn is_mnp(&self) -> bool {
        self.variant_type() == VariantType::Mnp
    }

    /// True if this is a transition (purine-purine or pyrimidine-pyrimidine).
    pub fn is_transition(&self) -> bool {
        if !self.is_snp() {
            return false;
        }
        vcf_ops::is_transition(&self.ref_allele, self.alt_alleles().first().unwrap_or(&""))
    }

    /// True if this is a transversion (not a transition SNP).
    pub fn is_transversion(&self) -> bool {
        self.is_snp() && !self.is_transition()
    }

    /// True if any genotype is heterozygous (has different allele indices).
    pub fn is_het(&self) -> bool {
        self.genotypes.iter().any(|gt| {
            let alleles: Vec<_> = gt.iter().filter_map(|a| *a).collect();
            alleles.len() >= 2 && alleles.windows(2).any(|w| w[0] != w[1])
        })
    }

    /// True if all genotype alleles are reference (0).
    pub fn is_hom_ref(&self) -> bool {
        !self.genotypes.is_empty()
            && self.genotypes.iter().all(|gt| gt.iter().all(|a| *a == Some(0)))
    }

    /// True if all genotype alleles are the same non-reference allele.
    pub fn is_hom_alt(&self) -> bool {
        self.genotypes.iter().any(|gt| {
            let alleles: Vec<_> = gt.iter().filter_map(|a| *a).collect();
            alleles.len() >= 2 && alleles[0] > 0 && alleles.iter().all(|&a| a == alleles[0])
        })
    }

    /// Split alt_allele on comma to get individual alternate alleles.
    pub fn alt_alleles(&self) -> Vec<&str> {
        self.alt_allele.split(',').collect()
    }

    /// True if there are multiple alternate alleles.
    pub fn is_multiallelic(&self) -> bool {
        self.alt_allele.contains(',')
    }

    /// Convert to a GenomicInterval spanning the variant.
    pub fn to_interval(&self) -> GenomicInterval {
        let end = self.pos + self.ref_allele.len() as i64;
        GenomicInterval {
            chrom: self.chrom.clone(),
            start: self.pos,
            end,
            strand: crate::interval::Strand::Unknown,
        }
    }
}

/// A genome assembly with chromosome information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub name: String,
    pub species: String,
    pub assembly: String,
    /// (chromosome_name, length_in_bp)
    pub chromosomes: Vec<(String, i64)>,
}

impl Genome {
    /// Built-in genome registry.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "GRCh38" | "hg38" => Some(Self {
                name: "GRCh38".into(),
                species: "Homo sapiens".into(),
                assembly: "GRCh38.p14".into(),
                chromosomes: grch38_chromosomes(),
            }),
            "GRCh37" | "hg19" => Some(Self {
                name: "GRCh37".into(),
                species: "Homo sapiens".into(),
                assembly: "GRCh37.p13".into(),
                chromosomes: grch37_chromosomes(),
            }),
            "T2T-CHM13" | "hs1" => Some(Self {
                name: "T2T-CHM13".into(),
                species: "Homo sapiens".into(),
                assembly: "T2T-CHM13v2.0".into(),
                chromosomes: t2t_chromosomes(),
            }),
            "GRCm39" | "mm39" => Some(Self {
                name: "GRCm39".into(),
                species: "Mus musculus".into(),
                assembly: "GRCm39".into(),
                chromosomes: mm39_chromosomes(),
            }),
            _ => None,
        }
    }
}

fn grch38_chromosomes() -> Vec<(String, i64)> {
    vec![
        ("chr1".into(), 248956422), ("chr2".into(), 242193529), ("chr3".into(), 198295559),
        ("chr4".into(), 190214555), ("chr5".into(), 181538259), ("chr6".into(), 170805979),
        ("chr7".into(), 159345973), ("chr8".into(), 145138636), ("chr9".into(), 138394717),
        ("chr10".into(), 133797422), ("chr11".into(), 135086622), ("chr12".into(), 133275309),
        ("chr13".into(), 114364328), ("chr14".into(), 107043718), ("chr15".into(), 101991189),
        ("chr16".into(), 90338345), ("chr17".into(), 83257441), ("chr18".into(), 80373285),
        ("chr19".into(), 58617616), ("chr20".into(), 64444167), ("chr21".into(), 46709983),
        ("chr22".into(), 50818468), ("chrX".into(), 156040895), ("chrY".into(), 57227415),
    ]
}

fn grch37_chromosomes() -> Vec<(String, i64)> {
    vec![
        ("chr1".into(), 249250621), ("chr2".into(), 243199373), ("chr3".into(), 198022430),
        ("chr4".into(), 191154276), ("chr5".into(), 180915260), ("chr6".into(), 171115067),
        ("chr7".into(), 159138663), ("chr8".into(), 146364022), ("chr9".into(), 141213431),
        ("chr10".into(), 135534747), ("chr11".into(), 135006516), ("chr12".into(), 133851895),
        ("chr13".into(), 115169878), ("chr14".into(), 107349540), ("chr15".into(), 102531392),
        ("chr16".into(), 90354753), ("chr17".into(), 81195210), ("chr18".into(), 78077248),
        ("chr19".into(), 59128983), ("chr20".into(), 63025520), ("chr21".into(), 48129895),
        ("chr22".into(), 51304566), ("chrX".into(), 155270560), ("chrY".into(), 59373566),
    ]
}

fn t2t_chromosomes() -> Vec<(String, i64)> {
    vec![
        ("chr1".into(), 248387328), ("chr2".into(), 242696752), ("chr3".into(), 201105948),
        ("chr4".into(), 193574945), ("chr5".into(), 182045439), ("chr6".into(), 172126628),
        ("chr7".into(), 160567428), ("chr8".into(), 146259331), ("chr9".into(), 150617247),
        ("chr10".into(), 134758134), ("chr11".into(), 135127769), ("chr12".into(), 133324548),
        ("chr13".into(), 113566686), ("chr14".into(), 101161492), ("chr15".into(), 99753195),
        ("chr16".into(), 96330374), ("chr17".into(), 84276897), ("chr18".into(), 80542538),
        ("chr19".into(), 61707364), ("chr20".into(), 66210255), ("chr21".into(), 45090682),
        ("chr22".into(), 51324926), ("chrX".into(), 154259566),
    ]
}

fn mm39_chromosomes() -> Vec<(String, i64)> {
    vec![
        ("chr1".into(), 195154279), ("chr2".into(), 181755017), ("chr3".into(), 159745316),
        ("chr4".into(), 156860686), ("chr5".into(), 151758149), ("chr6".into(), 149588044),
        ("chr7".into(), 144995196), ("chr8".into(), 130127694), ("chr9".into(), 124359700),
        ("chr10".into(), 130530862), ("chr11".into(), 121843856), ("chr12".into(), 120092757),
        ("chr13".into(), 120883175), ("chr14".into(), 125139656), ("chr15".into(), 104073951),
        ("chr16".into(), 98008968), ("chr17".into(), 95294699), ("chr18".into(), 90720763),
        ("chr19".into(), 61420004), ("chrX".into(), 169476592), ("chrY".into(), 91455967),
    ]
}

/// Phred quality score operations.
pub struct QualityOps;

impl QualityOps {
    /// Parse a quality string (ASCII Phred+33) into raw quality scores.
    pub fn from_ascii(s: &str) -> Vec<u8> {
        s.bytes().map(|b| b.saturating_sub(33)).collect()
    }

    /// Mean Phred quality score.
    pub fn mean_phred(scores: &[u8]) -> f64 {
        if scores.is_empty() { return 0.0; }
        scores.iter().map(|&q| q as f64).sum::<f64>() / scores.len() as f64
    }

    /// Minimum Phred quality score.
    pub fn min_phred(scores: &[u8]) -> u8 {
        scores.iter().copied().min().unwrap_or(0)
    }

    /// Average error rate from Phred scores: P(error) = 10^(-Q/10).
    pub fn error_rate(scores: &[u8]) -> f64 {
        if scores.is_empty() { return 0.0; }
        let total: f64 = scores.iter().map(|&q| 10f64.powf(-(q as f64) / 10.0)).sum();
        total / scores.len() as f64
    }

    /// Trim from 3' end where quality drops below threshold.
    pub fn trim_quality(scores: &[u8], threshold: u8) -> usize {
        // Find the rightmost position where a window of 5bp has mean >= threshold
        let window = 5.min(scores.len());
        if window == 0 { return 0; }
        let mut best_end = scores.len();
        for i in (window..=scores.len()).rev() {
            let mean: f64 = scores[i - window..i].iter().map(|&q| q as f64).sum::<f64>() / window as f64;
            if mean >= threshold as f64 {
                best_end = i;
                break;
            }
            best_end = i - 1;
        }
        best_end
    }
}
