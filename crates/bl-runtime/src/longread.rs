//! Long-read sequencing QC builtins (Nanopore / PacBio).
//!
//! Functions: fastq_stats, n50, read_length_hist, quality_filter,
//!            gc_per_read, read_quality_dist.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ──────────────────────────────────────────────────────────

pub fn longread_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("fastq_stats", Arity::Exact(1)),
        ("n50", Arity::Exact(1)),
        ("read_length_hist", Arity::Range(1, 2)),
        ("quality_filter", Arity::Exact(3)),
        ("gc_per_read", Arity::Exact(1)),
        ("read_quality_dist", Arity::Exact(1)),
    ]
}

pub fn is_longread_builtin(name: &str) -> bool {
    matches!(
        name,
        "fastq_stats"
            | "n50"
            | "read_length_hist"
            | "quality_filter"
            | "gc_per_read"
            | "read_quality_dist"
    )
}

pub fn call_longread_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "fastq_stats" => builtin_fastq_stats(args),
        "n50" => builtin_n50(args),
        "read_length_hist" => builtin_read_length_hist(args),
        "quality_filter" => builtin_quality_filter(args),
        "gc_per_read" => builtin_gc_per_read(args),
        "read_quality_dist" => builtin_read_quality_dist(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown longread builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Str"),
            None,
        )),
    }
}

/// Parse FASTQ text into Vec<(sequence, quality_string)>.
/// Skips blank lines; groups 4 lines per read.
fn parse_fastq(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut reads = Vec::new();
    let mut i = 0;
    while i + 3 < lines.len() {
        let header = lines[i];
        if header.starts_with('@') {
            let seq = lines[i + 1].to_string();
            // lines[i+2] is '+' separator
            let qual = lines[i + 3].to_string();
            reads.push((seq, qual));
            i += 4;
        } else {
            i += 1;
        }
    }
    reads
}

fn mean_phred(qual: &str) -> f64 {
    if qual.is_empty() {
        return 0.0;
    }
    let sum: f64 = qual.bytes().map(|b| (b.saturating_sub(33)) as f64).sum();
    sum / qual.len() as f64
}

fn gc_fraction(seq: &str) -> f64 {
    if seq.is_empty() {
        return 0.0;
    }
    let gc = seq
        .bytes()
        .filter(|&b| matches!(b, b'G' | b'C' | b'g' | b'c'))
        .count();
    gc as f64 / seq.len() as f64
}

fn compute_n50(lengths: &[usize]) -> usize {
    if lengths.is_empty() {
        return 0;
    }
    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a)); // descending
    let total: usize = sorted.iter().sum();
    let half = total / 2;
    let mut cumsum = 0usize;
    for &l in &sorted {
        cumsum += l;
        if cumsum >= half {
            return l;
        }
    }
    *sorted.last().unwrap_or(&0)
}

fn median_f64(v: &mut Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    }
}

// ── fastq_stats ───────────────────────────────────────────────────────

fn builtin_fastq_stats(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "fastq_stats")?;
    let reads = parse_fastq(text);

    if reads.is_empty() {
        let mut rec = HashMap::new();
        for key in &["n_reads", "total_bases", "n50", "max_length", "min_length"] {
            rec.insert(key.to_string(), Value::Int(0));
        }
        for key in &["mean_length", "median_length", "mean_quality"] {
            rec.insert(key.to_string(), Value::Float(0.0));
        }
        return Ok(Value::Record((rec).into()));
    }

    let lengths: Vec<usize> = reads.iter().map(|(s, _)| s.len()).collect();
    let total_bases: usize = lengths.iter().sum();
    let n_reads = reads.len();
    let max_len = *lengths.iter().max().unwrap_or(&0);
    let min_len = *lengths.iter().min().unwrap_or(&0);
    let mean_length = total_bases as f64 / n_reads as f64;

    let mut len_f: Vec<f64> = lengths.iter().map(|&l| l as f64).collect();
    let median_length = median_f64(&mut len_f);

    let n50_val = compute_n50(&lengths);

    // Mean quality across all bases
    let total_qual: f64 = reads
        .iter()
        .flat_map(|(_, q)| q.bytes().map(|b| (b.saturating_sub(33)) as f64))
        .sum();
    let total_qual_bases: usize = reads.iter().map(|(_, q)| q.len()).sum();
    let mean_quality = if total_qual_bases > 0 {
        total_qual / total_qual_bases as f64
    } else {
        0.0
    };

    let mut rec = HashMap::new();
    rec.insert("n_reads".to_string(), Value::Int(n_reads as i64));
    rec.insert("total_bases".to_string(), Value::Int(total_bases as i64));
    rec.insert("mean_length".to_string(), Value::Float(mean_length));
    rec.insert("median_length".to_string(), Value::Float(median_length));
    rec.insert("n50".to_string(), Value::Int(n50_val as i64));
    rec.insert("max_length".to_string(), Value::Int(max_len as i64));
    rec.insert("min_length".to_string(), Value::Int(min_len as i64));
    rec.insert("mean_quality".to_string(), Value::Float(mean_quality));

    Ok(Value::Record((rec).into()))
}

// ── n50 ───────────────────────────────────────────────────────────────

fn builtin_n50(args: Vec<Value>) -> Result<Value> {
    let lengths: Vec<usize> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                Value::Float(f) => Ok(*f as usize),
                _ => Err(BioLangError::type_error(
                    "n50() list must contain integers",
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        _ => return Err(BioLangError::type_error("n50() requires List[Int]", None)),
    };
    Ok(Value::Int(compute_n50(&lengths) as i64))
}

// ── read_length_hist ──────────────────────────────────────────────────

fn builtin_read_length_hist(args: Vec<Value>) -> Result<Value> {
    let lengths: Vec<f64> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as f64),
                Value::Float(f) => Ok(*f),
                _ => Err(BioLangError::type_error(
                    "read_length_hist() list must contain numbers",
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "read_length_hist() requires List",
                None,
            ))
        }
    };

    let bins: usize = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => (*n).max(1) as usize,
            _ => 20,
        }
    } else {
        20
    };

    if lengths.is_empty() {
        return Ok(Value::Table(Table::new(
            vec![
                "bin_start".to_string(),
                "bin_end".to_string(),
                "count".to_string(),
            ],
            vec![],
        )));
    }

    let min_l = lengths.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_l = lengths.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let bin_width = if (max_l - min_l).abs() < 1e-12 {
        1.0
    } else {
        (max_l - min_l) / bins as f64
    };

    let mut counts = vec![0i64; bins];
    for &l in &lengths {
        let idx = ((l - min_l) / bin_width).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }

    let rows: Vec<Vec<Value>> = (0..bins)
        .map(|i| {
            let start = min_l + i as f64 * bin_width;
            let end = start + bin_width;
            vec![
                Value::Float(start),
                Value::Float(end),
                Value::Int(counts[i]),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(
        vec![
            "bin_start".to_string(),
            "bin_end".to_string(),
            "count".to_string(),
        ],
        rows,
    )))
}

// ── quality_filter ────────────────────────────────────────────────────

fn builtin_quality_filter(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "quality_filter")?;
    let min_length = match &args[1] {
        Value::Int(n) => *n as usize,
        Value::Float(f) => *f as usize,
        _ => {
            return Err(BioLangError::type_error(
                "quality_filter() min_length must be Int",
                None,
            ))
        }
    };
    let min_quality = match &args[2] {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => {
            return Err(BioLangError::type_error(
                "quality_filter() min_quality must be Float",
                None,
            ))
        }
    };

    // Re-parse keeping original lines for output
    let all_lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    while i + 3 < all_lines.len() {
        let header = all_lines[i];
        if header.starts_with('@') {
            let seq = all_lines[i + 1];
            let sep = all_lines[i + 2];
            let qual = all_lines[i + 3];
            let passes = seq.len() >= min_length && mean_phred(qual) >= min_quality;
            if passes {
                out.push_str(header);
                out.push('\n');
                out.push_str(seq);
                out.push('\n');
                out.push_str(sep);
                out.push('\n');
                out.push_str(qual);
                out.push('\n');
            }
            i += 4;
        } else {
            i += 1;
        }
    }

    Ok(Value::Str(out))
}

// ── gc_per_read ───────────────────────────────────────────────────────

fn builtin_gc_per_read(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "gc_per_read")?;
    let reads = parse_fastq(text);
    let result: Vec<Value> = reads
        .iter()
        .map(|(seq, _)| Value::Float(gc_fraction(seq)))
        .collect();
    Ok(Value::List((result).into()))
}

// ── read_quality_dist ─────────────────────────────────────────────────

fn builtin_read_quality_dist(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "read_quality_dist")?;
    let reads = parse_fastq(text);

    // Bin mean per-read quality into integer bins 0..=40
    let mut counts = vec![0i64; 41];
    for (_, qual) in &reads {
        let mq = mean_phred(qual).round() as usize;
        let bin = mq.min(40);
        counts[bin] += 1;
    }

    let rows: Vec<Vec<Value>> = (0usize..=40)
        .map(|q| vec![Value::Int(q as i64), Value::Int(counts[q])])
        .collect();

    Ok(Value::Table(Table::new(
        vec!["quality_bin".to_string(), "count".to_string()],
        rows,
    )))
}
