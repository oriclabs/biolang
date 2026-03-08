//! Pure GFF3/GTF record types and parsing algorithms. No framework dependencies.

use serde::Serialize;
use std::collections::HashMap;

// ── Types ───────────────────────────────────────────────────────────────────

/// A single GFF3/GTF annotation record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GffRecord {
    pub seqid: String,
    pub source: String,
    pub feature_type: String,
    pub start: u64,
    pub end: u64,
    pub score: Option<f64>,
    pub strand: char,
    pub phase: Option<u8>,
    pub attributes: HashMap<String, String>,
}

/// GFF file format variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GffFormat {
    Gff3,
    Gtf,
}

// ── Format detection ────────────────────────────────────────────────────────

/// Detect whether an attributes column string is GFF3 or GTF format.
///
/// GFF3 uses `key=value;` while GTF uses `key "value";`.
pub fn detect_format(first_attr_line: &str) -> GffFormat {
    // GFF3 uses key=value; GTF uses key "value"
    // If the line has '=' and no unquoted space-separated values, it's GFF3
    if first_attr_line.contains('=') {
        GffFormat::Gff3
    } else {
        GffFormat::Gtf
    }
}

// ── URL encoding ────────────────────────────────────────────────────────────

/// Decode percent-encoded strings (GFF3 values may be URL-encoded).
pub fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

// ── Attribute parsing ───────────────────────────────────────────────────────

/// Parse GFF3 attribute column (`key=value;key2=value2`).
pub fn parse_gff3_attributes(s: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((key, val)) = part.split_once('=') {
            attrs.insert(key.trim().to_string(), url_decode(val.trim()));
        }
    }
    attrs
}

/// Parse GTF attribute column (`key "value"; key2 "value2"`).
pub fn parse_gtf_attributes(s: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(space_idx) = part.find([' ', '\t']) {
            let key = part[..space_idx].trim();
            let val = part[space_idx..].trim().trim_matches('"');
            if !key.is_empty() {
                attrs.insert(key.to_string(), val.to_string());
            }
        }
    }
    attrs
}

// ── Record parsing ──────────────────────────────────────────────────────────

/// Parse a single tab-delimited GFF/GTF line into a `GffRecord`.
///
/// The `fmt` parameter controls attribute parsing style.
/// Returns `Err` if the line has fewer than 9 columns or has invalid coordinates.
pub fn parse_gff_line(line: &str, fmt: GffFormat) -> Result<GffRecord, String> {
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 9 {
        return Err(format!(
            "GFF line has {} columns, expected >= 9",
            cols.len()
        ));
    }
    let start = cols[3]
        .parse::<u64>()
        .map_err(|_| format!("invalid start: {}", cols[3]))?;
    let end = cols[4]
        .parse::<u64>()
        .map_err(|_| format!("invalid end: {}", cols[4]))?;
    let score = if cols[5] == "." {
        None
    } else {
        cols[5].parse::<f64>().ok()
    };
    let strand = cols[6].chars().next().unwrap_or('.');
    let phase = if cols[7] == "." {
        None
    } else {
        cols[7].parse::<u8>().ok()
    };
    let attributes = match fmt {
        GffFormat::Gff3 => parse_gff3_attributes(cols[8]),
        GffFormat::Gtf => parse_gtf_attributes(cols[8]),
    };

    Ok(GffRecord {
        seqid: cols[0].to_string(),
        source: cols[1].to_string(),
        feature_type: cols[2].to_string(),
        start,
        end,
        score,
        strand,
        phase,
        attributes,
    })
}

// ── Record formatting ───────────────────────────────────────────────────────

/// Format a `GffRecord` back into a GFF3 or GTF tab-delimited line.
pub fn format_gff_line(rec: &GffRecord, fmt: GffFormat) -> String {
    let score_str = rec.score.map_or(".".to_string(), |s| format!("{s}"));
    let phase_str = rec.phase.map_or(".".to_string(), |p| format!("{p}"));
    let attrs = match fmt {
        GffFormat::Gff3 => rec
            .attributes
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(";"),
        GffFormat::Gtf => rec
            .attributes
            .iter()
            .map(|(k, v)| format!("{k} \"{v}\""))
            .collect::<Vec<_>>()
            .join("; "),
    };
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        rec.seqid, rec.source, rec.feature_type, rec.start, rec.end, score_str, rec.strand,
        phase_str, attrs
    )
}

// ── Region parsing ──────────────────────────────────────────────────────────

/// Parse a genomic region string `chr:start-end` into `(chrom, start, end)`.
///
/// Commas in numeric fields are stripped (e.g. `chr1:1,000-2,000`).
pub fn parse_region(region: &str) -> Result<(String, u64, u64), String> {
    let parts: Vec<&str> = region.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "invalid region format: {region} (expected chr:start-end)"
        ));
    }
    let chrom = parts[0].to_string();
    let range_parts: Vec<&str> = parts[1].split('-').collect();
    if range_parts.len() != 2 {
        return Err(format!("invalid region range: {region}"));
    }
    let start = range_parts[0]
        .replace(',', "")
        .parse::<u64>()
        .map_err(|_| format!("invalid start in region: {region}"))?;
    let end = range_parts[1]
        .replace(',', "")
        .parse::<u64>()
        .map_err(|_| format!("invalid end in region: {region}"))?;
    Ok((chrom, start, end))
}
