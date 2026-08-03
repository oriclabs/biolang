//! Sequence motif analysis builtins.
//!
//! Functions: iupac_scan, pwm_from_seqs, pwm_scan, motif_consensus,
//!            motif_enrichment, gc_bias.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ──────────────────────────────────────────────────────────

pub fn motif_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("iupac_scan", Arity::Exact(2)),
        ("pwm_from_seqs", Arity::Exact(1)),
        ("pwm_scan", Arity::Range(2, 3)),
        ("motif_consensus", Arity::Exact(1)),
        ("motif_enrichment", Arity::Exact(3)),
        ("gc_bias", Arity::Range(1, 2)),
    ]
}

pub fn is_motif_builtin(name: &str) -> bool {
    matches!(
        name,
        "iupac_scan"
            | "pwm_from_seqs"
            | "pwm_scan"
            | "motif_consensus"
            | "motif_enrichment"
            | "gc_bias"
    )
}

pub fn call_motif_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "iupac_scan" => builtin_iupac_scan(args),
        "pwm_from_seqs" => builtin_pwm_from_seqs(args),
        "pwm_scan" => builtin_pwm_scan(args),
        "motif_consensus" => builtin_motif_consensus(args),
        "motif_enrichment" => builtin_motif_enrichment(args),
        "gc_bias" => builtin_gc_bias(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown motif builtin '{name}'"),
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

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
            None,
        )),
    }
}

/// IUPAC base → set of matching nucleotides (uppercase).
fn iupac_matches(ch: u8) -> &'static [u8] {
    match ch.to_ascii_uppercase() {
        b'A' => &[b'A'],
        b'C' => &[b'C'],
        b'G' => &[b'G'],
        b'T' => &[b'T'],
        b'R' => &[b'A', b'G'],
        b'Y' => &[b'C', b'T'],
        b'S' => &[b'G', b'C'],
        b'W' => &[b'A', b'T'],
        b'K' => &[b'G', b'T'],
        b'M' => &[b'A', b'C'],
        b'B' => &[b'C', b'G', b'T'],
        b'D' => &[b'A', b'G', b'T'],
        b'H' => &[b'A', b'C', b'T'],
        b'V' => &[b'A', b'C', b'G'],
        b'N' => &[b'A', b'C', b'G', b'T'],
        _ => &[],
    }
}

fn iupac_match_base(seq_base: u8, pattern_base: u8) -> bool {
    let sb = seq_base.to_ascii_uppercase();
    iupac_matches(pattern_base).contains(&sb)
}

/// Scan `seq` for all 0-based positions where `pattern` (IUPAC) matches.
fn scan_iupac(seq: &str, pattern: &str) -> Vec<i64> {
    let s = seq.as_bytes();
    let p = pattern.as_bytes();
    if p.is_empty() || p.len() > s.len() {
        return vec![];
    }
    let mut positions = Vec::new();
    'outer: for i in 0..=(s.len() - p.len()) {
        for j in 0..p.len() {
            if !iupac_match_base(s[i + j], p[j]) {
                continue 'outer;
            }
        }
        positions.push(i as i64);
    }
    positions
}

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// Log-factorial table for Fisher exact (recomputed inline, small values).
fn log_factorial(n: u64) -> f64 {
    (1..=n).map(|i| (i as f64).ln()).sum()
}

fn log_binom(n: u64, k: u64) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    log_factorial(n) - log_factorial(k) - log_factorial(n - k)
}

fn fisher_two_sided(a: u64, b: u64, c: u64, d: u64) -> f64 {
    let n = a + b + c + d;
    let r1 = a + b;
    let r2 = c + d;
    let k = a + c;
    let log_p_obs = log_binom(r1, a) + log_binom(r2, c) - log_binom(n, k);
    let max_a = r1.min(k);
    let p_sum: f64 = (0..=max_a)
        .map(|x| {
            let lp = log_binom(r1, x) + log_binom(r2, k.saturating_sub(x)) - log_binom(n, k);
            if lp.is_finite() && lp <= log_p_obs + 1e-10 {
                lp.exp()
            } else {
                0.0
            }
        })
        .sum::<f64>();
    p_sum.min(1.0)
}

// ── iupac_scan ────────────────────────────────────────────────────────

fn builtin_iupac_scan(args: Vec<Value>) -> Result<Value> {
    let seq = require_str(&args[0], "iupac_scan")?;
    let pattern = require_str(&args[1], "iupac_scan")?;
    let positions = scan_iupac(seq, pattern);
    Ok(Value::List(
        positions
            .into_iter()
            .map(Value::Int)
            .collect::<Vec<_>>()
            .into(),
    ))
}

// ── pwm_from_seqs ─────────────────────────────────────────────────────

fn builtin_pwm_from_seqs(args: Vec<Value>) -> Result<Value> {
    let seqs: Vec<String> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    "pwm_from_seqs() sequences must be List[Str]",
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "pwm_from_seqs() requires List[Str]",
                None,
            ))
        }
    };

    if seqs.is_empty() {
        return Ok(Value::Table(Table::new(
            vec![
                "pos".to_string(),
                "A".to_string(),
                "C".to_string(),
                "G".to_string(),
                "T".to_string(),
            ],
            vec![],
        )));
    }

    let motif_len = seqs[0].len();
    let n = seqs.len() as f64;

    // Count A/C/G/T at each position
    let mut counts: Vec<[f64; 4]> = vec![[0.0; 4]; motif_len];
    for seq in &seqs {
        for (i, b) in seq.bytes().enumerate().take(motif_len) {
            let idx = match b.to_ascii_uppercase() {
                b'A' => 0,
                b'C' => 1,
                b'G' => 2,
                b'T' => 3,
                _ => continue,
            };
            counts[i][idx] += 1.0;
        }
    }

    let pseudocount = 0.25;
    let denom = n + 1.0; // n seqs + pseudocount weight

    let rows: Vec<Vec<Value>> = counts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let weights: Vec<f64> = c
                .iter()
                .map(|&cnt| {
                    let prob = (cnt + pseudocount) / denom;
                    (prob / 0.25f64).log2()
                })
                .collect();
            vec![
                Value::Int(i as i64),
                Value::Float(weights[0]),
                Value::Float(weights[1]),
                Value::Float(weights[2]),
                Value::Float(weights[3]),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(
        vec![
            "pos".to_string(),
            "A".to_string(),
            "C".to_string(),
            "G".to_string(),
            "T".to_string(),
        ],
        rows,
    )))
}

// ── pwm_scan ──────────────────────────────────────────────────────────

fn builtin_pwm_scan(args: Vec<Value>) -> Result<Value> {
    let seq = require_str(&args[0], "pwm_scan")?;
    let pwm = require_table(&args[1], "pwm_scan")?;
    let threshold = if args.len() > 2 {
        to_f64(&args[2]).unwrap_or(0.8)
    } else {
        0.8
    };

    let seq_bytes = seq.as_bytes();
    let motif_len = pwm.rows.len();

    if motif_len == 0 || motif_len > seq_bytes.len() {
        return Ok(Value::Table(Table::new(
            vec![
                "start".to_string(),
                "end".to_string(),
                "score".to_string(),
                "fraction".to_string(),
                "subseq".to_string(),
            ],
            vec![],
        )));
    }

    // Find col indices for A, C, G, T
    let col_idx = |name: &str| pwm.columns.iter().position(|c| c == name);
    let ai = col_idx("A").unwrap_or(1);
    let ci = col_idx("C").unwrap_or(2);
    let gi = col_idx("G").unwrap_or(3);
    let ti = col_idx("T").unwrap_or(4);

    // Build weight matrix and max score
    let weights: Vec<[f64; 4]> = pwm
        .rows
        .iter()
        .map(|row| {
            let get = |i: usize| row.get(i).and_then(|v| to_f64(v)).unwrap_or(0.0);
            [get(ai), get(ci), get(gi), get(ti)]
        })
        .collect();

    let max_score: f64 = weights
        .iter()
        .map(|w| w.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
        .sum();
    if max_score <= 0.0 {
        return Ok(Value::Table(Table::new(
            vec![
                "start".to_string(),
                "end".to_string(),
                "score".to_string(),
                "fraction".to_string(),
                "subseq".to_string(),
            ],
            vec![],
        )));
    }

    let mut rows = Vec::new();
    for start in 0..=(seq_bytes.len() - motif_len) {
        let score: f64 = weights
            .iter()
            .enumerate()
            .map(|(j, w)| {
                let idx = match seq_bytes[start + j].to_ascii_uppercase() {
                    b'A' => 0,
                    b'C' => 1,
                    b'G' => 2,
                    b'T' => 3,
                    _ => return 0.0,
                };
                w[idx]
            })
            .sum();

        let fraction = score / max_score;
        if fraction >= threshold {
            let end = start + motif_len;
            let subseq = std::str::from_utf8(&seq_bytes[start..end])
                .unwrap_or("")
                .to_string();
            rows.push(vec![
                Value::Int(start as i64),
                Value::Int(end as i64),
                Value::Float(score),
                Value::Float(fraction),
                Value::Str(subseq),
            ]);
        }
    }

    Ok(Value::Table(Table::new(
        vec![
            "start".to_string(),
            "end".to_string(),
            "score".to_string(),
            "fraction".to_string(),
            "subseq".to_string(),
        ],
        rows,
    )))
}

// ── motif_consensus ───────────────────────────────────────────────────

fn builtin_motif_consensus(args: Vec<Value>) -> Result<Value> {
    let pwm = require_table(&args[0], "motif_consensus")?;

    let col_idx = |name: &str| pwm.columns.iter().position(|c| c == name);
    let ai = col_idx("A").unwrap_or(1);
    let ci = col_idx("C").unwrap_or(2);
    let gi = col_idx("G").unwrap_or(3);
    let ti = col_idx("T").unwrap_or(4);

    let mut consensus = String::new();
    for row in &pwm.rows {
        let get = |i: usize| row.get(i).and_then(|v| to_f64(v)).unwrap_or(0.0);
        let wa = get(ai);
        let wc = get(ci);
        let wg = get(gi);
        let wt = get(ti);

        let threshold = 0.0f64;
        let a = wa > threshold;
        let c = wc > threshold;
        let g = wg > threshold;
        let t = wt > threshold;

        let ch = match (a, c, g, t) {
            (true, false, false, false) => 'A',
            (false, true, false, false) => 'C',
            (false, false, true, false) => 'G',
            (false, false, false, true) => 'T',
            (true, false, true, false) => 'R',
            (false, true, false, true) => 'Y',
            (false, true, true, false) => 'S',
            (true, false, false, true) => 'W',
            (false, false, true, true) => 'K',
            (true, true, false, false) => 'M',
            (false, true, true, true) => 'B',
            (true, false, true, true) => 'D',
            (true, true, false, true) => 'H',
            (true, true, true, false) => 'V',
            _ => 'N',
        };
        consensus.push(ch);
    }

    Ok(Value::Str(consensus))
}

// ── motif_enrichment ──────────────────────────────────────────────────

fn builtin_motif_enrichment(args: Vec<Value>) -> Result<Value> {
    let fg_seqs: Vec<String> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    "motif_enrichment() fg_seqs must be List[Str]",
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "motif_enrichment() first arg must be List[Str]",
                None,
            ))
        }
    };
    let bg_seqs: Vec<String> = match &args[1] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    "motif_enrichment() bg_seqs must be List[Str]",
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "motif_enrichment() second arg must be List[Str]",
                None,
            ))
        }
    };
    let pattern = require_str(&args[2], "motif_enrichment")?;

    let fg_hits = fg_seqs
        .iter()
        .filter(|s| !scan_iupac(s, pattern).is_empty())
        .count() as u64;
    let fg_total = fg_seqs.len() as u64;
    let bg_hits = bg_seqs
        .iter()
        .filter(|s| !scan_iupac(s, pattern).is_empty())
        .count() as u64;
    let bg_total = bg_seqs.len() as u64;

    let fg_no = fg_total - fg_hits;
    let bg_no = bg_total - bg_hits;

    let p_value = fisher_two_sided(fg_hits, fg_no, bg_hits, bg_no);
    let odds_ratio = if fg_no > 0 && bg_hits > 0 {
        (fg_hits as f64 * bg_no as f64) / (fg_no as f64 * bg_hits as f64)
    } else {
        f64::INFINITY
    };

    let mut rec = HashMap::new();
    rec.insert("fg_hits".to_string(), Value::Int(fg_hits as i64));
    rec.insert("fg_total".to_string(), Value::Int(fg_total as i64));
    rec.insert("bg_hits".to_string(), Value::Int(bg_hits as i64));
    rec.insert("bg_total".to_string(), Value::Int(bg_total as i64));
    rec.insert("odds_ratio".to_string(), Value::Float(odds_ratio));
    rec.insert("p_value".to_string(), Value::Float(p_value));

    Ok(Value::Record((rec).into()))
}

// ── gc_bias ───────────────────────────────────────────────────────────

fn builtin_gc_bias(args: Vec<Value>) -> Result<Value> {
    let seq = require_str(&args[0], "gc_bias")?;
    let window: usize = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => (*n).max(1) as usize,
            _ => 100,
        }
    } else {
        100
    };

    let bytes = seq.as_bytes();
    let step = (window / 2).max(1);
    let mut rows = Vec::new();

    let mut start = 0usize;
    while start < bytes.len() {
        let end = (start + window).min(bytes.len());
        let slice = &bytes[start..end];
        let gc = slice
            .iter()
            .filter(|&&b| matches!(b, b'G' | b'C' | b'g' | b'c'))
            .count();
        let gc_frac = if slice.is_empty() {
            0.0
        } else {
            gc as f64 / slice.len() as f64
        };
        rows.push(vec![
            Value::Int(start as i64),
            Value::Int(end as i64),
            Value::Float(gc_frac),
        ]);
        if start + step >= bytes.len() {
            break;
        }
        start += step;
    }

    Ok(Value::Table(Table::new(
        vec![
            "start".to_string(),
            "end".to_string(),
            "gc_fraction".to_string(),
        ],
        rows,
    )))
}
