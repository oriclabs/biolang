use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Tsv,
    Json,
    Bed,
    Vcf,
    Gff,
    Gtf,
    Fasta,
    Fastq,
}

impl Format {
    pub const ALL: [Self; 9] = [
        Self::Csv,
        Self::Tsv,
        Self::Json,
        Self::Bed,
        Self::Vcf,
        Self::Gff,
        Self::Gtf,
        Self::Fasta,
        Self::Fastq,
    ];

    pub fn detect(path: &Path) -> Result<Self, String> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("cannot detect format from '{}'", path.display()))?
            .to_ascii_lowercase();
        let base = name
            .strip_suffix(".gz")
            .or_else(|| name.strip_suffix(".bgz"))
            .unwrap_or(&name);
        let extension = Path::new(base)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("");
        match extension {
            "csv" => Ok(Self::Csv),
            "tsv" | "tab" => Ok(Self::Tsv),
            "json" => Ok(Self::Json),
            "bed" => Ok(Self::Bed),
            "vcf" => Ok(Self::Vcf),
            "gff" | "gff3" => Ok(Self::Gff),
            "gtf" | "gff2" => Ok(Self::Gtf),
            "fa" | "fasta" | "fna" | "faa" | "ffn" | "frn" => Ok(Self::Fasta),
            "fq" | "fastq" => Ok(Self::Fastq),
            _ => Err(format!(
                "unknown format for '{}'; pass --from/--to explicitly",
                path.display()
            )),
        }
    }

    pub fn is_tabular(self) -> bool {
        matches!(self, Self::Csv | Self::Tsv | Self::Json)
    }

    pub fn is_sequence(self) -> bool {
        matches!(self, Self::Fasta | Self::Fastq)
    }
}

impl fmt::Display for Format {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Json => "json",
            Self::Bed => "bed",
            Self::Vcf => "vcf",
            Self::Gff => "gff",
            Self::Gtf => "gtf",
            Self::Fasta => "fasta",
            Self::Fastq => "fastq",
        };
        formatter.write_str(name)
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "tsv" | "tab" => Ok(Self::Tsv),
            "json" | "json-array" => Ok(Self::Json),
            "bed" => Ok(Self::Bed),
            "vcf" => Ok(Self::Vcf),
            "gff" | "gff3" => Ok(Self::Gff),
            "gtf" | "gff2" => Ok(Self::Gtf),
            "fasta" | "fa" | "fna" | "faa" => Ok(Self::Fasta),
            "fastq" | "fq" => Ok(Self::Fastq),
            other => Err(format!("unsupported format '{other}'")),
        }
    }
}
