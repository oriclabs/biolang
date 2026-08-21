use crate::streams::open_reader;
use crate::{context, ConversionStats, ConvertError};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn write_wrapped(writer: &mut dyn Write, sequence: &str, width: usize) -> Result<(), ConvertError> {
    for chunk in sequence.as_bytes().chunks(width) {
        writer.write_all(chunk)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

fn validate_sequence(sequence: &str, format: &str, record: &str) -> Result<(), ConvertError> {
    if sequence.bytes().any(|symbol| !symbol.is_ascii_graphic()) {
        return Err(ConvertError::new(format!(
            "{format} record '{record}' contains a non-ASCII or whitespace sequence symbol"
        )));
    }
    Ok(())
}

pub fn normalize_fasta(
    input: &Path,
    writer: &mut dyn Write,
    width: usize,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let reader = BufReader::with_capacity(256 * 1024, open_reader(input)?);
    let mut header: Option<String> = None;
    let mut sequence = String::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| context(error, "cannot read FASTA"))?;
        let line = line.trim();
        if let Some(next_header) = line.strip_prefix('>') {
            if next_header.trim().is_empty() {
                return Err(ConvertError::new(format!(
                    "FASTA line {} has an empty header",
                    line_index + 1
                )));
            }
            if let Some(previous) = header.replace(next_header.trim().into()) {
                if sequence.is_empty() {
                    return Err(ConvertError::new(format!(
                        "FASTA record '{previous}' has no sequence"
                    )));
                }
                validate_sequence(&sequence, "FASTA", &previous)?;
                writeln!(writer, ">{previous}")?;
                write_wrapped(writer, &sequence, width)?;
                stats.records_read += 1;
                stats.records_written += 1;
                sequence.clear();
            }
        } else if !line.is_empty() {
            if header.is_none() {
                return Err(ConvertError::new(format!(
                    "FASTA sequence at line {} appears before a header",
                    line_index + 1
                )));
            }
            sequence.extend(line.chars().filter(|character| !character.is_whitespace()));
        }
    }
    let header = header.ok_or_else(|| ConvertError::new("FASTA contains no records"))?;
    if sequence.is_empty() {
        return Err(ConvertError::new(format!(
            "FASTA record '{header}' has no sequence"
        )));
    }
    validate_sequence(&sequence, "FASTA", &header)?;
    writeln!(writer, ">{header}")?;
    write_wrapped(writer, &sequence, width)?;
    stats.records_read += 1;
    stats.records_written += 1;
    Ok(())
}

struct FastqRecord {
    header: String,
    sequence: String,
    /// Whatever followed `+`, kept so normalization can put it back.
    ///
    /// The line is optional in FASTQ and usually empty, but when a producer
    /// repeats the identifier there, discarding it is a real loss — and this
    /// converter advertises FASTQ -> FASTQ as lossless in `bl-convert formats`
    /// and reports `"lossy": false`. Preserving it makes that claim true.
    plus_description: String,
    quality: String,
}

fn read_fastq_records(
    input: &Path,
    mut consume: impl FnMut(FastqRecord) -> Result<(), ConvertError>,
) -> Result<u64, ConvertError> {
    let mut reader = BufReader::with_capacity(256 * 1024, open_reader(input)?);
    let mut line = String::new();
    let mut line_number = 0usize;
    let mut count = 0u64;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        line_number += 1;
        let header_line = line.trim_end_matches(['\r', '\n']);
        if header_line.is_empty() {
            continue;
        }
        let header = header_line
            .strip_prefix('@')
            .ok_or_else(|| {
                ConvertError::new(format!("FASTQ line {line_number} must start with '@'"))
            })?
            .to_string();
        if header.is_empty() {
            return Err(ConvertError::new(format!(
                "FASTQ line {line_number} has an empty header"
            )));
        }
        let mut sequence = String::new();
        // Assigned on the only path that leaves this loop without returning.
        let plus_description;
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Err(ConvertError::new(format!(
                    "FASTQ record '{header}' ended before its '+' line"
                )));
            }
            line_number += 1;
            let content = line.trim_end_matches(['\r', '\n']);
            if let Some(description) = content.strip_prefix('+') {
                plus_description = description.to_string();
                break;
            }
            sequence.push_str(content);
        }
        if sequence.is_empty() {
            return Err(ConvertError::new(format!(
                "FASTQ record '{header}' has no sequence"
            )));
        }
        validate_sequence(&sequence, "FASTQ", &header)?;
        let mut quality = String::new();
        while quality.len() < sequence.len() {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Err(ConvertError::new(format!(
                    "FASTQ record '{header}' ended before all quality scores were read"
                )));
            }
            line_number += 1;
            quality.push_str(line.trim_end_matches(['\r', '\n']));
        }
        if quality.len() != sequence.len() {
            return Err(ConvertError::new(format!(
                "FASTQ record '{header}' has {} sequence symbols but {} quality symbols",
                sequence.len(),
                quality.len()
            )));
        }
        if quality.bytes().any(|score| !(33..=126).contains(&score)) {
            return Err(ConvertError::new(format!(
                "FASTQ record '{header}' has a quality symbol outside printable Phred+33 ASCII (! through ~)"
            )));
        }
        consume(FastqRecord {
            header,
            sequence,
            plus_description,
            quality,
        })?;
        count += 1;
    }
    if count == 0 {
        return Err(ConvertError::new("FASTQ contains no records"));
    }
    Ok(count)
}

pub fn normalize_fastq(
    input: &Path,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let count = read_fastq_records(input, |record| {
        writeln!(writer, "@{}", record.header)?;
        writeln!(writer, "{}", record.sequence)?;
        writeln!(writer, "+{}", record.plus_description)?;
        writeln!(writer, "{}", record.quality)?;
        Ok(())
    })?;
    stats.records_read = count;
    stats.records_written = count;
    Ok(())
}

pub fn fastq_to_fasta(
    input: &Path,
    writer: &mut dyn Write,
    width: usize,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let count = read_fastq_records(input, |record| {
        writeln!(writer, ">{}", record.header)?;
        write_wrapped(writer, &record.sequence, width)
    })?;
    stats.records_read = count;
    stats.records_written = count;
    stats.lossy = true;
    stats
        .warnings
        .push("FASTQ -> FASTA discards every per-base quality score.".into());
    Ok(())
}
