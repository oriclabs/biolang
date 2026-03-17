//! Core sequence builtins — WASM-safe (no filesystem or network deps).
//!
//! These provide `transcribe`, `translate`, `gc_content`, `complement`,
//! `reverse_complement`, `base_counts`, `dna`, `rna`, `protein`, `subseq`,
//! `find_motif`, `kmers`, `find_orfs`, `seq_len`, `codon_usage`, `tm`,
//! `validate_seq`, `ani`, `containment`, `dereplicate`, `enrichment_score`,
//! `gsea_score`, `resolve`, `blast`, `diff_table`, `qc_report`,
//! `primer_design`, `liftover`, `clinvar_lookup`, `gnomad_freq`,
//! `go_enrichment`, `fetch_sra`, `blast_remote`, `scan_bio`, and `read_pdf`.
//!
//! All logic delegates to `bio_core::seq_ops` — pure string transforms.
//! `resolve` uses the fetch hook for WASM-safe HTTP.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, BioSequence, Value};
use std::collections::{HashMap, HashSet};

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
        ("ani", Arity::Exact(2)),
        ("containment", Arity::Exact(2)),
        ("dereplicate", Arity::Exact(2)),
        ("enrichment_score", Arity::Exact(2)),
        ("gsea_score", Arity::Exact(2)),
        ("resolve", Arity::Range(1, 2)),
        ("blast", Arity::Range(1, 2)),
        ("diff_table", Arity::Range(2, 3)),
        ("qc_report", Arity::Exact(1)),
        ("primer_design", Arity::Range(3, 4)),
        ("liftover", Arity::Range(3, 5)),
        ("clinvar_lookup", Arity::Exact(1)),
        ("gnomad_freq", Arity::Exact(1)),
        ("go_enrichment", Arity::Range(1, 2)),
        ("fetch_sra", Arity::Exact(1)),
        ("blast_remote", Arity::Range(1, 2)),
        ("scan_bio", Arity::Exact(1)),
        ("read_pdf", Arity::Exact(1)),
        ("cite", Arity::Range(1, 2)),
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
            | "ani"
            | "containment"
            | "dereplicate"
            | "enrichment_score"
            | "gsea_score"
            | "resolve"
            | "blast"
            | "diff_table"
            | "qc_report"
            | "primer_design"
            | "liftover"
            | "clinvar_lookup"
            | "gnomad_freq"
            | "go_enrichment"
            | "fetch_sra"
            | "blast_remote"
            | "scan_bio"
            | "read_pdf"
            | "cite"
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
            let seq = get_seq_data_or_str(&args[0], "find_motif")?;
            let motif = get_seq_data_or_str(&args[1], "find_motif")?;
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
        "ani" => {
            let s1 = get_seq_data_or_str(&args[0], "ani")?;
            let s2 = get_seq_data_or_str(&args[1], "ani")?;
            let k = 21;
            if s1.len() < k || s2.len() < k {
                return Ok(Value::Float(0.0));
            }
            let set1: HashSet<&str> = (0..=s1.len() - k).map(|i| &s1[i..i + k]).collect();
            let set2: HashSet<&str> = (0..=s2.len() - k).map(|i| &s2[i..i + k]).collect();
            let intersection = set1.intersection(&set2).count() as f64;
            let union = set1.union(&set2).count() as f64;
            let jaccard = if union > 0.0 {
                intersection / union
            } else {
                0.0
            };
            Ok(Value::Float(jaccard))
        }
        "containment" => {
            let s1 = get_seq_data_or_str(&args[0], "containment")?;
            let s2 = get_seq_data_or_str(&args[1], "containment")?;
            let k = 21;
            if s1.len() < k || s2.len() < k {
                return Ok(Value::Float(0.0));
            }
            let set1: HashSet<&str> = (0..=s1.len() - k).map(|i| &s1[i..i + k]).collect();
            let set2: HashSet<&str> = (0..=s2.len() - k).map(|i| &s2[i..i + k]).collect();
            let intersection = set1.intersection(&set2).count() as f64;
            let containment = if !set1.is_empty() {
                intersection / set1.len() as f64
            } else {
                0.0
            };
            Ok(Value::Float(containment))
        }
        "dereplicate" => {
            let sequences = match &args[0] {
                Value::List(list) => list.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "dereplicate() requires a List of sequences, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };
            let threshold = match &args[1] {
                Value::Float(f) => *f,
                Value::Int(n) => *n as f64,
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "dereplicate() threshold requires Float, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };

            let k = 21;
            // Pre-extract k-mer sets for each sequence
            let seq_data: Vec<String> = sequences
                .iter()
                .map(|v| get_seq_data_or_str(v, "dereplicate"))
                .collect::<Result<Vec<_>>>()?;

            let kmer_sets: Vec<HashSet<String>> = seq_data
                .iter()
                .map(|s| {
                    if s.len() < k {
                        HashSet::new()
                    } else {
                        (0..=s.len() - k).map(|i| s[i..i + k].to_string()).collect()
                    }
                })
                .collect();

            // Greedy clustering
            let mut clusters: Vec<(usize, Vec<usize>)> = Vec::new(); // (representative, members)

            for i in 0..sequences.len() {
                let mut assigned = false;
                for cluster in &mut clusters {
                    let rep = cluster.0;
                    // Compute ANI (Jaccard) between i and representative
                    let intersection = kmer_sets[i].iter().filter(|k| kmer_sets[rep].contains(k.as_str())).count() as f64;
                    let union_size = kmer_sets[i].len() + kmer_sets[rep].len()
                        - kmer_sets[i].iter().filter(|k| kmer_sets[rep].contains(k.as_str())).count();
                    let jaccard = if union_size > 0 {
                        intersection / union_size as f64
                    } else {
                        0.0
                    };
                    if jaccard >= threshold {
                        cluster.1.push(i);
                        assigned = true;
                        break;
                    }
                }
                if !assigned {
                    clusters.push((i, vec![i]));
                }
            }

            let result: Vec<Value> = clusters
                .into_iter()
                .map(|(rep, members)| {
                    let mut fields = HashMap::new();
                    fields.insert("representative".to_string(), Value::Int(rep as i64));
                    fields.insert(
                        "members".to_string(),
                        Value::List(members.iter().map(|&m| Value::Int(m as i64)).collect()),
                    );
                    fields.insert("size".to_string(), Value::Int(members.len() as i64));
                    Value::Record(fields)
                })
                .collect();
            Ok(Value::List(result))
        }
        "enrichment_score" => {
            let values = match &args[0] {
                Value::Record(map) => map.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "enrichment_score() requires a Record of gene:value, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };
            let gene_set = match &args[1] {
                Value::List(list) => list.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "enrichment_score() gene_set requires a List, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };

            // Compute mean of all values
            let all_vals: Vec<f64> = values
                .values()
                .filter_map(|v| match v {
                    Value::Float(f) => Some(*f),
                    Value::Int(n) => Some(*n as f64),
                    _ => None,
                })
                .collect();
            let global_mean = if all_vals.is_empty() {
                0.0
            } else {
                all_vals.iter().sum::<f64>() / all_vals.len() as f64
            };

            // Compute mean of gene set values that exist in the record
            let mut set_sum = 0.0;
            let mut set_count = 0usize;
            for gene in &gene_set {
                let gene_name = match gene {
                    Value::Str(s) => s.clone(),
                    other => other.to_string(),
                };
                if let Some(val) = values.get(&gene_name) {
                    match val {
                        Value::Float(f) => {
                            set_sum += f;
                            set_count += 1;
                        }
                        Value::Int(n) => {
                            set_sum += *n as f64;
                            set_count += 1;
                        }
                        _ => {}
                    }
                }
            }
            let set_mean = if set_count == 0 {
                0.0
            } else {
                set_sum / set_count as f64
            };

            let score = if global_mean != 0.0 {
                set_mean / global_mean
            } else {
                0.0
            };
            Ok(Value::Float(score))
        }
        "gsea_score" => {
            let ranked_list = match &args[0] {
                Value::List(list) => list.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "gsea_score() requires a ranked List of gene names, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };
            let gene_set = match &args[1] {
                Value::List(list) => list.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!(
                            "gsea_score() gene_set requires a List, got {}",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            };

            let gene_set_names: HashSet<String> = gene_set
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();

            let n = ranked_list.len() as f64;
            let nh = gene_set_names.len() as f64;

            if n == 0.0 || nh == 0.0 || n == nh {
                return Ok(Value::Float(0.0));
            }

            // Standard GSEA walking sum:
            // hit increment = sqrt((N - Nh) / Nh)
            // miss decrement = sqrt(Nh / (N - Nh))
            let hit_inc = ((n - nh) / nh).sqrt();
            let miss_dec = (nh / (n - nh)).sqrt();

            let mut running_sum = 0.0_f64;
            let mut max_dev = 0.0_f64;

            for gene_val in &ranked_list {
                let gene_name = match gene_val {
                    Value::Str(s) => s.clone(),
                    other => other.to_string(),
                };
                if gene_set_names.contains(&gene_name) {
                    running_sum += hit_inc;
                } else {
                    running_sum -= miss_dec;
                }
                if running_sum.abs() > max_dev.abs() {
                    max_dev = running_sum;
                }
            }

            Ok(Value::Float(max_dev))
        }
        "resolve" => builtin_resolve(&args),
        "blast" => builtin_blast(&args),
        "diff_table" => builtin_diff_table(&args),
        "qc_report" => builtin_qc_report(&args),
        "primer_design" => builtin_primer_design(&args),
        "liftover" => builtin_liftover(&args),
        "clinvar_lookup" => builtin_clinvar_lookup(&args),
        "gnomad_freq" => builtin_gnomad_freq(&args),
        "go_enrichment" => builtin_go_enrichment(&args),
        "fetch_sra" => builtin_fetch_sra(&args),
        "blast_remote" => builtin_blast_remote(&args),
        "scan_bio" => builtin_scan_bio(&args),
        "read_pdf" => builtin_read_pdf(&args),
        "cite" => builtin_cite(&args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown seq builtin: {name}"),
            None,
        )),
    }
}

// ── resolve() — unified bioinformatics ID resolver ──────────────────

fn builtin_resolve(args: &[Value]) -> Result<Value> {
    let identifier = match &args[0] {
        Value::Str(s) => s.trim().to_string(),
        Value::Int(n) => n.to_string(),
        other => {
            return Err(BioLangError::type_error(
                format!("resolve() requires Str or Int identifier, got {}", other.type_of()),
                None,
            ))
        }
    };

    let db_filter = if args.len() > 1 {
        match &args[1] {
            Value::Str(s) => Some(s.to_lowercase()),
            _ => None,
        }
    } else {
        None
    };

    let id_type = detect_id_type(&identifier);

    match db_filter.as_deref() {
        Some("ncbi") => resolve_ncbi(&identifier, &id_type),
        Some("ensembl") => resolve_ensembl(&identifier, &id_type),
        Some("uniprot") => resolve_uniprot(&identifier, &id_type),
        Some(other) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("resolve(): unknown database '{other}', use 'ncbi', 'ensembl', or 'uniprot'"),
            None,
        )),
        None => {
            // Auto-route based on ID type
            match id_type.as_str() {
                "ensembl_gene" | "ensembl_transcript" | "ensembl_protein" => {
                    resolve_ensembl(&identifier, &id_type)
                }
                "uniprot" => resolve_uniprot(&identifier, &id_type),
                _ => resolve_ncbi(&identifier, &id_type),
            }
        }
    }
}

fn detect_id_type(id: &str) -> String {
    let upper = id.to_uppercase();
    if upper.starts_with("ENSG") {
        "ensembl_gene".to_string()
    } else if upper.starts_with("ENST") {
        "ensembl_transcript".to_string()
    } else if upper.starts_with("ENSP") {
        "ensembl_protein".to_string()
    } else if upper.starts_with("NM_") || upper.starts_with("NR_") {
        "refseq_mrna".to_string()
    } else if upper.starts_with("NP_") {
        "refseq_protein".to_string()
    } else if upper.starts_with("NC_") {
        "refseq_chromosome".to_string()
    } else if is_uniprot_accession(id) {
        "uniprot".to_string()
    } else if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
        "ncbi_gene_id".to_string()
    } else {
        "gene_symbol".to_string()
    }
}

fn is_uniprot_accession(id: &str) -> bool {
    // UniProt: starts with P, Q, or O followed by digit(s) and alphanumerics, 6-10 chars
    let bytes = id.as_bytes();
    if bytes.len() < 6 || bytes.len() > 10 {
        return false;
    }
    matches!(bytes[0], b'P' | b'Q' | b'O' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F'
             | b'G' | b'H' | b'I' | b'J' | b'K' | b'L' | b'M' | b'N' | b'R' | b'S'
             | b'T' | b'U' | b'V' | b'W' | b'X' | b'Y' | b'Z')
        && bytes.get(1).map_or(false, |b| b.is_ascii_digit())
        && bytes[2..].iter().all(|b| b.is_ascii_alphanumeric())
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            ' ' => out.push('+'),
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            _ => {
                let mut buf = [0u8; 4];
                for &b in c.encode_utf8(&mut buf).as_bytes() {
                    out.push('%');
                    out.push_str(&format!("{b:02X}"));
                }
            }
        }
    }
    out
}

fn fetch_json(url: &str) -> Result<serde_json::Value> {
    if let Some(result) = crate::csv::try_fetch_url(url) {
        let text = result.map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("resolve(): fetch error: {e}"), None)
        })?;
        serde_json::from_str(&text).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("resolve(): JSON parse error: {e}"), None)
        })
    } else {
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            "resolve(): no fetch hook available (network not configured)".to_string(),
            None,
        ))
    }
}

fn json_str(v: &serde_json::Value, key: &str) -> Value {
    match v.get(key) {
        Some(serde_json::Value::String(s)) => Value::Str(s.clone()),
        Some(serde_json::Value::Number(n)) => Value::Str(n.to_string()),
        _ => Value::Nil,
    }
}

fn resolve_ncbi(identifier: &str, id_type: &str) -> Result<Value> {
    let gene_id = if id_type == "ncbi_gene_id" {
        identifier.to_string()
    } else {
        // Search for the gene symbol/accession
        let search_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=gene&term={}[sym]+AND+human[orgn]&retmode=json",
            url_encode(identifier)
        );
        let search_json = fetch_json(&search_url)?;
        let id_list = search_json
            .pointer("/esearchresult/idlist")
            .and_then(|v| v.as_array());
        match id_list.and_then(|arr| arr.first()).and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let mut fields = HashMap::new();
                fields.insert("input".to_string(), Value::Str(identifier.to_string()));
                fields.insert("type".to_string(), Value::Str(id_type.to_string()));
                fields.insert("error".to_string(), Value::Str("no results found".to_string()));
                return Ok(Value::Record(fields));
            }
        }
    };

    // Fetch gene summary
    let summary_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=gene&id={gene_id}&retmode=json"
    );
    let summary_json = fetch_json(&summary_url)?;

    let gene_data = summary_json
        .pointer(&format!("/result/{gene_id}"))
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let mut fields = HashMap::new();
    fields.insert("input".to_string(), Value::Str(identifier.to_string()));
    fields.insert("type".to_string(), Value::Str(id_type.to_string()));
    fields.insert("ncbi_gene_id".to_string(), Value::Str(gene_id));
    fields.insert("symbol".to_string(), json_str(&gene_data, "name"));
    fields.insert("name".to_string(), json_str(&gene_data, "description"));
    fields.insert("chromosome".to_string(), json_str(&gene_data, "chromosome"));

    // Extract aliases (otheraliases field is comma-separated)
    let aliases = match gene_data.get("otheraliases").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Value::List(
            s.split(',').map(|a| Value::Str(a.trim().to_string())).collect(),
        ),
        _ => Value::List(vec![]),
    };
    fields.insert("aliases".to_string(), aliases);

    // Try to extract other designations
    fields.insert("description".to_string(), json_str(&gene_data, "otherdesignations"));

    Ok(Value::Record(fields))
}

fn resolve_ensembl(identifier: &str, id_type: &str) -> Result<Value> {
    let url = format!(
        "https://rest.ensembl.org/lookup/id/{}?content-type=application/json;expand=1",
        url_encode(identifier)
    );
    let json = fetch_json(&url)?;

    let mut fields = HashMap::new();
    fields.insert("input".to_string(), Value::Str(identifier.to_string()));
    fields.insert("type".to_string(), Value::Str(id_type.to_string()));
    fields.insert("ensembl_id".to_string(), json_str(&json, "id"));
    fields.insert("symbol".to_string(), json_str(&json, "display_name"));
    fields.insert("name".to_string(), json_str(&json, "description"));
    fields.insert("biotype".to_string(), json_str(&json, "biotype"));
    fields.insert("species".to_string(), json_str(&json, "species"));

    // Chromosome/location
    fields.insert("chromosome".to_string(), json_str(&json, "seq_region_name"));
    if let (Some(start), Some(end)) = (
        json.get("start").and_then(|v| v.as_i64()),
        json.get("end").and_then(|v| v.as_i64()),
    ) {
        fields.insert("start".to_string(), Value::Int(start));
        fields.insert("end".to_string(), Value::Int(end));
    }
    fields.insert("strand".to_string(), match json.get("strand").and_then(|v| v.as_i64()) {
        Some(1) => Value::Str("+".to_string()),
        Some(-1) => Value::Str("-".to_string()),
        _ => Value::Nil,
    });

    Ok(Value::Record(fields))
}

fn resolve_uniprot(identifier: &str, id_type: &str) -> Result<Value> {
    let url = format!(
        "https://rest.uniprot.org/uniprotkb/{}.json",
        url_encode(identifier)
    );
    let json = fetch_json(&url)?;

    let mut fields = HashMap::new();
    fields.insert("input".to_string(), Value::Str(identifier.to_string()));
    fields.insert("type".to_string(), Value::Str(id_type.to_string()));
    fields.insert("uniprot_id".to_string(), json_str(&json, "primaryAccession"));

    // Protein name
    let protein_name = json
        .pointer("/proteinDescription/recommendedName/fullName/value")
        .and_then(|v| v.as_str())
        .map(|s| Value::Str(s.to_string()))
        .unwrap_or(Value::Nil);
    fields.insert("name".to_string(), protein_name);

    // Gene name
    let gene_name = json
        .pointer("/genes/0/geneName/value")
        .and_then(|v| v.as_str())
        .map(|s| Value::Str(s.to_string()))
        .unwrap_or(Value::Nil);
    fields.insert("symbol".to_string(), gene_name);

    // Organism
    let organism = json
        .pointer("/organism/scientificName")
        .and_then(|v| v.as_str())
        .map(|s| Value::Str(s.to_string()))
        .unwrap_or(Value::Nil);
    fields.insert("organism".to_string(), organism);

    // Cross-references: extract Ensembl and NCBI gene IDs
    if let Some(xrefs) = json.get("uniProtKBCrossReferences").and_then(|v| v.as_array()) {
        for xref in xrefs {
            let db = xref.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let id = xref.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match db {
                "Ensembl" => {
                    if !fields.contains_key("ensembl_id") {
                        // Look in properties for gene ID
                        if let Some(props) = xref.get("properties").and_then(|v| v.as_array()) {
                            for prop in props {
                                if prop.get("key").and_then(|v| v.as_str()) == Some("GeneId") {
                                    if let Some(gid) = prop.get("value").and_then(|v| v.as_str()) {
                                        fields.insert("ensembl_id".to_string(), Value::Str(gid.to_string()));
                                    }
                                }
                            }
                        }
                        if !fields.contains_key("ensembl_id") {
                            fields.insert("ensembl_id".to_string(), Value::Str(id.to_string()));
                        }
                    }
                }
                "GeneID" => {
                    if !fields.contains_key("ncbi_gene_id") {
                        fields.insert("ncbi_gene_id".to_string(), Value::Str(id.to_string()));
                    }
                }
                _ => {}
            }
        }
    }

    // Fill missing xrefs with Nil
    fields.entry("ensembl_id".to_string()).or_insert(Value::Nil);
    fields.entry("ncbi_gene_id".to_string()).or_insert(Value::Nil);

    Ok(Value::Record(fields))
}

// ── blast() — local k-mer overlap BLAST-like search ─────────────────

fn builtin_blast(args: &[Value]) -> Result<Value> {
    let query = get_seq_data_or_str(&args[0], "blast")?;
    let targets = match args.get(1) {
        Some(Value::List(list)) => list.clone(),
        Some(other) => {
            return Err(BioLangError::type_error(
                format!("blast() target must be a List of Records, got {}", other.type_of()),
                None,
            ))
        }
        None => return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "blast() requires a target sequence list as second argument".to_string(),
            None,
        )),
    };

    let k = 11.min(query.len());
    if k == 0 {
        return Ok(Value::List(vec![]));
    }
    let query_kmers: HashSet<&str> = (0..=query.len().saturating_sub(k))
        .map(|i| &query[i..i + k])
        .collect();

    let mut hits: Vec<(String, f64, f64, usize)> = Vec::new();
    for target_val in &targets {
        let (id, seq) = match target_val {
            Value::Record(fields) => {
                let id = match fields.get("id") {
                    Some(Value::Str(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => "unknown".to_string(),
                };
                let seq = match fields.get("sequence") {
                    Some(v) => get_seq_data_or_str(v, "blast")?,
                    None => continue,
                };
                (id, seq)
            }
            other => {
                let seq = get_seq_data_or_str(other, "blast")?;
                (String::new(), seq)
            }
        };

        if seq.len() < k {
            continue;
        }
        let target_kmers: HashSet<&str> = (0..=seq.len() - k)
            .map(|i| &seq[i..i + k])
            .collect();
        let shared = query_kmers.iter().filter(|km| target_kmers.contains(*km)).count();
        let total = query_kmers.len().max(1);
        let score = shared as f64 / total as f64;
        let identity = score * 100.0;
        let alignment_length = shared * k;
        if shared > 0 {
            hits.push((id, score, identity, alignment_length));
        }
    }

    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(10);

    let result: Vec<Value> = hits
        .into_iter()
        .map(|(id, score, identity, alen)| {
            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Str(id));
            fields.insert("score".to_string(), Value::Float(score));
            fields.insert("identity".to_string(), Value::Float(identity));
            fields.insert("alignment_length".to_string(), Value::Int(alen as i64));
            Value::Record(fields)
        })
        .collect();
    Ok(Value::List(result))
}

// ── diff_table() — compare two tables/lists of records ──────────────

fn builtin_diff_table(args: &[Value]) -> Result<Value> {
    let rows_a = extract_rows(&args[0], "diff_table")?;
    let rows_b = extract_rows(&args[1], "diff_table")?;
    let key_col = match args.get(2) {
        Some(Value::Str(s)) => Some(s.clone()),
        _ => None,
    };

    let mut added: Vec<Value> = Vec::new();
    let mut removed: Vec<Value> = Vec::new();
    let mut changed: Vec<Value> = Vec::new();
    let mut unchanged_count: i64 = 0;

    if let Some(ref key) = key_col {
        // Key-based matching
        let map_a: HashMap<String, &HashMap<String, Value>> = rows_a
            .iter()
            .filter_map(|r| {
                r.get(key)
                    .map(|v| (format!("{v}"), r))
            })
            .collect();
        let map_b: HashMap<String, &HashMap<String, Value>> = rows_b
            .iter()
            .filter_map(|r| {
                r.get(key)
                    .map(|v| (format!("{v}"), r))
            })
            .collect();

        for (k, row_a) in &map_a {
            if let Some(row_b) = map_b.get(k) {
                let diffs = diff_records(k, row_a, row_b);
                if diffs.is_empty() {
                    unchanged_count += 1;
                } else {
                    changed.extend(diffs);
                }
            } else {
                removed.push(Value::Record((*row_a).clone()));
            }
        }
        for (k, row_b) in &map_b {
            if !map_a.contains_key(k) {
                added.push(Value::Record((*row_b).clone()));
            }
        }
    } else {
        // Index-based matching
        let max_len = rows_a.len().max(rows_b.len());
        for i in 0..max_len {
            match (rows_a.get(i), rows_b.get(i)) {
                (Some(a), Some(b)) => {
                    let key_str = format!("{i}");
                    let diffs = diff_records(&key_str, a, b);
                    if diffs.is_empty() {
                        unchanged_count += 1;
                    } else {
                        changed.extend(diffs);
                    }
                }
                (Some(a), None) => removed.push(Value::Record(a.clone())),
                (None, Some(b)) => added.push(Value::Record(b.clone())),
                (None, None) => {}
            }
        }
    }

    let mut result = HashMap::new();
    result.insert("added".to_string(), Value::List(added));
    result.insert("removed".to_string(), Value::List(removed));
    result.insert("changed".to_string(), Value::List(changed));
    result.insert("unchanged".to_string(), Value::Int(unchanged_count));
    Ok(Value::Record(result))
}

fn extract_rows(val: &Value, func: &str) -> Result<Vec<HashMap<String, Value>>> {
    match val {
        Value::List(list) => {
            let mut rows = Vec::new();
            for item in list {
                match item {
                    Value::Record(fields) => rows.push(fields.clone()),
                    _ => return Err(BioLangError::type_error(
                        format!("{func}() list items must be Records"),
                        None,
                    )),
                }
            }
            Ok(rows)
        }
        Value::Table(table) => {
            let mut rows = Vec::new();
            for row_vals in &table.rows {
                let mut row = HashMap::new();
                for (ci, col_name) in table.columns.iter().enumerate() {
                    if let Some(val) = row_vals.get(ci) {
                        row.insert(col_name.clone(), val.clone());
                    }
                }
                rows.push(row);
            }
            Ok(rows)
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table or List of Records, got {}", other.type_of()),
            None,
        )),
    }
}

fn diff_records(key: &str, a: &HashMap<String, Value>, b: &HashMap<String, Value>) -> Vec<Value> {
    let mut diffs = Vec::new();
    let all_keys: HashSet<&String> = a.keys().chain(b.keys()).collect();
    for field in all_keys {
        let va = a.get(field);
        let vb = b.get(field);
        let old_str = va.map(|v| format!("{v}")).unwrap_or_default();
        let new_str = vb.map(|v| format!("{v}")).unwrap_or_default();
        if old_str != new_str {
            let mut entry = HashMap::new();
            entry.insert("key".to_string(), Value::Str(key.to_string()));
            entry.insert("field".to_string(), Value::Str(field.clone()));
            entry.insert(
                "old_value".to_string(),
                va.cloned().unwrap_or(Value::Nil),
            );
            entry.insert(
                "new_value".to_string(),
                vb.cloned().unwrap_or(Value::Nil),
            );
            diffs.push(Value::Record(entry));
        }
    }
    diffs
}

// ── qc_report() — comprehensive FASTQ QC metrics ───────────────────

fn builtin_qc_report(args: &[Value]) -> Result<Value> {
    let reads = match &args[0] {
        Value::List(list) => list,
        other => {
            return Err(BioLangError::type_error(
                format!("qc_report() requires a List of FASTQ records, got {}", other.type_of()),
                None,
            ))
        }
    };

    if reads.is_empty() {
        let mut r = HashMap::new();
        r.insert("total_reads".to_string(), Value::Int(0));
        r.insert("total_bases".to_string(), Value::Int(0));
        return Ok(Value::Record(r));
    }

    let mut total_bases: i64 = 0;
    let mut lengths: Vec<usize> = Vec::with_capacity(reads.len());
    let mut qual_sums: Vec<f64> = Vec::with_capacity(reads.len());
    let mut gc_fracs: Vec<f64> = Vec::with_capacity(reads.len());
    let mut n_count: i64 = 0;
    let mut seen_seqs: HashMap<String, usize> = HashMap::new();

    for read in reads {
        let fields = match read {
            Value::Record(f) => f,
            _ => continue,
        };

        let seq_str = match fields.get("sequence").or_else(|| fields.get("seq")) {
            Some(v) => get_seq_data_or_str(v, "qc_report").unwrap_or_default(),
            None => continue,
        };
        let qual_str = match fields.get("quality").or_else(|| fields.get("qual")) {
            Some(Value::Str(s)) => s.clone(),
            _ => String::new(),
        };

        let len = seq_str.len();
        lengths.push(len);
        total_bases += len as i64;

        // GC content
        let gc = seq_str.chars().filter(|c| matches!(c.to_ascii_uppercase(), 'G' | 'C')).count();
        gc_fracs.push(if len > 0 { gc as f64 / len as f64 } else { 0.0 });

        // N count
        n_count += seq_str.chars().filter(|c| c.to_ascii_uppercase() == 'N').count() as i64;

        // Quality scores (Phred+33)
        if !qual_str.is_empty() {
            let mean_q: f64 = qual_str.bytes().map(|b| (b.saturating_sub(33)) as f64).sum::<f64>()
                / qual_str.len().max(1) as f64;
            qual_sums.push(mean_q);
        }

        // Duplicate tracking
        *seen_seqs.entry(seq_str).or_insert(0) += 1;
    }

    let total_reads = reads.len() as i64;
    lengths.sort_unstable();

    let min_len = *lengths.first().unwrap_or(&0) as i64;
    let max_len = *lengths.last().unwrap_or(&0) as i64;
    let mean_len = total_bases as f64 / total_reads.max(1) as f64;

    // Quality stats
    qual_sums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean_quality = if qual_sums.is_empty() {
        0.0
    } else {
        qual_sums.iter().sum::<f64>() / qual_sums.len() as f64
    };
    let median_quality = if qual_sums.is_empty() {
        0.0
    } else {
        let mid = qual_sums.len() / 2;
        if qual_sums.len() % 2 == 0 {
            (qual_sums[mid - 1] + qual_sums[mid]) / 2.0
        } else {
            qual_sums[mid]
        }
    };

    let q20_count = qual_sums.iter().filter(|&&q| q >= 20.0).count() as f64;
    let q30_count = qual_sums.iter().filter(|&&q| q >= 30.0).count() as f64;
    let q20_pct = if qual_sums.is_empty() { 0.0 } else { q20_count / qual_sums.len() as f64 * 100.0 };
    let q30_pct = if qual_sums.is_empty() { 0.0 } else { q30_count / qual_sums.len() as f64 * 100.0 };

    let gc_mean = if gc_fracs.is_empty() {
        0.0
    } else {
        gc_fracs.iter().sum::<f64>() / gc_fracs.len() as f64
    };

    let n_pct = if total_bases > 0 {
        n_count as f64 / total_bases as f64 * 100.0
    } else {
        0.0
    };

    let dup_reads = seen_seqs.values().filter(|&&c| c > 1).map(|&c| c - 1).sum::<usize>();
    let duplicate_pct = if total_reads > 0 {
        dup_reads as f64 / total_reads as f64 * 100.0
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert("total_reads".to_string(), Value::Int(total_reads));
    result.insert("total_bases".to_string(), Value::Int(total_bases));
    result.insert("mean_length".to_string(), Value::Float(mean_len));
    result.insert("min_length".to_string(), Value::Int(min_len));
    result.insert("max_length".to_string(), Value::Int(max_len));
    result.insert("mean_quality".to_string(), Value::Float(mean_quality));
    result.insert("median_quality".to_string(), Value::Float(median_quality));
    result.insert("q20_pct".to_string(), Value::Float(q20_pct));
    result.insert("q30_pct".to_string(), Value::Float(q30_pct));
    result.insert("gc_mean".to_string(), Value::Float(gc_mean));
    result.insert("n_pct".to_string(), Value::Float(n_pct));
    result.insert("duplicate_pct".to_string(), Value::Float(duplicate_pct));
    Ok(Value::Record(result))
}

// ── primer_design() — PCR primer design ─────────────────────────────

fn builtin_primer_design(args: &[Value]) -> Result<Value> {
    let seq = get_seq_data_or_str(&args[0], "primer_design")?.to_uppercase();
    let target_start = require_int(&args[1], "primer_design")? as usize;
    let target_end = require_int(&args[2], "primer_design")? as usize;

    if target_start >= target_end || target_end > seq.len() {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!("primer_design(): target region {target_start}..{target_end} invalid for sequence of length {}", seq.len()),
            None,
        ));
    }

    // Parse options
    let (min_len, max_len, min_tm, max_tm, min_gc, max_gc) = match args.get(3) {
        Some(Value::Record(opts)) => {
            let get_f = |k: &str, def: f64| -> f64 {
                match opts.get(k) {
                    Some(Value::Float(f)) => *f,
                    Some(Value::Int(n)) => *n as f64,
                    _ => def,
                }
            };
            let get_i = |k: &str, def: usize| -> usize {
                match opts.get(k) {
                    Some(Value::Int(n)) => *n as usize,
                    _ => def,
                }
            };
            (get_i("min_length", 18), get_i("max_length", 25),
             get_f("min_tm", 55.0), get_f("max_tm", 65.0),
             get_f("min_gc", 0.4), get_f("max_gc", 0.6))
        }
        _ => (18, 25, 55.0, 65.0, 0.4, 0.6),
    };

    let calc_tm = |s: &str| -> f64 {
        let len = s.len() as f64;
        let gc = s.chars().filter(|c| *c == 'G' || *c == 'C').count() as f64;
        64.9 + 41.0 * (gc - 16.4) / len
    };

    let calc_gc = |s: &str| -> f64 {
        let gc = s.chars().filter(|c| *c == 'G' || *c == 'C').count() as f64;
        gc / s.len().max(1) as f64
    };

    let has_poly_run = |s: &str| -> bool {
        let bytes = s.as_bytes();
        for i in 0..bytes.len().saturating_sub(3) {
            if bytes[i] == bytes[i + 1] && bytes[i + 1] == bytes[i + 2] && bytes[i + 2] == bytes[i + 3] {
                return true;
            }
        }
        false
    };

    let has_3prime_self_comp = |s: &str| -> bool {
        // Check if last 4 bases have self-complementarity
        if s.len() < 4 { return false; }
        let tail: Vec<char> = s.chars().rev().take(4).collect();
        let rc: Vec<char> = tail.iter().map(|c| match c {
            'A' => 'T', 'T' => 'A', 'G' => 'C', 'C' => 'G', _ => 'N',
        }).collect();
        tail == rc
    };

    let score_primer = |s: &str| -> f64 {
        let tm = calc_tm(s);
        let gc = calc_gc(s);
        let mut score = 100.0;
        if tm < min_tm { score -= (min_tm - tm) * 5.0; }
        if tm > max_tm { score -= (tm - max_tm) * 5.0; }
        if gc < min_gc { score -= (min_gc - gc) * 50.0; }
        if gc > max_gc { score -= (gc - max_gc) * 50.0; }
        if has_poly_run(s) { score -= 20.0; }
        if has_3prime_self_comp(s) { score -= 15.0; }
        score
    };

    let rev_comp = |s: &str| -> String {
        s.chars().rev().map(|c| match c {
            'A' => 'T', 'T' => 'A', 'G' => 'C', 'C' => 'G', ch => ch,
        }).collect()
    };

    // Collect forward primer candidates (upstream of target)
    let mut fwd_candidates: Vec<(String, f64)> = Vec::new();
    let scan_start = target_start.saturating_sub(200);
    for start in scan_start..target_start.saturating_sub(min_len) {
        for plen in min_len..=max_len {
            let end = start + plen;
            if end > target_start || end > seq.len() { break; }
            let primer = &seq[start..end];
            let s = score_primer(primer);
            if s > 0.0 {
                fwd_candidates.push((primer.to_string(), s));
            }
        }
    }
    fwd_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    fwd_candidates.truncate(10);

    // Collect reverse primer candidates (downstream of target)
    let mut rev_candidates: Vec<(String, f64)> = Vec::new();
    let scan_end = (target_end + 200).min(seq.len());
    for start in target_end..scan_end.saturating_sub(min_len) {
        for plen in min_len..=max_len {
            let end = start + plen;
            if end > seq.len() { break; }
            let region = &seq[start..end];
            let primer = rev_comp(region);
            let s = score_primer(&primer);
            if s > 0.0 {
                rev_candidates.push((primer, s));
            }
        }
    }
    rev_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rev_candidates.truncate(10);

    // Pair top forward and reverse primers
    let mut pairs: Vec<Value> = Vec::new();
    for (fwd, _fs) in fwd_candidates.iter().take(5) {
        for (rev, _rs) in rev_candidates.iter().take(5) {
            let fwd_tm = calc_tm(fwd);
            let rev_tm = calc_tm(rev);
            // Estimate product size from primer positions
            let fwd_end = seq.find(fwd.as_str()).unwrap_or(target_start);
            let rev_rc = rev_comp(rev);
            let rev_start = seq.rfind(rev_rc.as_str()).unwrap_or(target_end);
            let product_size = if rev_start + rev_rc.len() > fwd_end {
                rev_start + rev_rc.len() - fwd_end
            } else {
                0
            };
            let mut fields = HashMap::new();
            fields.insert("forward".to_string(), Value::Str(fwd.clone()));
            fields.insert("reverse".to_string(), Value::Str(rev.clone()));
            fields.insert("product_size".to_string(), Value::Int(product_size as i64));
            fields.insert("fwd_tm".to_string(), Value::Float(fwd_tm));
            fields.insert("rev_tm".to_string(), Value::Float(rev_tm));
            fields.insert("fwd_gc".to_string(), Value::Float(calc_gc(fwd)));
            fields.insert("rev_gc".to_string(), Value::Float(calc_gc(rev)));
            pairs.push(Value::Record(fields));
        }
    }

    // Sort by combined Tm closeness and return top 3
    pairs.sort_by(|a, b| {
        let score = |v: &Value| -> f64 {
            if let Value::Record(f) = v {
                let ft = match f.get("fwd_tm") { Some(Value::Float(f)) => *f, _ => 0.0 };
                let rt = match f.get("rev_tm") { Some(Value::Float(f)) => *f, _ => 0.0 };
                let mid_tm = (min_tm + max_tm) / 2.0;
                -((ft - mid_tm).abs() + (rt - mid_tm).abs() + (ft - rt).abs())
            } else {
                f64::NEG_INFINITY
            }
        };
        score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal)
    });
    pairs.truncate(3);
    Ok(Value::List(pairs))
}

// ── liftover() — coordinate mapping between genome builds ───────────

fn builtin_liftover(args: &[Value]) -> Result<Value> {
    let chrom = match &args[0] {
        Value::Str(s) => s.clone(),
        other => return Err(BioLangError::type_error(
            format!("liftover() chrom requires Str, got {}", other.type_of()),
            None,
        )),
    };
    let start = require_int(&args[1], "liftover")?;
    let end = require_int(&args[2], "liftover")?;
    let from_build = match args.get(3) {
        Some(Value::Str(s)) => s.clone(),
        _ => "GRCh37".to_string(),
    };
    let to_build = match args.get(4) {
        Some(Value::Str(s)) => s.clone(),
        _ => "GRCh38".to_string(),
    };

    let url = format!(
        "https://rest.ensembl.org/map/human/{}/{}:{}..{}:1/{}?content-type=application/json",
        url_encode(&from_build),
        url_encode(&chrom),
        start,
        end,
        url_encode(&to_build),
    );

    let json = fetch_json(&url)?;

    // Parse the mappings array
    let mappings = json.get("mappings").and_then(|v| v.as_array());
    if let Some(maps) = mappings {
        if let Some(first) = maps.first() {
            let mapped = first.get("mapped").unwrap_or(first);
            let mut fields = HashMap::new();
            fields.insert("input_chrom".to_string(), Value::Str(chrom));
            fields.insert("input_start".to_string(), Value::Int(start));
            fields.insert("input_end".to_string(), Value::Int(end));
            fields.insert("mapped_chrom".to_string(), json_str(mapped, "seq_region_name"));
            fields.insert("mapped_start".to_string(), match mapped.get("start").and_then(|v| v.as_i64()) {
                Some(n) => Value::Int(n),
                None => Value::Nil,
            });
            fields.insert("mapped_end".to_string(), match mapped.get("end").and_then(|v| v.as_i64()) {
                Some(n) => Value::Int(n),
                None => Value::Nil,
            });
            fields.insert("strand".to_string(), match mapped.get("strand").and_then(|v| v.as_i64()) {
                Some(1) => Value::Str("+".to_string()),
                Some(-1) => Value::Str("-".to_string()),
                _ => Value::Nil,
            });
            fields.insert("from_build".to_string(), Value::Str(from_build));
            fields.insert("to_build".to_string(), Value::Str(to_build));
            return Ok(Value::Record(fields));
        }
    }

    // No mapping found
    let mut fields = HashMap::new();
    fields.insert("input_chrom".to_string(), Value::Str(chrom));
    fields.insert("input_start".to_string(), Value::Int(start));
    fields.insert("input_end".to_string(), Value::Int(end));
    fields.insert("from_build".to_string(), Value::Str(from_build));
    fields.insert("to_build".to_string(), Value::Str(to_build));
    fields.insert("error".to_string(), Value::Str("no mapping found".to_string()));
    Ok(Value::Record(fields))
}

// ── clinvar_lookup() — query ClinVar via NCBI E-utilities ───────────

fn builtin_clinvar_lookup(args: &[Value]) -> Result<Value> {
    let query = require_str(&args[0], "clinvar_lookup")?;

    // Search ClinVar
    let search_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=clinvar&term={}&retmode=json",
        url_encode(&query)
    );
    let search_json = fetch_json(&search_url)?;
    let id_list = search_json
        .pointer("/esearchresult/idlist")
        .and_then(|v| v.as_array());
    let cv_id = match id_list.and_then(|arr| arr.first()).and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Str(query));
            fields.insert("error".to_string(), Value::Str("no ClinVar results found".to_string()));
            fields.insert("title".to_string(), Value::Nil);
            fields.insert("clinical_significance".to_string(), Value::Nil);
            fields.insert("review_status".to_string(), Value::Nil);
            fields.insert("gene".to_string(), Value::Nil);
            fields.insert("conditions".to_string(), Value::Nil);
            fields.insert("variation_type".to_string(), Value::Nil);
            return Ok(Value::Record(fields));
        }
    };

    // Fetch summary
    let summary_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=clinvar&id={cv_id}&retmode=json"
    );
    let summary_json = fetch_json(&summary_url)?;
    let data = summary_json
        .pointer(&format!("/result/{cv_id}"))
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Str(cv_id));
    fields.insert("title".to_string(), json_str(&data, "title"));

    // Clinical significance
    let clin_sig = data
        .pointer("/clinical_significance/description")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("clinical_significance").and_then(|v| v.as_str()))
        .map(|s| Value::Str(s.to_string()))
        .unwrap_or(Value::Nil);
    fields.insert("clinical_significance".to_string(), clin_sig);

    let review = data
        .pointer("/clinical_significance/review_status")
        .and_then(|v| v.as_str())
        .map(|s| Value::Str(s.to_string()))
        .unwrap_or(Value::Nil);
    fields.insert("review_status".to_string(), review);

    // Gene(s)
    let genes = data
        .get("genes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.get("symbol").and_then(|v| v.as_str()))
                .map(|s| Value::Str(s.to_string()))
                .collect::<Vec<_>>()
        });
    fields.insert(
        "gene".to_string(),
        match genes {
            Some(ref g) if g.len() == 1 => g[0].clone(),
            Some(g) if !g.is_empty() => Value::List(g),
            _ => json_str(&data, "gene_sort"),
        },
    );

    // Conditions (trait names)
    let conditions = data
        .get("trait_set")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("trait_name").and_then(|v| v.as_str()).map(|s| Value::Str(s.to_string()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    fields.insert(
        "conditions".to_string(),
        if conditions.is_empty() { Value::Nil } else { Value::List(conditions) },
    );

    fields.insert("variation_type".to_string(), json_str(&data, "obj_type"));

    Ok(Value::Record(fields))
}

// ── gnomad_freq() — query allele frequencies via myvariant.info ─────

fn builtin_gnomad_freq(args: &[Value]) -> Result<Value> {
    let rsid = require_str(&args[0], "gnomad_freq")?;

    let url = format!(
        "https://myvariant.info/v1/query?q={}&fields=gnomad_genome.af,gnomad_exome.af,dbsnp.gene,dbsnp.ref,dbsnp.alt",
        url_encode(&rsid)
    );
    let json = fetch_json(&url)?;

    let hits = json.get("hits").and_then(|v| v.as_array());
    let hit = hits.and_then(|arr| arr.first());

    let mut fields = HashMap::new();
    fields.insert("rsid".to_string(), Value::Str(rsid));

    match hit {
        Some(h) => {
            // Gene
            let gene = h
                .pointer("/dbsnp/gene/symbol")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    h.pointer("/dbsnp/gene")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|g| g.get("symbol"))
                        .and_then(|v| v.as_str())
                })
                .map(|s| Value::Str(s.to_string()))
                .unwrap_or(Value::Nil);
            fields.insert("gene".to_string(), gene);

            // Ref/Alt
            let ref_allele = h
                .pointer("/dbsnp/ref")
                .and_then(|v| v.as_str())
                .map(|s| Value::Str(s.to_string()))
                .unwrap_or(Value::Nil);
            fields.insert("ref".to_string(), ref_allele);

            let alt_allele = h
                .pointer("/dbsnp/alt")
                .and_then(|v| v.as_str())
                .map(|s| Value::Str(s.to_string()))
                .unwrap_or(Value::Nil);
            fields.insert("alt".to_string(), alt_allele);

            // gnomAD genome AF
            let genome_af = h
                .pointer("/gnomad_genome/af/af")
                .or_else(|| h.pointer("/gnomad_genome/af"))
                .and_then(|v| v.as_f64())
                .map(Value::Float)
                .unwrap_or(Value::Nil);
            fields.insert("genome_af".to_string(), genome_af);

            // gnomAD exome AF
            let exome_af = h
                .pointer("/gnomad_exome/af/af")
                .or_else(|| h.pointer("/gnomad_exome/af"))
                .and_then(|v| v.as_f64())
                .map(Value::Float)
                .unwrap_or(Value::Nil);
            fields.insert("exome_af".to_string(), exome_af);
        }
        None => {
            fields.insert("gene".to_string(), Value::Nil);
            fields.insert("ref".to_string(), Value::Nil);
            fields.insert("alt".to_string(), Value::Nil);
            fields.insert("genome_af".to_string(), Value::Nil);
            fields.insert("exome_af".to_string(), Value::Nil);
        }
    }

    Ok(Value::Record(fields))
}

// ── go_enrichment() — offline GO enrichment with built-in term db ───

fn builtin_go_enrichment(args: &[Value]) -> Result<Value> {
    let gene_list = match &args[0] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.to_uppercase()),
                other => Err(BioLangError::type_error(
                    format!("go_enrichment() gene list must contain strings, got {}", other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("go_enrichment() requires a List of gene symbols, got {}", other.type_of()),
                None,
            ))
        }
    };

    let ontology = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => "GO_Biological_Process_2023",
    };

    // Built-in GO term database (~50 major biological processes)
    let go_terms: Vec<(&str, &str, &[&str])> = match ontology {
        "GO_Molecular_Function_2023" => go_molecular_function_terms(),
        "GO_Cellular_Component_2023" => go_cellular_component_terms(),
        "KEGG_2021_Human" => kegg_pathway_terms(),
        _ => go_biological_process_terms(), // default
    };

    let gene_set: HashSet<String> = gene_list.iter().cloned().collect();
    let n_input = gene_set.len();
    let n_bg: usize = 20000; // approximate human protein-coding gene count

    let mut results: Vec<(f64, Value)> = Vec::new();
    for (go_id, term_name, term_genes) in &go_terms {
        let term_set: HashSet<&str> = term_genes.iter().copied().collect();
        let overlap: Vec<String> = gene_set
            .iter()
            .filter(|g| term_set.contains(g.as_str()))
            .cloned()
            .collect();
        if overlap.is_empty() {
            continue;
        }

        let k = overlap.len();
        let n = term_genes.len();
        // Binomial approximation of hypergeometric p-value
        let p = n as f64 / n_bg as f64;
        let expected = n_input as f64 * p;
        let pvalue = if (k as f64) > expected {
            // Poisson approximation: P(X >= k) where X ~ Poisson(expected)
            poisson_sf(k, expected)
        } else {
            1.0
        };

        let mut fields = HashMap::new();
        fields.insert("term_id".to_string(), Value::Str(go_id.to_string()));
        fields.insert("term".to_string(), Value::Str(term_name.to_string()));
        fields.insert("pvalue".to_string(), Value::Float(pvalue));
        fields.insert(
            "overlap".to_string(),
            Value::Str(format!("{}/{}", k, n)),
        );
        fields.insert(
            "genes".to_string(),
            Value::List(overlap.iter().map(|g| Value::Str(g.clone())).collect()),
        );

        results.push((pvalue, Value::Record(fields)));
    }

    // Sort by p-value ascending
    results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Apply Benjamini-Hochberg correction and take top 20
    let n_tests = results.len().max(1);
    let final_results: Vec<Value> = results
        .into_iter()
        .take(20)
        .enumerate()
        .map(|(i, (_pval, val))| {
            if let Value::Record(mut fields) = val {
                let raw_p = match fields.get("pvalue") {
                    Some(Value::Float(f)) => *f,
                    _ => 1.0,
                };
                let adj_p = (raw_p * n_tests as f64 / (i + 1) as f64).min(1.0);
                fields.insert("adjusted_pvalue".to_string(), Value::Float(adj_p));
                Value::Record(fields)
            } else {
                val
            }
        })
        .collect();

    Ok(Value::List(final_results))
}

/// Poisson survival function: P(X >= k) where X ~ Poisson(lambda)
fn poisson_sf(k: usize, lambda: f64) -> f64 {
    if lambda <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    // CDF = P(X < k) = sum_{i=0}^{k-1} e^{-lambda} * lambda^i / i!
    let mut cdf = 0.0;
    let mut term = (-lambda).exp(); // e^{-lambda} * lambda^0 / 0!
    for i in 0..k {
        cdf += term;
        term *= lambda / (i + 1) as f64;
    }
    (1.0 - cdf).max(0.0)
}

fn go_biological_process_terms() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("GO:0006915", "apoptotic process", &["TP53","BCL2","BAX","CASP3","CASP9","BID","BCL2L1","XIAP","CYCS","APAF1"]),
        ("GO:0006281", "DNA repair", &["BRCA1","BRCA2","ATM","ATR","RAD51","XRCC1","PARP1","MLH1","MSH2","CHEK2"]),
        ("GO:0008283", "cell proliferation", &["MYC","CCND1","CDK4","CDK6","RB1","E2F1","CDKN2A","PCNA","MCM2","MCM7"]),
        ("GO:0007049", "cell cycle", &["CDK1","CDK2","CCNA2","CCNB1","CCNE1","CDC25A","CDC25C","PLK1","AURKA","AURKB"]),
        ("GO:0006468", "protein phosphorylation", &["AKT1","MAPK1","MAPK3","SRC","EGFR","ERBB2","JAK2","ABL1","BRAF","RAF1"]),
        ("GO:0007165", "signal transduction", &["KRAS","HRAS","NRAS","PIK3CA","PTEN","AKT1","MTOR","MAPK1","STAT3","JAK2"]),
        ("GO:0006955", "immune response", &["TNF","IL6","IL1B","IFNG","IL10","CD4","CD8A","FOXP3","TLR4","NFKB1"]),
        ("GO:0006954", "inflammatory response", &["TNF","IL6","IL1B","IL8","CCL2","COX2","PTGS2","NLRP3","CXCL8","NFKB1"]),
        ("GO:0006260", "DNA replication", &["PCNA","MCM2","MCM3","MCM4","MCM5","MCM6","MCM7","POLA1","POLE","RFC1"]),
        ("GO:0006412", "translation", &["EIF4E","EIF4G1","EIF2S1","MTOR","RPS6","EEF2","RPS6KB1","EIF3A","EIF5A","RPL5"]),
        ("GO:0006351", "transcription, DNA-templated", &["TP53","MYC","JUN","FOS","SP1","CTCF","YY1","TFIIB","POLR2A","MED1"]),
        ("GO:0016032", "viral process", &["ACE2","TMPRSS2","IFNAR1","IFNB1","MX1","OAS1","ISG15","BST2","TRIM25","IRF3"]),
        ("GO:0007596", "blood coagulation", &["F2","F5","F7","F8","F9","F10","F11","FGA","FGB","VWF"]),
        ("GO:0042981", "regulation of apoptotic process", &["BCL2","MCL1","BCL2L1","BAX","BAK1","BIM","PUMA","NOXA","SURVIVIN","FLIP"]),
        ("GO:0001525", "angiogenesis", &["VEGFA","VEGFR2","FGF2","ANGPT1","ANGPT2","HIF1A","DLL4","NOTCH1","NRP1","ENG"]),
        ("GO:0008284", "positive regulation of cell proliferation", &["EGF","EGFR","PDGFB","FGF2","IGF1","ERBB2","MET","KIT","FGFR1","FGFR2"]),
        ("GO:0043065", "positive regulation of apoptotic process", &["TP53","BAX","BID","BAK1","CASP8","FAS","FASLG","TRAIL","DR5","APAF1"]),
        ("GO:0006935", "chemotaxis", &["CXCL12","CXCR4","CCL2","CCR2","CCL5","CCR5","CXCL8","CXCR2","CCL19","CCR7"]),
        ("GO:0030154", "cell differentiation", &["NOTCH1","WNT3A","BMP4","SHH","SOX2","PAX6","GATA1","PU1","RUNX1","MYOD1"]),
        ("GO:0001666", "response to hypoxia", &["HIF1A","EPAS1","VHL","PHD2","EPO","VEGFA","GLUT1","LDHA","PDK1","CA9"]),
        ("GO:0006629", "lipid metabolic process", &["FASN","ACACA","HMGCR","SREBF1","PPARG","PPARA","ACOX1","CPT1A","LIPE","DGAT1"]),
        ("GO:0006811", "ion transport", &["SCN5A","KCNQ1","KCNH2","CACNA1C","CFTR","SLC12A1","ATP1A1","CLCN1","TRPV1","KCNJ11"]),
        ("GO:0006814", "sodium ion transport", &["SCN1A","SCN2A","SCN5A","SCN9A","SLC9A1","SLC5A1","SLC5A2","ENaC","ATP1A1","SCN8A"]),
        ("GO:0007399", "nervous system development", &["BDNF","NGF","NTRK2","NTRK1","GDNF","NRXN1","NLGN1","DISC1","DSCAM","ROBO1"]),
        ("GO:0006695", "cholesterol biosynthetic process", &["HMGCR","HMGCS1","SQLE","FDFT1","DHCR7","DHCR24","CYP51A1","SREBF2","ACAT2","MVK"]),
        ("GO:0055114", "oxidation-reduction process", &["SOD1","SOD2","CAT","GPX1","GSR","NQO1","TXNRD1","CYP1A1","CYP3A4","ALDH2"]),
        ("GO:0007155", "cell adhesion", &["CDH1","CDH2","ITGB1","ITGA5","ICAM1","VCAM1","SELE","SELL","CD44","EPCAM"]),
        ("GO:0016055", "Wnt signaling pathway", &["WNT1","WNT3A","CTNNB1","APC","GSK3B","AXIN1","LEF1","TCF7","FZD1","LRP6"]),
        ("GO:0007219", "Notch signaling pathway", &["NOTCH1","NOTCH2","NOTCH3","JAG1","JAG2","DLL1","DLL4","HES1","HEY1","RBPJ"]),
        ("GO:0048015", "phosphatidylinositol-mediated signaling", &["PIK3CA","PIK3R1","PTEN","AKT1","AKT2","PDK1","MTOR","INPP5D","PIK3CB","PIK3CD"]),
        ("GO:0006950", "response to stress", &["HSP90AA1","HSPA1A","HSP90B1","DNAJB1","HSPA5","HSPB1","ATF4","DDIT3","XBP1","ATF6"]),
        ("GO:0006805", "xenobiotic metabolic process", &["CYP1A1","CYP1A2","CYP2D6","CYP3A4","CYP2C9","CYP2C19","UGT1A1","GSTM1","GSTP1","NAT2"]),
        ("GO:0000723", "telomere maintenance", &["TERT","TERC","POT1","TRF1","TRF2","TIN2","TPP1","RAP1","RTEL1","DKC1"]),
        ("GO:0006302", "double-strand break repair", &["BRCA1","BRCA2","RAD51","RAD50","MRE11","NBS1","XRCC4","LIG4","53BP1","RIF1"]),
        ("GO:0045944", "positive regulation of transcription by RNA pol II", &["MYC","JUN","FOS","RELA","SP1","CREB1","SRF","ETS1","STAT1","STAT3"]),
        ("GO:0000278", "mitotic cell cycle", &["CDK1","CCNB1","PLK1","AURKA","BUB1","MAD2L1","CDC20","APC","CENPE","KIF11"]),
        ("GO:0001837", "epithelial to mesenchymal transition", &["SNAI1","SNAI2","TWIST1","ZEB1","ZEB2","CDH1","CDH2","VIM","TGFB1","FOXC2"]),
        ("GO:0006270", "DNA replication initiation", &["ORC1","ORC2","ORC3","ORC4","ORC5","ORC6","CDC6","CDT1","MCM2","MCM4"]),
        ("GO:0045087", "innate immune response", &["TLR2","TLR3","TLR4","TLR7","TLR9","MYD88","STING","CGAS","RIG1","MAVS"]),
        ("GO:0006979", "response to oxidative stress", &["NFE2L2","KEAP1","SOD1","SOD2","CAT","GPX1","HMOX1","NQO1","TXNRD1","PRDX1"]),
        ("GO:0006914", "autophagy", &["BECN1","ATG5","ATG7","ATG12","LC3B","ULK1","SQSTM1","MTOR","AMPK","VPS34"]),
        ("GO:0007169", "transmembrane receptor protein tyrosine kinase signaling", &["EGFR","ERBB2","ERBB3","PDGFRA","FGFR1","MET","RET","ALK","ROS1","KIT"]),
        ("GO:0006936", "muscle contraction", &["MYH7","MYH6","TNNT2","TNNI3","TPM1","ACTC1","MYL2","MYL3","RYR2","SCN5A"]),
        ("GO:0045893", "positive regulation of DNA transcription", &["TP53","RB1","SMAD2","SMAD3","SMAD4","TGFBR1","TGFBR2","BMP2","BMP4","RUNX2"]),
        ("GO:0006511", "ubiquitin-dependent protein catabolic process", &["UBE2I","UBA1","UBE2D1","UBR5","MDM2","FBXW7","BTRC","CUL1","SKP2","VHL"]),
        ("GO:0008380", "RNA splicing", &["SF3B1","U2AF1","SRSF2","ZRSR2","PRPF8","PRPF31","SNRNP200","HNRNPA1","RBFOX2","PTBP1"]),
        ("GO:0007186", "G protein-coupled receptor signaling", &["GNAS","GNAI1","GNB1","ADCY5","ADRB1","ADRB2","DRD2","HTR2A","OPRM1","GRM5"]),
        ("GO:0032496", "response to lipopolysaccharide", &["TLR4","MD2","CD14","MYD88","IRAK1","TRAF6","NFKB1","TNF","IL6","IL1B"]),
        ("GO:0043066", "negative regulation of apoptotic process", &["BCL2","BCL2L1","MCL1","XIAP","BIRC5","AKT1","FLIP","SURVIVIN","IAP","HSP70"]),
    ]
}

fn go_molecular_function_terms() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("GO:0003677", "DNA binding", &["TP53","MYC","JUN","FOS","SP1","CTCF","BRCA1","ATM","E2F1","YY1"]),
        ("GO:0003723", "RNA binding", &["FMR1","HNRNPA1","HNRNPC","ELAVL1","TARDBP","FUS","DICER1","AGO2","DDX3X","PABPC1"]),
        ("GO:0004672", "protein kinase activity", &["AKT1","CDK2","CDK4","MAPK1","MAPK3","EGFR","SRC","ABL1","BRAF","JAK2"]),
        ("GO:0004842", "ubiquitin-protein transferase activity", &["MDM2","FBXW7","RNF8","UBE3A","BRCA1","VHL","TRIM25","NEDD4","ITCH","SMURF1"]),
        ("GO:0016301", "kinase activity", &["PIK3CA","PIK3CB","PIK3CD","AKT1","AKT2","MTOR","RPS6KB1","PDK1","SGK1","DYRK1A"]),
        ("GO:0003700", "DNA-binding transcription factor activity", &["TP53","MYC","JUN","FOS","STAT3","RELA","SP1","ETS1","GATA1","PAX6"]),
        ("GO:0005524", "ATP binding", &["ABCB1","ABCG2","HSP90AA1","HSPA1A","CFTR","ACTB","MYH7","KIF11","SMC1A","SMC3"]),
        ("GO:0004674", "protein serine/threonine kinase activity", &["BRAF","RAF1","MAP2K1","CDK1","CDK2","CHEK1","CHEK2","PLK1","AURKA","NEK2"]),
        ("GO:0046872", "metal ion binding", &["SOD1","CA2","ACE","ACE2","MMP2","MMP9","ADAM17","HDAC1","HDAC2","HDAC3"]),
        ("GO:0003682", "chromatin binding", &["BRD4","BRD2","SMARCA4","SMARCB1","ARID1A","EZH2","KMT2A","KDM5A","BAZ2A","CHD4"]),
    ]
}

fn go_cellular_component_terms() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("GO:0005634", "nucleus", &["TP53","RB1","MYC","BRCA1","BRCA2","ATM","ATR","CTCF","PCNA","RAD51"]),
        ("GO:0005737", "cytoplasm", &["AKT1","MAPK1","MAPK3","SRC","KRAS","HRAS","PTEN","HSP90AA1","ACTB","TUBB"]),
        ("GO:0005886", "plasma membrane", &["EGFR","ERBB2","CDH1","ITGB1","CD44","EPCAM","CFTR","SCN5A","KCNQ1","ACE2"]),
        ("GO:0005739", "mitochondrion", &["BCL2","BAX","CYCS","SOD2","COX4I1","VDAC1","MFN1","MFN2","DRP1","PINK1"]),
        ("GO:0005783", "endoplasmic reticulum", &["HSPA5","CANX","CALR","SEC61A1","ATF6","IRE1","XBP1","PDIA3","UGGT1","ERO1A"]),
        ("GO:0005794", "Golgi apparatus", &["GOLPH3","GM130","GOLGA2","RAB1A","SEC23A","COPI","COPII","MAN1A1","B4GALT1","ST6GAL1"]),
        ("GO:0005829", "cytosol", &["GAPDH","LDHA","PKM","ENO1","TPI1","ALDOA","PFK1","HK2","GPI","PGAM1"]),
        ("GO:0000775", "chromosome, centromeric region", &["CENPA","CENPB","CENPC","CENPF","CENPE","INCENP","AURKB","BUB1","MAD2L1","NDC80"]),
        ("GO:0005694", "chromosome", &["SMC1A","SMC3","RAD21","STAG1","CTCF","H3F3A","H2AFX","TOP2A","TERT","TERC"]),
        ("GO:0005576", "extracellular region", &["TNF","IL6","IL1B","VEGFA","EGF","FGF2","TGFB1","BMP2","WNT3A","SHH"]),
    ]
}

fn kegg_pathway_terms() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("hsa05200", "Pathways in cancer", &["TP53","KRAS","PIK3CA","AKT1","EGFR","BRAF","MYC","PTEN","RB1","CDKN2A"]),
        ("hsa04110", "Cell cycle", &["CDK1","CDK2","CDK4","CDK6","CCND1","CCNE1","RB1","TP53","CDKN1A","E2F1"]),
        ("hsa04151", "PI3K-Akt signaling", &["PIK3CA","PIK3R1","AKT1","AKT2","PTEN","MTOR","PDK1","GSK3B","BAD","FOXO3"]),
        ("hsa04010", "MAPK signaling", &["KRAS","HRAS","NRAS","BRAF","RAF1","MAP2K1","MAPK1","MAPK3","JUN","FOS"]),
        ("hsa04064", "NF-kappa B signaling", &["NFKB1","RELA","IKBKB","IKBKG","TRAF2","TRAF6","TNF","IL1B","TLR4","MYD88"]),
        ("hsa04310", "Wnt signaling", &["WNT1","WNT3A","CTNNB1","APC","GSK3B","AXIN1","FZD1","LRP6","DVL1","TCF7"]),
        ("hsa04330", "Notch signaling", &["NOTCH1","NOTCH2","JAG1","JAG2","DLL1","DLL4","HES1","HEY1","RBPJ","MAML1"]),
        ("hsa04350", "TGF-beta signaling", &["TGFB1","TGFB2","TGFBR1","TGFBR2","SMAD2","SMAD3","SMAD4","SMAD7","BMP2","BMP4"]),
        ("hsa04630", "JAK-STAT signaling", &["JAK1","JAK2","JAK3","TYK2","STAT1","STAT3","STAT5A","STAT5B","SOCS1","SOCS3"]),
        ("hsa04210", "Apoptosis", &["CASP3","CASP8","CASP9","BAX","BCL2","BID","CYCS","APAF1","XIAP","FAS"]),
        ("hsa03030", "DNA replication", &["MCM2","MCM3","MCM4","MCM5","MCM6","MCM7","PCNA","POLA1","POLE","RFC1"]),
        ("hsa03410", "Base excision repair", &["OGG1","MUTYH","UNG","APEX1","XRCC1","POLB","LIG3","PARP1","FEN1","NEIL1"]),
        ("hsa03420", "Nucleotide excision repair", &["XPA","XPB","XPC","XPD","XPF","XPG","ERCC1","DDB1","DDB2","CSA"]),
        ("hsa03430", "Mismatch repair", &["MLH1","MSH2","MSH6","PMS2","MSH3","EXO1","PCNA","RFC1","LIG1","POLD1"]),
        ("hsa04115", "p53 signaling", &["TP53","MDM2","MDM4","CDKN1A","BAX","PUMA","NOXA","GADD45A","SESN2","TIGAR"]),
    ]
}

// ── fetch_sra() — query SRA metadata via NCBI E-utilities ───────────

fn builtin_fetch_sra(args: &[Value]) -> Result<Value> {
    let accession = require_str(&args[0], "fetch_sra")?;

    // Search SRA
    let search_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=sra&term={}&retmode=json",
        url_encode(&accession)
    );
    let search_json = fetch_json(&search_url)?;
    let id_list = search_json
        .pointer("/esearchresult/idlist")
        .and_then(|v| v.as_array());
    let sra_id = match id_list.and_then(|arr| arr.first()).and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            let mut fields = HashMap::new();
            fields.insert("accession".to_string(), Value::Str(accession));
            fields.insert("error".to_string(), Value::Str("no SRA results found".to_string()));
            fields.insert("title".to_string(), Value::Nil);
            fields.insert("platform".to_string(), Value::Nil);
            fields.insert("strategy".to_string(), Value::Nil);
            fields.insert("source".to_string(), Value::Nil);
            fields.insert("organism".to_string(), Value::Nil);
            fields.insert("total_runs".to_string(), Value::Nil);
            fields.insert("total_bases".to_string(), Value::Nil);
            fields.insert("download_cmd".to_string(), Value::Nil);
            return Ok(Value::Record(fields));
        }
    };

    // Fetch summary
    let summary_url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=sra&id={sra_id}&retmode=json"
    );
    let summary_json = fetch_json(&summary_url)?;
    let data = summary_json
        .pointer(&format!("/result/{sra_id}"))
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let mut fields = HashMap::new();
    fields.insert("accession".to_string(), Value::Str(accession.clone()));
    fields.insert("title".to_string(), json_str(&data, "title"));

    // SRA expxml often contains embedded XML in a string field; extract what we can
    let expxml = data.get("expxml").and_then(|v| v.as_str()).unwrap_or("");

    // Extract platform from expxml
    let platform = extract_xml_attr(expxml, "Platform", "instrument_model")
        .or_else(|| extract_xml_tag(expxml, "Platform"))
        .unwrap_or_default();
    fields.insert(
        "platform".to_string(),
        if platform.is_empty() { Value::Nil } else { Value::Str(platform) },
    );

    // Extract strategy
    let strategy = extract_xml_tag(expxml, "Library_Strategy").unwrap_or_default();
    fields.insert(
        "strategy".to_string(),
        if strategy.is_empty() { Value::Nil } else { Value::Str(strategy) },
    );

    // Extract source
    let source = extract_xml_tag(expxml, "Library_Source").unwrap_or_default();
    fields.insert(
        "source".to_string(),
        if source.is_empty() { Value::Nil } else { Value::Str(source) },
    );

    // Extract organism
    let organism = extract_xml_attr(expxml, "Organism", "ScientificName")
        .or_else(|| extract_xml_tag(expxml, "Organism"))
        .unwrap_or_default();
    fields.insert(
        "organism".to_string(),
        if organism.is_empty() { Value::Nil } else { Value::Str(organism) },
    );

    // Runs info from the runs field
    let runs = data.get("runs").and_then(|v| v.as_str()).unwrap_or("");
    let run_count = runs.matches("<Run").count();
    fields.insert("total_runs".to_string(), Value::Int(run_count as i64));

    // Extract total bases from runs XML
    let total_bases = extract_xml_attr(runs, "Run", "total_bases")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    fields.insert(
        "total_bases".to_string(),
        if total_bases > 0 { Value::Int(total_bases) } else { Value::Nil },
    );

    // Build download command
    fields.insert(
        "download_cmd".to_string(),
        Value::Str(format!("fasterq-dump {} --split-3 -e 8", accession)),
    );

    Ok(Value::Record(fields))
}

/// Simple XML attribute extraction: find `<tag ... attr="value"` and return value
fn extract_xml_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let tag_start = xml.find(&format!("<{}", tag))?;
    let rest = &xml[tag_start..];
    let tag_end = rest.find('>')?;
    let tag_content = &rest[..tag_end];
    let attr_pattern = format!("{}=\"", attr);
    let attr_start = tag_content.find(&attr_pattern)?;
    let value_start = attr_start + attr_pattern.len();
    let value_end = tag_content[value_start..].find('"')?;
    Some(tag_content[value_start..value_start + value_end].to_string())
}

/// Simple XML tag content extraction: `<Tag>content</Tag>` → content
fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}", tag);
    let start_pos = xml.find(&open)?;
    let rest = &xml[start_pos..];
    let content_start = rest.find('>')? + 1;
    let content_end = rest.find(&close)?;
    if content_start < content_end {
        Some(rest[content_start..content_end].trim().to_string())
    } else {
        None
    }
}

// ── blast_remote() — submit to NCBI BLAST and return RID ────────────

fn builtin_blast_remote(args: &[Value]) -> Result<Value> {
    let sequence = get_seq_data_or_str(&args[0], "blast_remote")?;
    let database = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => "nt".to_string(),
    };

    // Determine program from sequence content
    let program = if sequence.chars().all(|c| matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'G' | 'C' | 'N' | 'U')) {
        "blastn"
    } else {
        "blastp"
    };

    // Submit BLAST request via PUT
    let submit_url = format!(
        "https://blast.ncbi.nlm.nih.gov/blast/Blast.cgi?CMD=Put&PROGRAM={}&DATABASE={}&QUERY={}",
        program,
        url_encode(&database),
        url_encode(&sequence),
    );

    let mut fields = HashMap::new();
    fields.insert("program".to_string(), Value::Str(program.to_string()));
    fields.insert("database".to_string(), Value::Str(database));
    fields.insert("query_length".to_string(), Value::Int(sequence.len() as i64));

    // Try to submit and parse the RID
    if let Some(result) = crate::csv::try_fetch_url(&submit_url) {
        match result {
            Ok(text) => {
                // Parse RID from the response (HTML/text format)
                // Look for: RID = XXXXXXXX
                let rid = text
                    .lines()
                    .find(|line| line.trim().starts_with("RID"))
                    .and_then(|line| {
                        line.split('=').nth(1).map(|s| s.trim().to_string())
                    })
                    .or_else(|| {
                        // Also try QBlastInfo format
                        text.find("RID = ").map(|pos| {
                            let start = pos + 6;
                            let end = text[start..]
                                .find('\n')
                                .map(|e| start + e)
                                .unwrap_or(text.len());
                            text[start..end].trim().to_string()
                        })
                    });

                match rid {
                    Some(rid) if !rid.is_empty() => {
                        fields.insert("rid".to_string(), Value::Str(rid.clone()));
                        fields.insert("submitted".to_string(), Value::Bool(true));
                        fields.insert(
                            "status_url".to_string(),
                            Value::Str(format!(
                                "https://blast.ncbi.nlm.nih.gov/blast/Blast.cgi?CMD=Get&RID={}&FORMAT_TYPE=JSON2",
                                rid
                            )),
                        );
                        fields.insert(
                            "results_url".to_string(),
                            Value::Str(format!(
                                "https://blast.ncbi.nlm.nih.gov/Blast.cgi?CMD=Get&RID={}&FORMAT_TYPE=Text",
                                rid
                            )),
                        );
                    }
                    _ => {
                        fields.insert("rid".to_string(), Value::Nil);
                        fields.insert("submitted".to_string(), Value::Bool(false));
                        fields.insert(
                            "error".to_string(),
                            Value::Str("could not parse RID from BLAST response".to_string()),
                        );
                        fields.insert("status_url".to_string(), Value::Nil);
                        fields.insert("results_url".to_string(), Value::Nil);
                    }
                }
            }
            Err(e) => {
                fields.insert("rid".to_string(), Value::Nil);
                fields.insert("submitted".to_string(), Value::Bool(false));
                fields.insert("error".to_string(), Value::Str(format!("BLAST submission failed: {e}")));
                fields.insert("status_url".to_string(), Value::Nil);
                fields.insert("results_url".to_string(), Value::Nil);
            }
        }
    } else {
        fields.insert("rid".to_string(), Value::Nil);
        fields.insert("submitted".to_string(), Value::Bool(false));
        fields.insert(
            "error".to_string(),
            Value::Str("no fetch hook available (network not configured)".to_string()),
        );
        fields.insert("status_url".to_string(), Value::Nil);
        fields.insert("results_url".to_string(), Value::Nil);
    }

    Ok(Value::Record(fields))
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

// ── scan_bio(text) — detect biological entities in text ──────────────

fn builtin_scan_bio(args: &[Value]) -> Result<Value> {
    let text = require_str(&args[0], "scan_bio")?;

    let mut genes = Vec::new();
    let mut variants = Vec::new();
    let mut accessions = Vec::new();
    let mut species = Vec::new();
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    // --- Gene detection ---
    let gene_set: HashSet<&str> = [
        "TP53", "BRCA1", "BRCA2", "PTEN", "RB1", "APC", "VHL", "KRAS", "NRAS", "BRAF",
        "PIK3CA", "EGFR", "ERBB2", "HER2", "ALK", "RET", "MET", "ROS1", "MYC", "MYCN",
        "CDKN2A", "CDK4", "CDK6", "MDM2", "ATM", "ATR", "CHEK2", "PALB2", "RAD51",
        "MLH1", "MSH2", "MSH6", "PMS2", "JAK2", "FLT3", "KIT", "ABL1", "BCR",
        "NOTCH1", "CTNNB1", "SMAD4", "STK11", "NF1", "NF2", "IDH1", "IDH2",
        "DNMT3A", "TET2", "NPM1", "RUNX1", "WT1", "TERT", "FGFR1", "FGFR2", "FGFR3",
        "MTOR", "AKT1", "EZH2", "ARID1A", "BCL2", "BCL6", "BTK", "CFTR", "HTT", "DMD",
        "GBA", "LRRK2", "SNCA", "APP", "APOE", "HBB", "MTHFR", "CYP2D6", "ACE2",
        "GAPDH", "ACTB", "VEGFA", "TNF", "IL6", "STAT3", "SOX2", "FOXL2", "SHH",
        "PDGFRA", "MAP2K1", "CCND1", "CCNE1", "PBRM1", "SETD2", "KDM6A",
        "TSC1", "TSC2", "BAP1", "SMARCB1", "DICER1", "POLE", "POLD1",
        "RAD51C", "RAD51D", "BRIP1", "BARD1", "FANCA", "FANCC", "NBN", "MRE11",
        "SMN1", "SMN2", "PKD1", "PKD2", "F5", "F2", "SERPINA1", "VKORC1",
        "TPMT", "NUDT15", "DPYD", "UGT1A1", "CYP2C19", "CYP3A4", "SLCO1B1",
        "PSEN1", "PSEN2", "MAPT", "GRN", "C9orf72", "SOD1", "FUS", "TARDBP",
        "HBA1", "HBA2", "G6PD", "GATA1", "GATA2", "CEBPA", "SPI1", "PAX5",
        "CD79A", "CD79B", "MYD88", "CARD11", "PLCG2", "IL2", "IL7", "IFNG",
        "TGFB1", "SMAD2", "SMAD3", "BMP4", "WNT1", "WNT3A", "GLI1", "PTCH1", "SMO",
        "TMPRSS2", "FMR1", "NAT2", "ABCB1",
    ]
    .iter()
    .copied()
    .collect();

    // Exclude common English words that look like gene symbols
    let exclude: HashSet<&str> = [
        "THE", "AND", "FOR", "NOT", "WITH", "FROM", "BUT", "ALL", "ARE", "WAS",
        "SET", "MAP", "LET", "RUN", "USE", "AGE", "END", "TOP", "CAN", "MAY",
        "HAS", "HAD", "BEEN", "HAVE", "THAT", "THIS", "WILL", "HER", "HIS",
    ]
    .iter()
    .copied()
    .collect();

    let gene_re = regex::Regex::new(r"\b([A-Z][A-Z0-9]{1,9})\b").unwrap();
    for cap in gene_re.captures_iter(&text) {
        let s = &cap[1];
        if gene_set.contains(s) && !exclude.contains(s) && seen.insert(format!("g:{s}")) {
            genes.push(Value::Str(s.to_string()));
        }
    }

    // --- Variant detection ---
    // rsIDs
    let rs_re = regex::Regex::new(r"(?i)\b(rs\d{3,12})\b").unwrap();
    for cap in rs_re.captures_iter(&text) {
        let id = cap[1].to_lowercase();
        if seen.insert(format!("v:{id}")) {
            variants.push(Value::Str(id));
        }
    }
    // HGVS notation
    let hgvs_re =
        regex::Regex::new(r"\b((?:NM_|NP_|NC_)\d+(?:\.\d+)?:[cpg]\.\w+)\b").unwrap();
    for cap in hgvs_re.captures_iter(&text) {
        let id = cap[1].to_string();
        if seen.insert(format!("v:{id}")) {
            variants.push(Value::Str(id));
        }
    }
    // ClinVar VCV IDs
    let vcv_re = regex::Regex::new(r"\b(VCV\d{6,12})\b").unwrap();
    for cap in vcv_re.captures_iter(&text) {
        let id = cap[1].to_string();
        if seen.insert(format!("v:{id}")) {
            variants.push(Value::Str(id));
        }
    }

    // --- Accession detection ---
    let acc_patterns: Vec<regex::Regex> = vec![
        regex::Regex::new(r"\b(GSE\d{3,8})\b").unwrap(),
        regex::Regex::new(r"\b(GSM\d{3,8})\b").unwrap(),
        regex::Regex::new(r"\b(SRR\d{5,10})\b").unwrap(),
        regex::Regex::new(r"\b(SRP\d{5,10})\b").unwrap(),
        regex::Regex::new(r"\b(SRX\d{5,10})\b").unwrap(),
        regex::Regex::new(r"\b(ERR\d{5,10})\b").unwrap(),
        regex::Regex::new(r"\b(PRJNA\d{4,8})\b").unwrap(),
        regex::Regex::new(r"\b(PRJEB\d{4,8})\b").unwrap(),
        regex::Regex::new(r"\b(SAMN\d{6,10})\b").unwrap(),
        regex::Regex::new(r"\b(ENSG\d{11})\b").unwrap(),
        regex::Regex::new(r"\b(ENST\d{11})\b").unwrap(),
        regex::Regex::new(r"\b(NM_\d{6,9}(?:\.\d+)?)\b").unwrap(),
        regex::Regex::new(r"\b(GCA_\d{9}\.\d)\b").unwrap(),
        regex::Regex::new(r"\b(GCF_\d{9}\.\d)\b").unwrap(),
    ];
    for re in &acc_patterns {
        for cap in re.captures_iter(&text) {
            let id = cap[1].to_string();
            if seen.insert(format!("a:{id}")) {
                accessions.push(Value::Str(id));
            }
        }
    }

    // --- Species detection ---
    let species_patterns: Vec<(&str, regex::Regex)> = vec![
        (
            "Human",
            regex::Regex::new(r"(?i)\b(?:Homo sapiens|human)\b").unwrap(),
        ),
        (
            "Mouse",
            regex::Regex::new(r"(?i)\b(?:Mus musculus|mouse)\b").unwrap(),
        ),
        (
            "Rat",
            regex::Regex::new(r"(?i)\b(?:Rattus norvegicus)\b").unwrap(),
        ),
        (
            "Fruit fly",
            regex::Regex::new(r"(?i)\b(?:Drosophila melanogaster)\b").unwrap(),
        ),
        (
            "C. elegans",
            regex::Regex::new(r"(?i)\b(?:C\. elegans|Caenorhabditis elegans)\b").unwrap(),
        ),
        (
            "Zebrafish",
            regex::Regex::new(r"(?i)\b(?:Danio rerio|zebrafish)\b").unwrap(),
        ),
        (
            "Yeast",
            regex::Regex::new(r"(?i)\b(?:Saccharomyces cerevisiae|budding yeast)\b").unwrap(),
        ),
        (
            "E. coli",
            regex::Regex::new(r"(?i)\b(?:E\. coli|Escherichia coli)\b").unwrap(),
        ),
        (
            "Arabidopsis",
            regex::Regex::new(r"(?i)\b(?:Arabidopsis thaliana)\b").unwrap(),
        ),
    ];
    for (name, re) in &species_patterns {
        if re.is_match(&text) && seen.insert(format!("s:{name}")) {
            species.push(Value::Str(name.to_string()));
        }
    }

    // --- DOI detection ---
    let doi_re = regex::Regex::new(r"\b(10\.\d{4,9}/[^\s]{5,50})\b").unwrap();
    for cap in doi_re.captures_iter(&text) {
        let id = cap[1].trim_end_matches(|c: char| c == '.' || c == ',' || c == ')' || c == ';').to_string();
        if seen.insert(format!("a:doi:{id}")) {
            accessions.push(Value::Str(id));
        }
    }

    // --- File URL detection (scan for bio file extensions in URLs) ---
    let url_re = regex::Regex::new(r"(https?://\S+\.(?:fasta|fa|fastq|fq|vcf|bed|gff|gtf|bam|sam|csv|tsv)(?:\.gz)?)").unwrap();
    for cap in url_re.captures_iter(&text) {
        let url = cap[1].to_string();
        if seen.insert(format!("f:{url}")) {
            files.push(Value::Str(url));
        }
    }

    let total = genes.len() + variants.len() + accessions.len() + species.len() + files.len();

    let mut result = HashMap::new();
    result.insert("genes".to_string(), Value::List(genes));
    result.insert("variants".to_string(), Value::List(variants));
    result.insert("accessions".to_string(), Value::List(accessions));
    result.insert("species".to_string(), Value::List(species));
    result.insert("files".to_string(), Value::List(files));
    result.insert("total".to_string(), Value::Int(total as i64));

    Ok(Value::Record(result))
}

// ── read_pdf(path) — extract text from a PDF file ───────────────────

fn builtin_read_pdf(args: &[Value]) -> Result<Value> {
    let _path = require_str(&args[0], "read_pdf")?;

    #[cfg(target_arch = "wasm32")]
    {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            "read_pdf() requires the CLI: bl -e 'read_pdf(\"file.pdf\")'".to_string(),
            None,
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new(&_path);
        if !path.exists() {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("read_pdf(): file not found: {_path}"),
                None,
            ));
        }
        if path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) != Some("pdf".to_string()) {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("read_pdf(): expected .pdf file, got: {_path}"),
                None,
            ));
        }

        #[cfg(feature = "native")]
        {
            let bytes = std::fs::read(path).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("read_pdf(): {e}"), None)
            })?;
            let text = pdf_extract::extract_text_from_mem(&bytes).map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("read_pdf(): failed to extract text: {e}"),
                    None,
                )
            })?;
            Ok(Value::Str(text))
        }

        #[cfg(not(feature = "native"))]
        {
            Err(BioLangError::runtime(
                ErrorKind::IOError,
                "read_pdf() requires native mode (not available in WASM)".to_string(),
                None,
            ))
        }
    }
}

fn get_seq_data_or_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
        Value::Str(s) => Ok(s.to_uppercase()),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires a sequence type or Str, got {}",
                other.type_of()
            ),
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

// ── cite(doi, [style]) — fetch citation from CrossRef and format ───────

fn builtin_cite(args: &[Value]) -> Result<Value> {
    let doi = require_str(&args[0], "cite")?;
    let style = if args.len() > 1 {
        require_str(&args[1], "cite")?.to_lowercase()
    } else {
        "all".to_string()
    };

    let url = format!("https://api.crossref.org/works/{}", doi);
    let json = fetch_json(&url)?;

    let msg = &json["message"];
    if msg.is_null() {
        return Err(BioLangError::runtime(ErrorKind::IOError, format!("cite: DOI not found: {doi}"), None));
    }

    let title = msg["title"].as_array()
        .and_then(|a| a.first())
        .and_then(|t| t.as_str())
        .unwrap_or("Untitled");

    let authors: Vec<String> = msg["author"].as_array()
        .map(|arr| arr.iter().map(|a| {
            let family = a["family"].as_str().unwrap_or("");
            let given = a["given"].as_str().unwrap_or("");
            if given.is_empty() { family.to_string() }
            else { format!("{family}, {}.", given.chars().next().unwrap_or(' ')) }
        }).collect())
        .unwrap_or_default();

    let year = msg["published-print"]["date-parts"].as_array()
        .or_else(|| msg["published-online"]["date-parts"].as_array())
        .or_else(|| msg["created"]["date-parts"].as_array())
        .and_then(|dp| dp.first())
        .and_then(|d| d.as_array())
        .and_then(|d| d.first())
        .and_then(|y| y.as_u64())
        .map(|y| y.to_string())
        .unwrap_or_else(|| "n.d.".to_string());

    let journal = msg["container-title"].as_array()
        .and_then(|a| a.first())
        .and_then(|j| j.as_str())
        .unwrap_or("");

    let volume = msg["volume"].as_str().unwrap_or("");
    let pages = msg["page"].as_str().unwrap_or("");
    let doi_str = msg["DOI"].as_str().unwrap_or(&doi);

    let author_str = if authors.is_empty() { "Unknown".to_string() }
        else if authors.len() <= 3 { authors.join(", ") }
        else { format!("{} et al.", authors[0]) };

    let full_authors = authors.join(" and ");

    let apa = format!(
        "{} ({}). {}. {}{}{}. https://doi.org/{}",
        author_str, year, title, journal,
        if !volume.is_empty() { format!(", {volume}") } else { String::new() },
        if !pages.is_empty() { format!(", {pages}") } else { String::new() },
        doi_str
    );

    let bibtex = format!(
        "@article{{{doi_key},\n  author = {{{full_authors}}},\n  title = {{{{{title}}}}},\n  journal = {{{journal}}},\n  year = {{{year}}},\n  volume = {{{volume}}},\n  pages = {{{pages}}},\n  doi = {{{doi_str}}}\n}}",
        doi_key = doi_str.replace('/', "_").replace('.', "_"),
    );

    let ris = format!(
        "TY  - JOUR\n{}TI  - {}\nJO  - {}\nVL  - {}\nSP  - {}\nPY  - {}\nDO  - {}\nER  -",
        authors.iter().map(|a| format!("AU  - {a}\n")).collect::<String>(),
        title, journal, volume, pages, year, doi_str
    );

    // Vancouver
    let vancouver = format!(
        "{}. {}. {}. {}{}{}.  doi:{}",
        author_str, title, journal, year,
        if !volume.is_empty() { format!(";{volume}") } else { String::new() },
        if !pages.is_empty() { format!(":{pages}") } else { String::new() },
        doi_str
    );

    // Harvard
    let harvard = format!(
        "{} ({}) '{}', {}{}{}.  https://doi.org/{}",
        author_str, year, title, journal,
        if !volume.is_empty() { format!(", {volume}") } else { String::new() },
        if !pages.is_empty() { format!(", pp. {pages}") } else { String::new() },
        doi_str
    );

    if style == "apa" {
        Ok(Value::Str(apa))
    } else if style == "vancouver" {
        Ok(Value::Str(vancouver))
    } else if style == "harvard" {
        Ok(Value::Str(harvard))
    } else if style == "bibtex" || style == "bib" {
        Ok(Value::Str(bibtex))
    } else if style == "ris" {
        Ok(Value::Str(ris))
    } else {
        let mut result = HashMap::new();
        result.insert("title".to_string(), Value::Str(title.to_string()));
        result.insert("authors".to_string(), Value::Str(full_authors));
        result.insert("year".to_string(), Value::Str(year));
        result.insert("journal".to_string(), Value::Str(journal.to_string()));
        result.insert("doi".to_string(), Value::Str(doi_str.to_string()));
        result.insert("apa".to_string(), Value::Str(apa));
        result.insert("vancouver".to_string(), Value::Str(vancouver));
        result.insert("harvard".to_string(), Value::Str(harvard));
        result.insert("bibtex".to_string(), Value::Str(bibtex));
        result.insert("ris".to_string(), Value::Str(ris));
        Ok(Value::Record(result))
    }
}
