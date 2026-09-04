//! WASM-safe bio file parsers using the fetch hook instead of noodles/filesystem.
//!
//! Provides `read_text`, `read_fasta`, `read_fastq`, `read_vcf`, `read_bed`,
//! `read_gff`, and `read_gtf` for WASM builds. These read the full file text
//! returned by `__blFetch.sync`; format-specific readers then parse it.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, BioSequence, Table, Value};
use std::collections::HashMap;

// ── Registration ────────────────────────────────────────────────

pub fn bio_wasm_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("read_text", Arity::Exact(1)),
        ("read_fasta", Arity::Exact(1)),
        ("read_fastq", Arity::Exact(1)),
        ("read_vcf", Arity::Exact(1)),
        ("read_bed", Arity::Exact(1)),
        ("read_gff", Arity::Exact(1)),
        ("read_gtf", Arity::Exact(1)),
    ]
}

pub fn is_bio_wasm_builtin(name: &str) -> bool {
    matches!(
        name,
        "read_text"
            | "read_fasta"
            | "read_fastq"
            | "read_vcf"
            | "read_bed"
            | "read_gff"
            | "read_gtf"
    )
}

pub fn call_bio_wasm_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "read_text" => read_text(args),
        "read_fasta" => parse_fasta(args),
        "read_fastq" => parse_fastq(args),
        "read_vcf" => parse_vcf(args),
        "read_bed" => parse_bed(args),
        "read_gff" => parse_gff(args),
        "read_gtf" => parse_gtf(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown bio_wasm builtin: {name}"),
            None,
        )),
    }
}

fn read_text(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_text")?;
    Ok(Value::Str(fetch_file_text(&path, "read_text")?))
}

// ── Helpers ─────────────────────────────────────────────────────

fn fetch_file_text(path: &str, fn_name: &str) -> std::result::Result<String, BioLangError> {
    if let Some(result) = crate::csv::try_fetch_url(path) {
        match result {
            Ok(text) if !text.starts_with("ERROR:") => Ok(text),
            Ok(err) => Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("{fn_name}: {err}"),
                None,
            )),
            Err(e) => Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("{fn_name}: {e}"),
                None,
            )),
        }
    } else {
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("{fn_name}: no fetch hook available for '{path}'"),
            None,
        ))
    }
}

fn require_str(args: &[Value], fn_name: &str) -> std::result::Result<String, BioLangError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(s.to_string()),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{fn_name} expects a string path argument"),
            None,
        )),
    }
}

// ── FASTA ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_text_reader_is_registered() {
        assert!(is_bio_wasm_builtin("read_text"));
        assert_eq!(
            bio_wasm_builtin_list()
                .into_iter()
                .find(|(name, _)| *name == "read_text")
                .map(|(_, arity)| arity),
            Some(Arity::Exact(1))
        );
    }

    #[test]
    fn browser_text_reader_validates_the_path_type() {
        let error = call_bio_wasm_builtin("read_text", vec![Value::Int(1)]).unwrap_err();
        assert_eq!(error.kind, ErrorKind::TypeError);
        assert!(error.message.contains("read_text expects a string path"));
    }
}

fn parse_fasta(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_fasta")?;
    let text = fetch_file_text(&path, "read_fasta")?;
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for entry in text.split('>').skip(1) {
        let mut lines = entry.lines();
        let header = lines.next().unwrap_or("");
        let (id, description) = match header.split_once(char::is_whitespace) {
            Some((id, desc)) => (id, desc),
            None => (header, ""),
        };
        let sequence: String = lines
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<String>()
            .to_uppercase();
        let length = sequence.len() as i64;

        rows.push(vec![
            Value::Str(id.to_string()),
            Value::Str(description.to_string()),
            Value::DNA(BioSequence { data: sequence }),
            Value::Int(length),
        ]);
    }

    Ok(Value::Table(Table::new(
        vec![
            "id".into(),
            "description".into(),
            "seq".into(),
            "length".into(),
        ],
        rows,
    )))
}

// ── FASTQ ───────────────────────────────────────────────────────

fn parse_fastq(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_fastq")?;
    let text = fetch_file_text(&path, "read_fastq")?;
    let lines: Vec<&str> = text.lines().collect();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    let mut i = 0;
    while i + 3 < lines.len() {
        let header = lines[i];
        let sequence = lines[i + 1].to_uppercase();
        // lines[i + 2] is the '+' separator
        let quality = lines[i + 3];

        let definition = header.strip_prefix('@').unwrap_or(header);
        let (id, description) = match definition.split_once(char::is_whitespace) {
            Some((id, desc)) => (id, desc.trim()),
            None => (definition, ""),
        };

        rows.push(vec![
            Value::Str(id.to_string()),
            Value::Str(description.to_string()),
            Value::DNA(BioSequence {
                data: sequence.clone(),
            }),
            Value::Int(sequence.len() as i64),
            Value::Str(quality.to_string()),
        ]);

        i += 4;
    }

    Ok(Value::Table(Table::new(
        vec![
            "id".into(),
            "description".into(),
            "seq".into(),
            "length".into(),
            "quality".into(),
        ],
        rows,
    )))
}

// ── VCF ─────────────────────────────────────────────────────────

fn parse_vcf(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_vcf")?;
    let text = fetch_file_text(&path, "read_vcf")?;
    let mut records: Vec<Value> = Vec::new();

    for line in text.lines() {
        if line.starts_with("##") || line.starts_with("#") {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 8 {
            continue;
        }

        let info = if cols[7] == "." {
            HashMap::new()
        } else {
            HashMap::from([("_raw".to_string(), Value::Str(cols[7].to_string()))])
        };
        let quality = match cols[5] {
            "." => 0.0,
            value => value.parse::<f64>().unwrap_or(0.0),
        };

        records.push(Value::Variant {
            chrom: cols[0].to_string(),
            pos: cols[1].parse::<i64>().unwrap_or(0),
            id: cols[2].to_string(),
            ref_allele: cols[3].to_string(),
            alt_allele: cols[4].to_string(),
            quality,
            filter: cols[6].to_string(),
            info,
        });
    }

    Ok(Value::List((records).into()))
}

// ── BED ─────────────────────────────────────────────────────────

fn parse_bed(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_bed")?;
    let text = fetch_file_text(&path, "read_bed")?;
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut num_fields = 0usize;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("track")
            || line.starts_with("browser")
        {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 3 {
            continue;
        }

        let field_count = cols.len().min(6);
        num_fields = num_fields.max(field_count);
        let mut row = vec![
            Value::Str(cols[0].to_string()),
            Value::Int(cols[1].parse::<i64>().unwrap_or(0)),
            Value::Int(cols[2].parse::<i64>().unwrap_or(0)),
        ];
        if cols.len() > 3 {
            row.push(Value::Str(cols[3].to_string()));
        }
        if cols.len() > 4 {
            row.push(match cols[4].parse::<i64>() {
                Ok(value) => Value::Int(value),
                Err(_) => Value::Float(cols[4].parse::<f64>().unwrap_or(0.0)),
            });
        }
        if cols.len() > 5 {
            row.push(Value::Str(cols[5].to_string()));
        }

        rows.push(row);
    }

    let columns = match num_fields {
        0..=3 => vec!["chrom", "start", "end"],
        4 => vec!["chrom", "start", "end", "name"],
        5 => vec!["chrom", "start", "end", "name", "score"],
        _ => vec!["chrom", "start", "end", "name", "score", "strand"],
    }
    .into_iter()
    .map(str::to_string)
    .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── GFF ─────────────────────────────────────────────────────────

fn parse_gff(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_gff")?;
    let text = fetch_file_text(&path, "read_gff")?;
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 9 {
            continue;
        }

        rows.push(vec![
            Value::Str(cols[0].to_string()),
            Value::Str(cols[1].to_string()),
            Value::Str(cols[2].to_string()),
            Value::Int(cols[3].parse::<i64>().unwrap_or(0)),
            Value::Int(cols[4].parse::<i64>().unwrap_or(0)),
            match cols[5] {
                "." => Value::Nil,
                value => Value::Float(value.parse::<f64>().unwrap_or(0.0)),
            },
            Value::Str(cols[6].to_string()),
            Value::Str(cols[7].to_string()),
            Value::Str(cols[8].to_string()),
        ]);
    }

    Ok(Value::Table(Table::new(
        vec![
            "seqid".into(),
            "source".into(),
            "type".into(),
            "start".into(),
            "end".into(),
            "score".into(),
            "strand".into(),
            "phase".into(),
            "attributes".into(),
        ],
        rows,
    )))
}

// ── GTF ─────────────────────────────────────────────────────────

fn parse_gtf(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_gtf")?;
    let text = fetch_file_text(&path, "read_gtf")?;
    let mut records: Vec<Value> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 9 {
            continue;
        }

        // Parse GTF attributes (col 8): key "value"; pairs
        // Format: gene_id "BRCA1"; transcript_id "NM_007294";
        let mut attrs: HashMap<String, Value> = HashMap::new();
        for item in cols[8].split(';') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            // Split on first whitespace: key "value"
            if let Some((key, value)) = item.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                attrs.insert(key.to_string(), Value::Str(value.to_string()));
            } else {
                attrs.insert(item.to_string(), Value::Bool(true));
            }
        }

        records.push(Value::Record(
            HashMap::from([
                ("seqid".to_string(), Value::Str(cols[0].to_string())),
                ("source".to_string(), Value::Str(cols[1].to_string())),
                ("type".to_string(), Value::Str(cols[2].to_string())),
                (
                    "start".to_string(),
                    Value::Int(cols[3].parse::<i64>().unwrap_or(0)),
                ),
                (
                    "end".to_string(),
                    Value::Int(cols[4].parse::<i64>().unwrap_or(0)),
                ),
                ("score".to_string(), Value::Str(cols[5].to_string())),
                ("strand".to_string(), Value::Str(cols[6].to_string())),
                ("phase".to_string(), Value::Str(cols[7].to_string())),
                ("attributes".to_string(), Value::Record(attrs.into())),
            ])
            .into(),
        ));
    }

    Ok(Value::List((records).into()))
}
