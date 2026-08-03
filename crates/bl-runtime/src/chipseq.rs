//! ChIP-seq / ATAC-seq analysis builtins.
//!
//! Functions: merge_peaks, consensus_peaks, frip_score,
//! tss_enrichment, peak_annotation.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn chipseq_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("merge_peaks", Arity::Exact(1)),
        ("consensus_peaks", Arity::Range(1, 2)),
        ("frip_score", Arity::Exact(3)),
        ("tss_enrichment", Arity::Exact(2)),
        ("peak_annotation", Arity::Exact(2)),
    ]
}

pub fn is_chipseq_builtin(name: &str) -> bool {
    matches!(
        name,
        "merge_peaks" | "consensus_peaks" | "frip_score" | "tss_enrichment" | "peak_annotation"
    )
}

pub fn call_chipseq_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "merge_peaks" => builtin_merge_peaks(args),
        "consensus_peaks" => builtin_consensus_peaks(args),
        "frip_score" => builtin_frip_score(args),
        "tss_enrichment" => builtin_tss_enrichment(args),
        "peak_annotation" => builtin_peak_annotation(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown chipseq builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
            None,
        )),
    }
}

fn col_index(table: &Table, name: &str, func: &str) -> Result<usize> {
    table.columns.iter().position(|c| c == name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("{func}(): column '{name}' not found"),
            None,
        )
    })
}

fn to_i64(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => 0,
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }
}

/// Extract peaks as (chrom, start, end) from a Table.
fn extract_peaks(table: &Table, func: &str) -> Result<Vec<(String, i64, i64)>> {
    let chrom_col = col_index(table, "chrom", func)?;
    let start_col = col_index(table, "start", func)?;
    let end_col = col_index(table, "end", func)?;

    Ok(table
        .rows
        .iter()
        .map(|row| {
            let chrom = match &row[chrom_col] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            };
            let start = to_i64(&row[start_col]);
            let end = to_i64(&row[end_col]);
            (chrom, start, end)
        })
        .collect())
}

/// Merge overlapping/adjacent intervals. Input must be sorted by (chrom, start).
fn merge_intervals(mut peaks: Vec<(String, i64, i64)>) -> Vec<(String, i64, i64, i64)> {
    peaks.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut merged: Vec<(String, i64, i64, i64)> = Vec::new();

    for (chrom, start, end) in peaks {
        if let Some(last) = merged.last_mut() {
            if last.0 == chrom && start <= last.2 {
                last.2 = last.2.max(end);
                last.3 += 1;
                continue;
            }
        }
        merged.push((chrom, start, end, 1));
    }
    merged
}

fn merged_to_table(merged: Vec<(String, i64, i64, i64)>) -> Value {
    let columns = ["chrom", "start", "end", "n_merged"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let rows = merged
        .into_iter()
        .map(|(c, s, e, n)| vec![Value::Str(c), Value::Int(s), Value::Int(e), Value::Int(n)])
        .collect();
    Value::Table(Table::new(columns, rows))
}

// ── merge_peaks ──────────────────────────────────────────────────────

fn builtin_merge_peaks(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "merge_peaks")?;
    let peaks = extract_peaks(table, "merge_peaks")?;
    let merged = merge_intervals(peaks);
    Ok(merged_to_table(merged))
}

// ── consensus_peaks ──────────────────────────────────────────────────

fn builtin_consensus_peaks(args: Vec<Value>) -> Result<Value> {
    let tables = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Table(t) => Ok(t.clone()),
                _ => Err(BioLangError::type_error(
                    "consensus_peaks() tables must be List<Table>",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        Value::Table(t) => vec![t.clone()],
        _ => {
            return Err(BioLangError::type_error(
                "consensus_peaks() first arg must be List<Table> or Table",
                None,
            ))
        }
    };

    let min_overlap = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n as usize,
            _ => 2,
        }
    } else {
        2
    };

    let n_samples = tables.len();

    // Collect all peaks across all samples and merge
    let mut all_peaks: Vec<(String, i64, i64)> = Vec::new();
    for table in &tables {
        let peaks = extract_peaks(table, "consensus_peaks")?;
        all_peaks.extend(peaks);
    }
    let merged = merge_intervals(all_peaks);

    // For each merged peak, count how many sample tables have a peak overlapping it
    let columns = ["chrom", "start", "end", "n_samples"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let rows: Vec<Vec<Value>> = merged
        .into_iter()
        .filter_map(|(chrom, start, end, _)| {
            let mut count = 0usize;
            for table in &tables {
                if let Ok(peaks) = extract_peaks(table, "consensus_peaks") {
                    let overlaps = peaks
                        .iter()
                        .any(|(c, s, e)| c == &chrom && *e > start && *s < end);
                    if overlaps {
                        count += 1;
                    }
                }
            }
            if count >= min_overlap.min(n_samples) {
                Some(vec![
                    Value::Str(chrom),
                    Value::Int(start),
                    Value::Int(end),
                    Value::Int(count as i64),
                ])
            } else {
                None
            }
        })
        .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── frip_score ───────────────────────────────────────────────────────

fn builtin_frip_score(args: Vec<Value>) -> Result<Value> {
    // peaks_table is arg[0] but unused for computation
    let bam_total = match &args[1] {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => {
            return Err(BioLangError::type_error(
                "frip_score() bam_read_count must be Int",
                None,
            ))
        }
    };
    let reads_in_peaks = match &args[2] {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => {
            return Err(BioLangError::type_error(
                "frip_score() reads_in_peaks must be Int",
                None,
            ))
        }
    };

    if bam_total == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::DivisionByZero,
            "frip_score(): bam_read_count is 0".to_string(),
            None,
        ));
    }

    Ok(Value::Float(reads_in_peaks as f64 / bam_total as f64))
}

// ── tss_enrichment ───────────────────────────────────────────────────

fn builtin_tss_enrichment(args: Vec<Value>) -> Result<Value> {
    let signal = match &args[0] {
        Value::List(l) => l.iter().map(to_f64).collect::<Vec<_>>(),
        _ => {
            return Err(BioLangError::type_error(
                "tss_enrichment() signal_values must be List",
                None,
            ))
        }
    };

    let flank = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => {
            return Err(BioLangError::type_error(
                "tss_enrichment() flank_size must be Int",
                None,
            ))
        }
    };

    let expected_len = 2 * flank + 1;
    if signal.len() < expected_len {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "tss_enrichment(): signal length {} < expected {}",
                signal.len(),
                expected_len
            ),
            None,
        ));
    }

    let center_start = flank.saturating_sub(100);
    let center_end = (flank + 100).min(signal.len());
    let center_mean = if center_end > center_start {
        signal[center_start..center_end].iter().sum::<f64>() / (center_end - center_start) as f64
    } else {
        0.0
    };

    let flank_size = 200.min(flank);
    let left: &[f64] = &signal[..flank_size];
    let right: &[f64] = &signal[signal.len().saturating_sub(flank_size)..];
    let flank_vals: Vec<f64> = left.iter().chain(right.iter()).copied().collect();
    let flank_mean = if flank_vals.is_empty() {
        1.0
    } else {
        flank_vals.iter().sum::<f64>() / flank_vals.len() as f64
    };

    Ok(Value::Float(if flank_mean == 0.0 {
        0.0
    } else {
        center_mean / flank_mean
    }))
}

// ── peak_annotation ──────────────────────────────────────────────────

fn builtin_peak_annotation(args: Vec<Value>) -> Result<Value> {
    let peaks_table = require_table(&args[0], "peak_annotation")?.clone();
    let genes_table = require_table(&args[1], "peak_annotation")?;

    let peak_chrom_col = col_index(&peaks_table, "chrom", "peak_annotation")?;
    let peak_start_col = col_index(&peaks_table, "start", "peak_annotation")?;
    let peak_end_col = col_index(&peaks_table, "end", "peak_annotation")?;

    let gene_chrom_col = col_index(genes_table, "chrom", "peak_annotation")?;
    let gene_tss_col = col_index(genes_table, "tss", "peak_annotation")?;
    let gene_name_col = col_index(genes_table, "gene_name", "peak_annotation")?;

    let mut out_columns = peaks_table.columns.clone();
    out_columns.push("nearest_gene".to_string());
    out_columns.push("dist_to_tss".to_string());

    let rows = peaks_table
        .rows
        .into_iter()
        .map(|mut peak_row| {
            let chrom = match &peak_row[peak_chrom_col] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            };
            let peak_mid =
                (to_i64(&peak_row[peak_start_col]) + to_i64(&peak_row[peak_end_col])) / 2;

            let mut nearest_gene = String::from(".");
            let mut min_dist = i64::MAX;

            for gene_row in &genes_table.rows {
                let gc = match &gene_row[gene_chrom_col] {
                    Value::Str(s) => s.as_str(),
                    _ => "",
                };
                if gc != chrom {
                    continue;
                }
                let tss = to_i64(&gene_row[gene_tss_col]);
                let dist = (peak_mid - tss).abs();
                if dist < min_dist {
                    min_dist = dist;
                    nearest_gene = match &gene_row[gene_name_col] {
                        Value::Str(s) => s.clone(),
                        _ => String::from("."),
                    };
                }
            }

            peak_row.push(Value::Str(nearest_gene));
            peak_row.push(Value::Int(if min_dist == i64::MAX { -1 } else { min_dist }));
            peak_row
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, rows)))
}
