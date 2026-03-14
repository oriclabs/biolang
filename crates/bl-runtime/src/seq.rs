//! Core sequence builtins — WASM-safe (no filesystem or network deps).
//!
//! These provide `transcribe`, `translate`, `gc_content`, `complement`,
//! `reverse_complement`, `base_counts`, `dna`, `rna`, `protein`, `subseq`,
//! `find_motif`, `kmers`, `find_orfs`, `seq_len`, `codon_usage`, `tm`,
//! and `validate_seq`.
//!
//! All logic delegates to `bio_core::seq_ops` — pure string transforms.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, BioSequence, Value};
use std::collections::HashMap;

pub fn seq_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("dna", Arity::Exact(1)),
        ("rna", Arity::Exact(1)),
        ("protein", Arity::Exact(1)),
        ("transcribe", Arity::Exact(1)),
        ("translate", Arity::Exact(1)),
        ("reverse_complement", Arity::Exact(1)),
        ("complement", Arity::Exact(1)),
        ("gc_content", Arity::Exact(1)),
        ("base_counts", Arity::Exact(1)),
        ("subseq", Arity::Range(2, 3)),
        ("find_motif", Arity::Exact(2)),
        ("kmers", Arity::Exact(2)),
        ("find_orfs", Arity::Range(1, 2)),
        ("seq_len", Arity::Exact(1)),
        ("codon_usage", Arity::Exact(1)),
        ("tm", Arity::Exact(1)),
        ("validate_seq", Arity::Exact(1)),
    ]
}

pub fn is_seq_builtin(name: &str) -> bool {
    matches!(
        name,
        "dna"
            | "rna"
            | "protein"
            | "transcribe"
            | "translate"
            | "reverse_complement"
            | "complement"
            | "gc_content"
            | "base_counts"
            | "subseq"
            | "find_motif"
            | "kmers"
            | "find_orfs"
            | "seq_len"
            | "codon_usage"
            | "tm"
            | "validate_seq"
    )
}

pub fn call_seq_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "dna" => {
            let s = require_str(&args[0], "dna")?;
            validate_dna(&s)?;
            Ok(Value::DNA(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "rna" => {
            let s = require_str(&args[0], "rna")?;
            validate_rna(&s)?;
            Ok(Value::RNA(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "protein" => {
            let s = require_str(&args[0], "protein")?;
            Ok(Value::Protein(BioSequence {
                data: s.to_uppercase(),
            }))
        }
        "transcribe" => {
            let seq = require_dna(&args[0], "transcribe")?;
            Ok(Value::RNA(BioSequence {
                data: bl_core::bio_core::seq_ops::transcribe(&seq.data),
            }))
        }
        "translate" => {
            require_rna_or_dna(&args[0], "translate")?;
            let seq_data = match &args[0] {
                Value::DNA(s) => &s.data,
                Value::RNA(s) => &s.data,
                _ => unreachable!(),
            };
            Ok(Value::Protein(BioSequence {
                data: bl_core::bio_core::seq_ops::translate_to_stop(seq_data),
            }))
        }
        "reverse_complement" => match &args[0] {
            Value::DNA(seq) => Ok(Value::DNA(BioSequence {
                data: bl_core::bio_core::seq_ops::reverse_complement_dna(&seq.data),
            })),
            Value::RNA(seq) => Ok(Value::RNA(BioSequence {
                data: bl_core::bio_core::seq_ops::reverse_complement_rna(&seq.data),
            })),
            other => Err(BioLangError::type_error(
                format!(
                    "reverse_complement() requires DNA or RNA, got {}",
                    other.type_of()
                ),
                None,
            )),
        },
        "complement" => match &args[0] {
            Value::DNA(seq) => Ok(Value::DNA(BioSequence {
                data: bl_core::bio_core::seq_ops::complement_dna(&seq.data),
            })),
            Value::RNA(seq) => Ok(Value::RNA(BioSequence {
                data: bl_core::bio_core::seq_ops::complement_rna(&seq.data),
            })),
            other => Err(BioLangError::type_error(
                format!(
                    "complement() requires DNA or RNA, got {}",
                    other.type_of()
                ),
                None,
            )),
        },
        "gc_content" => {
            let seq = require_nucleic(&args[0], "gc_content")?;
            Ok(Value::Float(bl_core::bio_core::seq_ops::gc_content(
                &seq.data,
            )))
        }
        "base_counts" => builtin_base_counts(&args[0]),
        "subseq" => {
            let seq_val = &args[0];
            let start = require_int(&args[1], "subseq")? as usize;
            let data = get_seq_data(seq_val, "subseq")?;

            let end = if args.len() > 2 {
                require_int(&args[2], "subseq")? as usize
            } else {
                data.len()
            };

            if start > data.len() || end > data.len() || start > end {
                return Err(BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!(
                        "subseq({start}, {end}) out of bounds for sequence of length {}",
                        data.len()
                    ),
                    None,
                ));
            }

            let sub = &data[start..end];
            match seq_val {
                Value::DNA(_) => Ok(Value::DNA(BioSequence {
                    data: sub.to_string(),
                })),
                Value::RNA(_) => Ok(Value::RNA(BioSequence {
                    data: sub.to_string(),
                })),
                Value::Protein(_) => Ok(Value::Protein(BioSequence {
                    data: sub.to_string(),
                })),
                _ => unreachable!(),
            }
        }
        "find_motif" => {
            let seq = get_seq_data(&args[0], "find_motif")?;
            let motif = require_str(&args[1], "find_motif")?;
            let positions = bl_core::bio_core::seq_ops::find_motif(&seq, &motif);
            Ok(Value::List(
                positions
                    .into_iter()
                    .map(|p| Value::Int(p as i64))
                    .collect(),
            ))
        }
        "kmers" => {
            let seq = get_seq_data(&args[0], "kmers")?;
            let k = require_int(&args[1], "kmers")? as usize;
            let result = bl_core::bio_core::seq_ops::kmers(&seq, k);
            Ok(Value::List(
                result
                    .into_iter()
                    .map(|s| Value::Str(s.to_string()))
                    .collect(),
            ))
        }
        "find_orfs" => {
            let seq = get_seq_data(&args[0], "find_orfs")?;
            let min_length = if args.len() > 1 {
                require_int(&args[1], "find_orfs")? as usize
            } else {
                100
            };

            let orfs = bl_core::bio_core::seq_ops::find_orfs(&seq, min_length);
            let values: Vec<Value> = orfs
                .into_iter()
                .map(|orf| {
                    let mut fields = HashMap::new();
                    fields.insert("start".to_string(), Value::Int(orf.start as i64));
                    fields.insert("end".to_string(), Value::Int(orf.end as i64));
                    fields.insert(
                        "length".to_string(),
                        Value::Int((orf.end - orf.start) as i64),
                    );
                    fields.insert("frame".to_string(), Value::Int(orf.frame as i64));
                    fields.insert(
                        "protein".to_string(),
                        Value::Protein(BioSequence {
                            data: orf.protein,
                        }),
                    );
                    Value::Record(fields)
                })
                .collect();
            Ok(Value::List(values))
        }
        "seq_len" => {
            let seq = get_seq_data(&args[0], "seq_len")?;
            Ok(Value::Int(seq.len() as i64))
        }
        "codon_usage" => {
            let seq = get_seq_data(&args[0], "codon_usage")?;
            let mut usage: HashMap<String, i64> = HashMap::new();
            let bytes = seq.as_bytes();
            let mut i = 0;
            while i + 3 <= bytes.len() {
                let codon = std::str::from_utf8(&bytes[i..i + 3])
                    .unwrap_or("???")
                    .to_uppercase();
                *usage.entry(codon).or_insert(0) += 1;
                i += 3;
            }
            let mut result = HashMap::new();
            for (codon, count) in usage {
                result.insert(codon, Value::Int(count));
            }
            Ok(Value::Record(result))
        }
        "tm" => {
            let seq = get_seq_data(&args[0], "tm")?;
            // Wallace rule for short oligos, nearest-neighbor approx for longer
            let len = seq.len();
            let gc = seq
                .chars()
                .filter(|c| matches!(c.to_ascii_uppercase(), 'G' | 'C'))
                .count() as f64;
            let at = seq
                .chars()
                .filter(|c| matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'U'))
                .count() as f64;
            let tm = if len < 14 {
                // Wallace rule
                2.0 * at + 4.0 * gc
            } else {
                // Basic salt-adjusted: Tm = 81.5 + 16.6*log10(0.05) + 41*(GC/N) - 600/N
                81.5 + 16.6 * 0.05_f64.log10() + 41.0 * (gc / len as f64)
                    - 600.0 / len as f64
            };
            Ok(Value::Float(tm))
        }
        "validate_seq" => {
            let data = get_seq_data(&args[0], "validate_seq")?;
            let is_valid = bl_core::bio_core::seq_ops::is_valid_dna(&data)
                || bl_core::bio_core::seq_ops::is_valid_rna(&data);
            Ok(Value::Bool(is_valid))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown seq builtin: {name}"),
            None,
        )),
    }
}

// ── Helper functions ────────────────────────────────────────────────

fn builtin_base_counts(val: &Value) -> Result<Value> {
    let data = get_seq_data(val, "base_counts")?;
    let mut counts: HashMap<char, i64> = HashMap::new();
    for c in data.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let total = data.len() as f64;
    let gc = (*counts.get(&'G').unwrap_or(&0) + *counts.get(&'C').unwrap_or(&0)) as f64;

    let mut result = HashMap::new();
    let bases = match val {
        Value::DNA(_) => vec!['A', 'T', 'G', 'C', 'N'],
        Value::RNA(_) => vec!['A', 'U', 'G', 'C', 'N'],
        _ => vec!['A', 'T', 'G', 'C', 'N'],
    };
    for b in bases {
        result.insert(b.to_string(), Value::Int(*counts.get(&b).unwrap_or(&0)));
    }
    result.insert(
        "GC".to_string(),
        Value::Float(if total > 0.0 { gc / total } else { 0.0 }),
    );
    result.insert("total".to_string(), Value::Int(data.len() as i64));
    Ok(Value::Record(result))
}

fn require_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::Str(s) => Ok(s.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_dna<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_rna_or_dna<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA or RNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_nucleic<'a>(val: &'a Value, func: &str) -> Result<&'a BioSequence> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) => Ok(seq),
        other => Err(BioLangError::type_error(
            format!("{func}() requires DNA or RNA, got {}", other.type_of()),
            None,
        )),
    }
}

fn get_seq_data(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires a sequence type, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn validate_dna(s: &str) -> Result<()> {
    if !bl_core::bio_core::seq_ops::is_valid_dna(s) {
        for (i, c) in s.chars().enumerate() {
            if !matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'G' | 'C' | 'N') {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("invalid DNA base '{c}' at position {i}"),
                    None,
                ));
            }
        }
    }
    Ok(())
}

fn validate_rna(s: &str) -> Result<()> {
    if !bl_core::bio_core::seq_ops::is_valid_rna(s) {
        for (i, c) in s.chars().enumerate() {
            if !matches!(c.to_ascii_uppercase(), 'A' | 'U' | 'G' | 'C' | 'N') {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("invalid RNA base '{c}' at position {i}"),
                    None,
                ));
            }
        }
    }
    Ok(())
}
