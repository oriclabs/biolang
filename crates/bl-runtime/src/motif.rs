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
        ("motif_profile", Arity::Range(1, 2)),
        ("motif_score", Arity::Exact(1)),
        ("profile_probability", Arity::Exact(2)),
        ("profile_most_probable", Arity::Exact(3)),
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
            | "motif_profile"
            | "motif_score"
            | "profile_probability"
            | "profile_most_probable"
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
        "motif_profile" => builtin_motif_profile(args),
        "motif_score" => builtin_motif_score(args),
        "profile_probability" => builtin_profile_probability(args),
        "profile_most_probable" => builtin_profile_most_probable(args),
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
        b'A' => b"A",
        b'C' => b"C",
        b'G' => b"G",
        b'T' => b"T",
        b'R' => b"AG",
        b'Y' => b"CT",
        b'S' => b"GC",
        b'W' => b"AT",
        b'K' => b"GT",
        b'M' => b"AC",
        b'B' => b"CGT",
        b'D' => b"AGT",
        b'H' => b"ACT",
        b'V' => b"ACG",
        b'N' => b"ACGT",
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
            let get = |i: usize| row.get(i).and_then(to_f64).unwrap_or(0.0);
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
        let get = |i: usize| row.get(i).and_then(to_f64).unwrap_or(0.0);
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

// ── Motif profiles ───────────────────────────────────────────────────
//
// The pieces every motif-finding algorithm is assembled from: turn a set of
// equal-length motifs into a position profile, score a candidate against it, and
// score the set itself. `pwm_from_seqs` above returns a Table for reporting;
// these return plain records because they sit inside loops that run thousands of
// times, and because a pseudocount is not optional in practice — without one, a
// base missing from a column makes every k-mer containing it impossible rather
// than merely unlikely.

/// The four bases, in the order every profile record uses.
const BASES: [char; 4] = ['A', 'C', 'G', 'T'];

fn motif_strings(value: &Value, func: &str) -> Result<Vec<String>> {
    let items = match value {
        Value::List(items) => items,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "{func}() requires a list of strings, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    let motifs: Vec<String> = items
        .iter()
        .map(|item| match item {
            Value::Str(s) => Ok(s.to_ascii_uppercase()),
            Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => {
                Ok(seq.data.to_ascii_uppercase())
            }
            other => Err(BioLangError::type_error(
                format!("{func}() motifs must be strings, got {}", other.type_of()),
                None,
            )),
        })
        .collect::<Result<_>>()?;
    if motifs.is_empty() {
        return Err(BioLangError::type_error(
            format!("{func}() needs at least one motif"),
            None,
        ));
    }
    let width = motifs[0].chars().count();
    if motifs.iter().any(|m| m.chars().count() != width) {
        return Err(BioLangError::type_error(
            format!("{func}() needs every motif to be the same length"),
            None,
        ));
    }
    Ok(motifs)
}

/// Count each base at each position.
fn base_counts(motifs: &[String]) -> Vec<[f64; 4]> {
    let width = motifs[0].chars().count();
    let mut counts = vec![[0.0f64; 4]; width];
    for motif in motifs {
        for (position, base) in motif.chars().enumerate() {
            if let Some(index) = BASES.iter().position(|&b| b == base) {
                counts[position][index] += 1.0;
            }
        }
    }
    counts
}

/// `motif_profile(motifs)` or `motif_profile(motifs, pseudocount)` — the
/// position profile of a set of equal-length motifs.
///
/// Returns one record per position, keyed by base. The pseudocount defaults to
/// zero, which is the textbook's first version of the algorithm; passing 1 gives
/// Laplace's rule of succession, which is the version that actually works.
fn builtin_motif_profile(args: Vec<Value>) -> Result<Value> {
    let motifs = motif_strings(&args[0], "motif_profile")?;
    let pseudocount = match args.get(1) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f64,
        Some(other) => {
            return Err(BioLangError::type_error(
                format!(
                    "motif_profile() pseudocount must be a number, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
        None => 0.0,
    };

    let rows = base_counts(&motifs)
        .into_iter()
        .map(|mut column| {
            for value in &mut column {
                *value += pseudocount;
            }
            let total: f64 = column.iter().sum();
            let fields: HashMap<String, Value> = BASES
                .iter()
                .zip(column)
                .map(|(base, count)| {
                    let share = if total > 0.0 { count / total } else { 0.0 };
                    (base.to_string(), Value::Float(share))
                })
                .collect();
            Value::Record(std::sync::Arc::new(fields))
        })
        .collect::<Vec<_>>();
    Ok(Value::List(rows.into()))
}

/// `motif_score(motifs)` — how far the motifs are from agreeing.
///
/// The number of entries differing from their column's most common base, summed
/// over every column. Zero means the motifs are identical, and lower is what
/// every motif search is trying to reach.
fn builtin_motif_score(args: Vec<Value>) -> Result<Value> {
    let motifs = motif_strings(&args[0], "motif_score")?;
    let depth = motifs.len() as f64;
    let score: f64 = base_counts(&motifs)
        .into_iter()
        .map(|column| depth - column.iter().copied().fold(0.0, f64::max))
        .sum();
    Ok(Value::Int(score as i64))
}

fn read_profile(value: &Value, func: &str) -> Result<Vec<HashMap<String, f64>>> {
    let positions = match value {
        Value::List(items) => items,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "{func}() requires a profile — a list of records, one per position, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    positions
        .iter()
        .map(|position| match position {
            Value::Map(fields) | Value::Record(fields) => Ok(fields
                .iter()
                .filter_map(|(base, value)| match value {
                    Value::Float(f) => Some((base.clone(), *f)),
                    Value::Int(i) => Some((base.clone(), *i as f64)),
                    _ => None,
                })
                .collect()),
            other => Err(BioLangError::type_error(
                format!(
                    "{func}() profile positions must be records, got {}",
                    other.type_of()
                ),
                None,
            )),
        })
        .collect()
}

fn probability_of(kmer: &str, profile: &[HashMap<String, f64>]) -> f64 {
    kmer.chars()
        .enumerate()
        .map(|(position, base)| {
            profile
                .get(position)
                .and_then(|column| column.get(&base.to_string()))
                .copied()
                .unwrap_or(0.0)
        })
        .product()
}

/// `profile_probability(kmer, profile)` — the chance the profile generates
/// `kmer`, which is one multiplication per position.
fn builtin_profile_probability(args: Vec<Value>) -> Result<Value> {
    let kmer = match &args[0] {
        Value::Str(s) => s.to_ascii_uppercase(),
        Value::DNA(seq) | Value::RNA(seq) => seq.data.to_ascii_uppercase(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "profile_probability() requires a string, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    let profile = read_profile(&args[1], "profile_probability")?;
    Ok(Value::Float(probability_of(&kmer, &profile)))
}

/// `profile_most_probable(text, k, profile)` — the k-mer of `text` the profile
/// is most likely to have produced.
///
/// Ties go to the leftmost, which is not a detail worth glossing: the textbook's
/// answers depend on it, and with a zero pseudocount most k-mers tie at zero.
fn builtin_profile_most_probable(args: Vec<Value>) -> Result<Value> {
    let text = match &args[0] {
        Value::Str(s) => s.to_ascii_uppercase(),
        Value::DNA(seq) | Value::RNA(seq) => seq.data.to_ascii_uppercase(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "profile_most_probable() requires a string, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    let k = match &args[1] {
        Value::Int(n) if *n > 0 => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "profile_most_probable() k must be a positive integer, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    let profile = read_profile(&args[2], "profile_most_probable")?;

    let characters: Vec<char> = text.chars().collect();
    if characters.len() < k {
        return Ok(Value::Str(String::new()));
    }
    let mut best = String::from_iter(&characters[0..k]);
    let mut best_probability = probability_of(&best, &profile);
    for start in 1..=characters.len() - k {
        let candidate = String::from_iter(&characters[start..start + k]);
        let probability = probability_of(&candidate, &profile);
        // Strictly greater, so the leftmost wins a tie.
        if probability > best_probability {
            best_probability = probability;
            best = candidate;
        }
    }
    Ok(Value::Str(best))
}

#[cfg(test)]
mod motif_profile_tests {
    use super::*;

    fn strings(items: &[&str]) -> Value {
        Value::List(
            items
                .iter()
                .map(|s| Value::Str((*s).to_string()))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    #[test]
    fn score_counts_disagreement() {
        assert_eq!(
            builtin_motif_score(vec![strings(&["ACG", "ACG"])]).unwrap(),
            Value::Int(0),
            "identical motifs agree everywhere"
        );
        assert_eq!(
            builtin_motif_score(vec![strings(&["ACG", "ACT"])]).unwrap(),
            Value::Int(1),
            "one column differs in one of two rows"
        );
    }

    #[test]
    fn a_pseudocount_makes_an_unseen_base_possible() {
        let motifs = strings(&["AAA", "AAA"]);
        let plain = builtin_motif_profile(vec![motifs.clone()]).unwrap();
        let smoothed = builtin_motif_profile(vec![motifs, Value::Int(1)]).unwrap();

        let column_of = |profile: &Value, base: &str| -> f64 {
            match profile {
                Value::List(items) => match &items[0] {
                    Value::Record(fields) => match fields.get(base) {
                        Some(Value::Float(f)) => *f,
                        other => panic!("expected a float, got {other:?}"),
                    },
                    other => panic!("expected a record, got {other:?}"),
                },
                other => panic!("expected a list, got {other:?}"),
            }
        };
        assert_eq!(column_of(&plain, "C"), 0.0, "unseen without a pseudocount");
        assert!(column_of(&smoothed, "C") > 0.0, "possible with one");
        assert_eq!(column_of(&smoothed, "A"), 3.0 / 6.0);
    }

    #[test]
    fn most_probable_breaks_ties_leftwards() {
        // A flat profile makes every k-mer equally likely, so the answer is
        // whichever comes first — which the textbook's answers depend on.
        let flat = builtin_motif_profile(vec![strings(&["AC", "GT"]), Value::Int(1)]).unwrap();
        let got =
            builtin_profile_most_probable(vec![Value::Str("ACGTAC".into()), Value::Int(2), flat])
                .unwrap();
        assert_eq!(got, Value::Str("AC".into()));
    }

    #[test]
    fn most_probable_finds_the_planted_kmer() {
        let profile = builtin_motif_profile(vec![strings(&["TTT", "TTT"]), Value::Int(1)]).unwrap();
        let got = builtin_profile_most_probable(vec![
            Value::Str("AAAGGGTTTCCC".into()),
            Value::Int(3),
            profile,
        ])
        .unwrap();
        assert_eq!(got, Value::Str("TTT".into()));
    }

    #[test]
    fn motifs_of_different_lengths_are_rejected() {
        let error = builtin_motif_score(vec![strings(&["AC", "ACG"])]).expect_err("ragged");
        assert!(error.to_string().contains("same length"), "{error}");
    }
}
