use crate::streams::open_reader;
use crate::{context, ConversionStats, ConvertError, ConvertOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub fn vcf_to_bed(
    input: &Path,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let reader = BufReader::with_capacity(256 * 1024, open_reader(input)?);
    let mut saw_header = false;
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| context(error, "cannot read VCF"))?;
        if line.starts_with("#CHROM\t") {
            saw_header = true;
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !saw_header {
            return Err(ConvertError::new(format!(
                "VCF record at line {} appears before the #CHROM header",
                line_index + 1
            )));
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 8 {
            return Err(ConvertError::new(format!(
                "VCF line {} has {} fields; expected at least 8",
                line_index + 1,
                fields.len()
            )));
        }
        if fields[0].is_empty() || fields[0] == "." {
            return Err(ConvertError::new(format!(
                "VCF line {} has an empty CHROM",
                line_index + 1
            )));
        }
        if fields[3].is_empty() || fields[3] == "." {
            return Err(ConvertError::new(format!(
                "VCF line {} has an invalid REF allele",
                line_index + 1
            )));
        }
        let position = fields[1].parse::<u64>().map_err(|_| {
            ConvertError::new(format!("VCF line {} has invalid POS", line_index + 1))
        })?;
        if position == 0 {
            return Err(ConvertError::new(format!(
                "VCF line {} has POS 0; VCF coordinates are 1-based",
                line_index + 1
            )));
        }
        let start = position - 1;
        let reference_length = fields[3].len().max(1) as u64;
        let info_end = fields[7]
            .split(';')
            .find_map(|entry| entry.strip_prefix("END="))
            .map(|value| {
                let end = value.parse::<u64>().map_err(|_| {
                    ConvertError::new(format!("VCF line {} has invalid INFO/END", line_index + 1))
                })?;
                if end < position {
                    return Err(ConvertError::new(format!(
                        "VCF line {} has INFO/END before POS",
                        line_index + 1
                    )));
                }
                Ok(end)
            })
            .transpose()?;
        let end = info_end.unwrap_or(start + reference_length);
        let name = if fields[2] == "." || fields[2].is_empty() {
            format!("{}>{}", fields[3], fields[4])
        } else {
            fields[2].to_string()
        };
        writeln!(writer, "{}\t{}\t{}\t{}\t0\t.", fields[0], start, end, name)?;
        stats.records_read += 1;
        stats.records_written += 1;
    }
    if !saw_header {
        return Err(ConvertError::new("VCF is missing its #CHROM header"));
    }
    stats.lossy = true;
    stats.warnings.push(
        "VCF -> BED retains genomic span and ID/alleles. BED score is set to 0 because VCF QUAL is not the 0-1000 BED score; INFO, FILTER, FORMAT and sample genotypes are not represented."
            .into(),
    );
    stats.warnings.push(
        "VCF POS was converted from 1-based coordinates to BED's 0-based half-open interval; INFO/END was used when present."
            .into(),
    );
    Ok(())
}

pub fn gff_to_bed(
    input: &Path,
    writer: &mut dyn Write,
    options: &ConvertOptions,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let reader = BufReader::with_capacity(256 * 1024, open_reader(input)?);
    let mut normalized_unknown_strand = false;
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| context(error, "cannot read GFF/GTF"))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 {
            return Err(ConvertError::new(format!(
                "GFF/GTF line {} has {} fields; expected exactly 9",
                line_index + 1,
                fields.len()
            )));
        }
        stats.records_read += 1;
        if options
            .feature
            .as_deref()
            .is_some_and(|feature| fields[2] != feature)
        {
            stats.records_skipped += 1;
            continue;
        }
        let start = fields[3].parse::<u64>().map_err(|_| {
            ConvertError::new(format!("GFF/GTF line {} has invalid start", line_index + 1))
        })?;
        let end = fields[4].parse::<u64>().map_err(|_| {
            ConvertError::new(format!("GFF/GTF line {} has invalid end", line_index + 1))
        })?;
        if start == 0 || end < start {
            return Err(ConvertError::new(format!(
                "GFF/GTF line {} has invalid 1-based inclusive interval {}-{}",
                line_index + 1,
                start,
                end
            )));
        }
        let attributes = parse_attributes(fields[8]);
        let name = options
            .name_attribute
            .as_deref()
            .and_then(|key| find_attribute(&attributes, key))
            .or_else(|| find_attribute(&attributes, "Name"))
            .or_else(|| find_attribute(&attributes, "gene_name"))
            .or_else(|| find_attribute(&attributes, "gene_id"))
            .or_else(|| find_attribute(&attributes, "ID"))
            .unwrap_or(fields[2]);
        let strand = match fields[6] {
            "+" | "-" | "." => fields[6],
            "?" => {
                normalized_unknown_strand = true;
                "."
            }
            _ => {
                return Err(ConvertError::new(format!(
                    "GFF/GTF line {} has invalid strand '{}'",
                    line_index + 1,
                    fields[6]
                )))
            }
        };
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}",
            fields[0],
            start - 1,
            end,
            name,
            0,
            strand
        )?;
        stats.records_written += 1;
    }
    stats.lossy = true;
    stats.warnings.push(
        "GFF/GTF -> BED6 retains interval, a selected name and strand. BED score is set to 0 because GFF score has different semantics; source, feature type, phase and remaining attributes are not represented."
            .into(),
    );
    stats.warnings.push(
        "GFF/GTF 1-based inclusive starts were converted to BED 0-based half-open starts.".into(),
    );
    if normalized_unknown_strand {
        stats
            .warnings
            .push("GFF/GTF unknown strand '?' was normalized to BED strand '.'.".into());
    }
    Ok(())
}

fn parse_attributes(value: &str) -> Vec<(String, String)> {
    value
        .split(';')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            if let Some((key, value)) = entry.split_once('=') {
                return Some((key.trim().into(), value.trim().trim_matches('"').into()));
            }
            let mut parts = entry.splitn(2, char::is_whitespace);
            let key = parts.next()?.trim();
            let value = parts.next()?.trim().trim_matches('"');
            Some((key.into(), value.into()))
        })
        .collect()
}

fn find_attribute<'a>(attributes: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// A `track` or `browser` line is the keyword followed by space-separated
/// key=value pairs; a BED data line separates its columns with tabs.
///
/// Matching the bare prefix instead let any interval whose contig merely began
/// with those letters through untouched: `track1<TAB>BOGUS<TAB>NOTANUMBER` was
/// copied to the output verbatim, counted as no record at all, and never
/// coordinate-checked — in the one converter that exists to coordinate-check.
/// Assembly scaffolds carry arbitrary names, so this is reachable with real
/// data. Requiring a following space keeps genuine header lines working and
/// puts a contig named `track` (tab next) back on the validated path.
fn is_bed_header(trimmed: &str) -> bool {
    if trimmed.starts_with('#') {
        return true;
    }
    ["track", "browser"].iter().any(|keyword| {
        trimmed
            .strip_prefix(keyword)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}

pub fn normalize_bed(
    input: &Path,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let reader = BufReader::with_capacity(256 * 1024, open_reader(input)?);
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| context(error, "cannot read BED"))?;
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        if is_bed_header(trimmed) {
            writeln!(writer, "{trimmed}")?;
            continue;
        }
        let fields = trimmed.split('\t').collect::<Vec<_>>();
        if fields.len() < 3 {
            return Err(ConvertError::new(format!(
                "BED line {} has {} fields; expected at least 3",
                line_index + 1,
                fields.len()
            )));
        }
        let start = fields[1].parse::<u64>().map_err(|_| {
            ConvertError::new(format!("BED line {} has invalid start", line_index + 1))
        })?;
        let end = fields[2].parse::<u64>().map_err(|_| {
            ConvertError::new(format!("BED line {} has invalid end", line_index + 1))
        })?;
        if end < start {
            return Err(ConvertError::new(format!(
                "BED line {} ends before it starts",
                line_index + 1
            )));
        }
        writeln!(writer, "{}", fields.join("\t"))?;
        stats.records_read += 1;
        stats.records_written += 1;
    }
    Ok(())
}
