//! WASM-safe bio file parsers using the fetch hook instead of noodles/filesystem.
//!
//! Provides `read_fasta`, `read_fastq`, `read_vcf`, `read_bed`, `read_gff` for
//! WASM builds. These parse the full file text returned by `__blFetch.sync`.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

// ── Registration ────────────────────────────────────────────────

pub fn bio_wasm_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
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
        "read_fasta" | "read_fastq" | "read_vcf" | "read_bed" | "read_gff" | "read_gtf"
    )
}

pub fn call_bio_wasm_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
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

fn parse_fasta(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_fasta")?;
    let text = fetch_file_text(&path, "read_fasta")?;
    let mut records: Vec<Value> = Vec::new();

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
            .collect();
        let length = sequence.len() as i64;

        records.push(Value::Record(HashMap::from([
            ("id".to_string(), Value::Str(id.to_string())),
            ("description".to_string(), Value::Str(description.to_string())),
            ("sequence".to_string(), Value::Str(sequence)),
            ("length".to_string(), Value::Int(length)),
        ])));
    }

    Ok(Value::List(records))
}

// ── FASTQ ───────────────────────────────────────────────────────

fn parse_fastq(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_fastq")?;
    let text = fetch_file_text(&path, "read_fastq")?;
    let lines: Vec<&str> = text.lines().collect();
    let mut records: Vec<Value> = Vec::new();

    let mut i = 0;
    while i + 3 < lines.len() {
        let header = lines[i];
        let sequence = lines[i + 1];
        // lines[i + 2] is the '+' separator
        let quality = lines[i + 3];

        let id = header.strip_prefix('@').unwrap_or(header);
        let id = id.split_whitespace().next().unwrap_or(id);

        records.push(Value::Record(HashMap::from([
            ("id".to_string(), Value::Str(id.to_string())),
            ("sequence".to_string(), Value::Str(sequence.to_string())),
            ("quality".to_string(), Value::Str(quality.to_string())),
            ("length".to_string(), Value::Int(sequence.len() as i64)),
        ])));

        i += 4;
    }

    Ok(Value::List(records))
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

        // Parse INFO field into sub-record
        let mut info_map: HashMap<String, Value> = HashMap::new();
        for item in cols[7].split(';') {
            if let Some((k, v)) = item.split_once('=') {
                info_map.insert(k.to_string(), Value::Str(v.to_string()));
            } else {
                // Flag field (no value)
                info_map.insert(item.to_string(), Value::Bool(true));
            }
        }

        records.push(Value::Record(HashMap::from([
            ("CHROM".to_string(), Value::Str(cols[0].to_string())),
            ("POS".to_string(), Value::Int(cols[1].parse::<i64>().unwrap_or(0))),
            ("ID".to_string(), Value::Str(cols[2].to_string())),
            ("REF".to_string(), Value::Str(cols[3].to_string())),
            ("ALT".to_string(), Value::Str(cols[4].to_string())),
            ("QUAL".to_string(), Value::Float(cols[5].parse::<f64>().unwrap_or(0.0))),
            ("FILTER".to_string(), Value::Str(cols[6].to_string())),
            ("INFO".to_string(), Value::Record(info_map)),
        ])));
    }

    Ok(Value::List(records))
}

// ── BED ─────────────────────────────────────────────────────────

fn parse_bed(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_bed")?;
    let text = fetch_file_text(&path, "read_bed")?;
    let mut records: Vec<Value> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("track") || line.starts_with("browser") {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 3 {
            continue;
        }

        let mut rec: HashMap<String, Value> = HashMap::new();
        rec.insert("chrom".to_string(), Value::Str(cols[0].to_string()));
        rec.insert("start".to_string(), Value::Int(cols[1].parse::<i64>().unwrap_or(0)));
        rec.insert("end".to_string(), Value::Int(cols[2].parse::<i64>().unwrap_or(0)));

        if cols.len() > 3 {
            rec.insert("name".to_string(), Value::Str(cols[3].to_string()));
        }
        if cols.len() > 4 {
            rec.insert("score".to_string(), Value::Float(cols[4].parse::<f64>().unwrap_or(0.0)));
        }
        if cols.len() > 5 {
            rec.insert("strand".to_string(), Value::Str(cols[5].to_string()));
        }

        records.push(Value::Record(rec));
    }

    Ok(Value::List(records))
}

// ── GFF ─────────────────────────────────────────────────────────

fn parse_gff(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args, "read_gff")?;
    let text = fetch_file_text(&path, "read_gff")?;
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

        // Parse attributes (col 8): key=value pairs separated by ;
        let mut attrs: HashMap<String, Value> = HashMap::new();
        for item in cols[8].split(';') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some((k, v)) = item.split_once('=') {
                attrs.insert(k.to_string(), Value::Str(v.to_string()));
            } else {
                attrs.insert(item.to_string(), Value::Bool(true));
            }
        }

        records.push(Value::Record(HashMap::from([
            ("seqid".to_string(), Value::Str(cols[0].to_string())),
            ("source".to_string(), Value::Str(cols[1].to_string())),
            ("type".to_string(), Value::Str(cols[2].to_string())),
            ("start".to_string(), Value::Int(cols[3].parse::<i64>().unwrap_or(0))),
            ("end".to_string(), Value::Int(cols[4].parse::<i64>().unwrap_or(0))),
            ("score".to_string(), Value::Str(cols[5].to_string())),
            ("strand".to_string(), Value::Str(cols[6].to_string())),
            ("phase".to_string(), Value::Str(cols[7].to_string())),
            ("attributes".to_string(), Value::Record(attrs)),
        ])));
    }

    Ok(Value::List(records))
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

        records.push(Value::Record(HashMap::from([
            ("seqid".to_string(), Value::Str(cols[0].to_string())),
            ("source".to_string(), Value::Str(cols[1].to_string())),
            ("type".to_string(), Value::Str(cols[2].to_string())),
            ("start".to_string(), Value::Int(cols[3].parse::<i64>().unwrap_or(0))),
            ("end".to_string(), Value::Int(cols[4].parse::<i64>().unwrap_or(0))),
            ("score".to_string(), Value::Str(cols[5].to_string())),
            ("strand".to_string(), Value::Str(cols[6].to_string())),
            ("phase".to_string(), Value::Str(cols[7].to_string())),
            ("attributes".to_string(), Value::Record(attrs)),
        ])));
    }

    Ok(Value::List(records))
}
