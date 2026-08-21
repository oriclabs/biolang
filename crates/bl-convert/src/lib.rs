pub mod containers;
mod format;
mod genomic;
mod sequence;
mod streams;
mod tabular;

pub use format::Format;

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
pub struct ConvertError(String);

impl ConvertError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ConvertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ConvertError {}

impl From<std::io::Error> for ConvertError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<csv::Error> for ConvertError {
    fn from(error: csv::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<serde_json::Error> for ConvertError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub from: Option<Format>,
    pub to: Option<Format>,
    pub force: bool,
    pub dry_run: bool,
    pub feature: Option<String>,
    pub name_attribute: Option<String>,
    pub line_width: usize,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            force: false,
            dry_run: false,
            feature: None,
            name_attribute: None,
            line_width: 80,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConversionStats {
    pub records_read: u64,
    pub records_written: u64,
    pub records_skipped: u64,
    pub lossy: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionReport {
    pub schema: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub from: Format,
    pub to: Format,
    pub dry_run: bool,
    pub input_bytes: u64,
    pub output_bytes: Option<u64>,
    pub records_read: u64,
    pub records_written: u64,
    pub records_skipped: u64,
    pub lossy: bool,
    pub warnings: Vec<String>,
    pub elapsed_ms: u128,
    pub output_validated: bool,
}

pub fn supported_pairs() -> Vec<(Format, Format, bool)> {
    vec![
        (Format::Csv, Format::Csv, false),
        (Format::Csv, Format::Tsv, false),
        (Format::Csv, Format::Json, false),
        (Format::Tsv, Format::Csv, false),
        (Format::Tsv, Format::Tsv, false),
        (Format::Tsv, Format::Json, false),
        (Format::Json, Format::Csv, true),
        (Format::Json, Format::Tsv, true),
        (Format::Json, Format::Json, false),
        (Format::Bed, Format::Bed, false),
        (Format::Vcf, Format::Bed, true),
        (Format::Gff, Format::Bed, true),
        (Format::Gtf, Format::Bed, true),
        (Format::Fasta, Format::Fasta, false),
        (Format::Fastq, Format::Fastq, false),
        (Format::Fastq, Format::Fasta, true),
    ]
}

pub fn convert(
    input: &Path,
    output: &Path,
    options: &ConvertOptions,
) -> Result<ConversionReport, ConvertError> {
    if !input.is_file() {
        return Err(ConvertError::new(format!(
            "input file '{}' does not exist",
            input.display()
        )));
    }
    let input_canonical = input.canonicalize()?;
    if output.exists() {
        if output.is_dir() {
            return Err(ConvertError::new(format!(
                "output '{}' is a directory",
                output.display()
            )));
        }
        if !options.force && !options.dry_run {
            return Err(ConvertError::new(format!(
                "output '{}' already exists; pass --force to replace it",
                output.display()
            )));
        }
        if output.canonicalize().ok().as_ref() == Some(&input_canonical) {
            return Err(ConvertError::new(
                "input and output resolve to the same file; choose a different output path",
            ));
        }
    }

    let from = options
        .from
        .map(Ok)
        .unwrap_or_else(|| Format::detect(input))
        .map_err(ConvertError::new)?;
    let to = options
        .to
        .map(Ok)
        .unwrap_or_else(|| Format::detect(output))
        .map_err(ConvertError::new)?;
    if !supported_pairs()
        .iter()
        .any(|(source, target, _)| *source == from && *target == to)
    {
        return Err(ConvertError::new(format!(
            "conversion {from} -> {to} is not supported; run 'bl-convert formats'",
        )));
    }
    if options.line_width == 0 || options.line_width > 1_000_000 {
        return Err(ConvertError::new(
            "--line-width must be between 1 and 1000000",
        ));
    }
    if output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bgz"))
    {
        return Err(ConvertError::new(
            "BGZF output is not implemented; use .gz for ordinary gzip output rather than writing misleading .bgz data",
        ));
    }

    let started = Instant::now();
    let input_bytes = fs::metadata(input)?.len();
    let mut stats = ConversionStats::default();

    if options.dry_run {
        let mut sink = std::io::sink();
        dispatch(input, from, to, &mut sink, options, &mut stats)?;
    } else {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let mut builder = tempfile::Builder::new();
        builder.prefix(".bl-convert-");
        if output
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
        {
            builder.suffix(".gz");
        }
        let temporary = builder.tempfile_in(parent)?;
        let temporary_path = temporary.into_temp_path();
        {
            let mut writer = streams::open_writer(&temporary_path)?;
            dispatch(input, from, to, &mut writer, options, &mut stats)?;
            writer.flush()?;
        }
        let validated_records = validate_file(&temporary_path, to)?;
        if validated_records != stats.records_written {
            return Err(ConvertError::new(format!(
                "output validation counted {validated_records} records but conversion wrote {}",
                stats.records_written
            )));
        }
        temporary_path.persist(output).map_err(|error| {
            ConvertError::new(format!(
                "cannot persist '{}': {}",
                output.display(),
                error.error
            ))
        })?;
    }

    let output_bytes = if options.dry_run {
        None
    } else {
        Some(fs::metadata(output)?.len())
    };
    Ok(ConversionReport {
        schema: "bl-convert.report/v1".into(),
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        from,
        to,
        dry_run: options.dry_run,
        input_bytes,
        output_bytes,
        records_read: stats.records_read,
        records_written: stats.records_written,
        records_skipped: stats.records_skipped,
        lossy: stats.lossy,
        warnings: stats.warnings,
        elapsed_ms: started.elapsed().as_millis(),
        output_validated: !options.dry_run && output.is_file(),
    })
}

pub fn validate_file(path: &Path, format: Format) -> Result<u64, ConvertError> {
    match format {
        Format::Csv | Format::Tsv => tabular::validate(path, format),
        Format::Json => tabular::validate(path, format),
        Format::Bed => {
            let mut sink = std::io::sink();
            let mut stats = ConversionStats::default();
            genomic::normalize_bed(path, &mut sink, &mut stats)?;
            Ok(stats.records_read)
        }
        Format::Fasta => {
            let mut sink = std::io::sink();
            let mut stats = ConversionStats::default();
            sequence::normalize_fasta(path, &mut sink, 80, &mut stats)?;
            Ok(stats.records_read)
        }
        Format::Fastq => {
            let mut sink = std::io::sink();
            let mut stats = ConversionStats::default();
            sequence::normalize_fastq(path, &mut sink, &mut stats)?;
            Ok(stats.records_read)
        }
        Format::Vcf => {
            let mut sink = std::io::sink();
            let mut stats = ConversionStats::default();
            genomic::vcf_to_bed(path, &mut sink, &mut stats)?;
            Ok(stats.records_read)
        }
        Format::Gff | Format::Gtf => {
            let mut sink = std::io::sink();
            let mut stats = ConversionStats::default();
            genomic::gff_to_bed(path, &mut sink, &ConvertOptions::default(), &mut stats)?;
            Ok(stats.records_read)
        }
    }
}

fn dispatch(
    input: &Path,
    from: Format,
    to: Format,
    writer: &mut dyn Write,
    options: &ConvertOptions,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    if from.is_tabular() && to.is_tabular() {
        return tabular::convert(input, from, to, writer, stats);
    }
    match (from, to) {
        (Format::Vcf, Format::Bed) => genomic::vcf_to_bed(input, writer, stats),
        (Format::Gff | Format::Gtf, Format::Bed) => {
            genomic::gff_to_bed(input, writer, options, stats)
        }
        (Format::Bed, Format::Bed) => genomic::normalize_bed(input, writer, stats),
        (Format::Fasta, Format::Fasta) => {
            sequence::normalize_fasta(input, writer, options.line_width, stats)
        }
        (Format::Fastq, Format::Fastq) => sequence::normalize_fastq(input, writer, stats),
        (Format::Fastq, Format::Fasta) => {
            sequence::fastq_to_fasta(input, writer, options.line_width, stats)
        }
        _ => Err(ConvertError::new(format!(
            "conversion {from} -> {to} is not implemented"
        ))),
    }
}

pub(crate) fn context(error: impl fmt::Display, context: impl fmt::Display) -> ConvertError {
    ConvertError::new(format!("{context}: {error}"))
}
