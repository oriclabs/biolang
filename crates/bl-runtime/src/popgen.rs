//! Population genetics builtins.
//!
//! Functions: hwe_test, fst_weir_cockerham, tajima_d, ld_r2,
//!            allele_freq_spectrum, nucleotide_diversity.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn popgen_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("hwe_test", Arity::Exact(3)),
        ("fst_weir_cockerham", Arity::Exact(2)),
        ("tajima_d", Arity::Exact(2)),
        ("ld_r2", Arity::Exact(2)),
        ("allele_freq_spectrum", Arity::Range(1, 2)),
        ("nucleotide_diversity", Arity::Exact(1)),
    ]
}

pub fn is_popgen_builtin(name: &str) -> bool {
    matches!(
        name,
        "hwe_test"
            | "fst_weir_cockerham"
            | "tajima_d"
            | "ld_r2"
            | "allele_freq_spectrum"
            | "nucleotide_diversity"
    )
}

pub fn call_popgen_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "hwe_test" => builtin_hwe_test(args),
        "fst_weir_cockerham" => builtin_fst_weir_cockerham(args),
        "tajima_d" => builtin_tajima_d(args),
        "ld_r2" => builtin_ld_r2(args),
        "allele_freq_spectrum" => builtin_allele_freq_spectrum(args),
        "nucleotide_diversity" => builtin_nucleotide_diversity(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown popgen builtin: {name}"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn val_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

fn val_to_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Int(n) => Some(*n),
        Value::Float(f) => Some(*f as i64),
        _ => None,
    }
}

fn require_f64_list(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(l) => l
            .iter()
            .map(|v| {
                val_to_f64(v).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() list must contain numbers"), None)
                })
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List of numbers"),
            None,
        )),
    }
}

fn require_int_list(val: &Value, func: &str) -> Result<Vec<i64>> {
    match val {
        Value::List(l) => l
            .iter()
            .map(|v| {
                val_to_i64(v).ok_or_else(|| {
                    BioLangError::type_error(
                        format!("{func}() list must contain integers"),
                        None,
                    )
                })
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List of integers"),
            None,
        )),
    }
}

/// Chi-square CDF approximation via Wilson-Hilferty transformation.
fn chi2_cdf(x: f64, df: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let z = ((x / df).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * df))) / (2.0 / (9.0 * df)).sqrt();
    normal_cdf(z)
}

fn normal_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

/// Complementary error function approximation (Abramowitz & Stegun 7.1.26).
fn erfc(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t * (0.254829592
        + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    poly * (-x * x).exp()
}

// ── hwe_test(n_aa, n_ab, n_bb) → Table ───────────────────────────────
// Hardy-Weinberg equilibrium chi-square test.
// n_aa: homozygous reference, n_ab: heterozygous, n_bb: homozygous alt.

fn builtin_hwe_test(args: Vec<Value>) -> Result<Value> {
    let n_aa = val_to_i64(&args[0]).ok_or_else(|| {
        BioLangError::type_error("hwe_test() n_aa must be Int", None)
    })? as f64;
    let n_ab = val_to_i64(&args[1]).ok_or_else(|| {
        BioLangError::type_error("hwe_test() n_ab must be Int", None)
    })? as f64;
    let n_bb = val_to_i64(&args[2]).ok_or_else(|| {
        BioLangError::type_error("hwe_test() n_bb must be Int", None)
    })? as f64;
    let n = n_aa + n_ab + n_bb;
    if n == 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "hwe_test() total count must be > 0".to_string(),
            None,
        ));
    }
    let p = (2.0 * n_aa + n_ab) / (2.0 * n); // ref allele freq
    let q = 1.0 - p;
    let e_aa = p * p * n;
    let e_ab = 2.0 * p * q * n;
    let e_bb = q * q * n;
    let chi2 = chi2_term(n_aa, e_aa) + chi2_term(n_ab, e_ab) + chi2_term(n_bb, e_bb);
    let pvalue = 1.0 - chi2_cdf(chi2, 1.0);
    let rows = vec![vec![
        Value::Float(p),
        Value::Float(q),
        Value::Float(e_aa),
        Value::Float(e_ab),
        Value::Float(e_bb),
        Value::Float(chi2),
        Value::Float(pvalue),
    ]];
    Ok(Value::Table(Table::new(
        vec![
            "p_ref".to_string(),
            "q_alt".to_string(),
            "expected_aa".to_string(),
            "expected_ab".to_string(),
            "expected_bb".to_string(),
            "chi2".to_string(),
            "pvalue".to_string(),
        ],
        rows,
    )))
}

fn chi2_term(obs: f64, exp: f64) -> f64 {
    if exp < 1e-10 {
        0.0
    } else {
        (obs - exp).powi(2) / exp
    }
}

// ── fst_weir_cockerham(allele_counts_pop1, allele_counts_pop2) → Float ─
// Weir-Cockerham Fst averaged across variants.
// Each argument is a List of [n_ref, n_total] per variant (or Table with those cols).

fn builtin_fst_weir_cockerham(args: Vec<Value>) -> Result<Value> {
    let pop1 = parse_allele_counts(&args[0], "fst_weir_cockerham")?;
    let pop2 = parse_allele_counts(&args[1], "fst_weir_cockerham")?;
    if pop1.len() != pop2.len() || pop1.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "fst_weir_cockerham() both populations must have same number of variants".to_string(),
            None,
        ));
    }
    let mut num_sum = 0.0f64;
    let mut denom_sum = 0.0f64;
    let r = 2.0; // number of populations
    for ((n1_ref, n1_tot), (n2_ref, n2_tot)) in pop1.iter().zip(pop2.iter()) {
        let n1 = *n1_tot as f64;
        let n2 = *n2_tot as f64;
        if n1 < 2.0 || n2 < 2.0 {
            continue;
        }
        let p1 = *n1_ref as f64 / n1;
        let p2 = *n2_ref as f64 / n2;
        let n_bar = (n1 + n2) / r;
        let n_c = (n1 + n2) - (n1 * n1 + n2 * n2) / (n1 + n2);
        let p_bar = (n1 * p1 + n2 * p2) / (n1 + n2);
        // Sample variance of allele frequencies among pops
        let msp = (n1 * (p1 - p_bar).powi(2) + n2 * (p2 - p_bar).powi(2)) / (r - 1.0);
        // Within-population variance
        let msg = (n1 * p1 * (1.0 - p1) + n2 * p2 * (1.0 - p2)) / (n1 + n2 - r);
        let a = (msp - msg) / n_c;
        let b = msg;
        // Fst numerator and denominator
        num_sum += a;
        denom_sum += a + b;
        let _ = n_bar; // used implicitly via n_c
    }
    let fst = if denom_sum > 0.0 { num_sum / denom_sum } else { 0.0 };
    Ok(Value::Float(fst.clamp(0.0, 1.0)))
}

fn parse_allele_counts(val: &Value, func: &str) -> Result<Vec<(i64, i64)>> {
    match val {
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::List(pair) if pair.len() >= 2 => {
                    let n_ref = val_to_i64(&pair[0]).ok_or_else(|| {
                        BioLangError::type_error(
                            format!("{func}() n_ref must be Int"),
                            None,
                        )
                    })?;
                    let n_tot = val_to_i64(&pair[1]).ok_or_else(|| {
                        BioLangError::type_error(
                            format!("{func}() n_total must be Int"),
                            None,
                        )
                    })?;
                    Ok((n_ref, n_tot))
                }
                _ => Err(BioLangError::type_error(
                    format!("{func}() each element must be [n_ref, n_total]"),
                    None,
                )),
            })
            .collect(),
        Value::Table(t) => {
            // Expect columns: n_ref, n_total (or first two numeric cols)
            t.rows
                .iter()
                .map(|row| {
                    let n_ref = val_to_i64(row.get(0).unwrap_or(&Value::Int(0))).unwrap_or(0);
                    let n_tot = val_to_i64(row.get(1).unwrap_or(&Value::Int(0))).unwrap_or(0);
                    Ok((n_ref, n_tot))
                })
                .collect()
        }
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List of [n_ref, n_total] pairs or Table"),
            None,
        )),
    }
}

// ── tajima_d(seg_counts, n_sequences) → Float ────────────────────────
// Tajima's D statistic.
// seg_counts: List of pairwise differences per site (integers), or total S and π.
// n_sequences: number of haplotypes sampled.

fn builtin_tajima_d(args: Vec<Value>) -> Result<Value> {
    let seg_counts = require_int_list(&args[0], "tajima_d")?;
    let n = val_to_i64(&args[1]).ok_or_else(|| {
        BioLangError::type_error("tajima_d() n_sequences must be Int", None)
    })?;
    if n < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "tajima_d() n_sequences must be >= 2".to_string(),
            None,
        ));
    }
    let n = n as f64;
    // S = number of segregating sites (non-zero entries)
    let s = seg_counts.iter().filter(|&&c| c > 0).count() as f64;
    if s == 0.0 {
        return Ok(Value::Float(0.0));
    }
    // π = average pairwise differences
    let total_diffs: f64 = seg_counts.iter().map(|&c| c as f64).sum();
    let n_pairs = n * (n - 1.0) / 2.0;
    let pi = total_diffs / n_pairs;
    // Watterson's theta
    let a1: f64 = (1..n as usize).map(|i| 1.0 / i as f64).sum();
    let theta_w = s / a1;
    // Tajima's D denominator components
    let a2: f64 = (1..n as usize).map(|i| 1.0 / (i * i) as f64).sum();
    let b1 = (n + 1.0) / (3.0 * (n - 1.0));
    let b2 = 2.0 * (n * n + n + 3.0) / (9.0 * n * (n - 1.0));
    let c1 = b1 - 1.0 / a1;
    let c2 = b2 - (n + 2.0) / (a1 * n) + a2 / (a1 * a1);
    let e1 = c1 / a1;
    let e2 = c2 / (a1 * a1 + a2);
    let var_d = e1 * s + e2 * s * (s - 1.0);
    if var_d <= 0.0 {
        return Ok(Value::Float(0.0));
    }
    let d = (pi - theta_w) / var_d.sqrt();
    Ok(Value::Float(d))
}

// ── ld_r2(haplotype_a, haplotype_b) → Float ──────────────────────────
// Linkage disequilibrium r² between two biallelic SNP haplotypes.
// Each argument is a List of 0/1 integers (alleles across haplotypes).

fn builtin_ld_r2(args: Vec<Value>) -> Result<Value> {
    let a = require_int_list(&args[0], "ld_r2")?;
    let b = require_int_list(&args[1], "ld_r2")?;
    if a.len() != b.len() || a.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ld_r2() haplotype vectors must be non-empty and equal length".to_string(),
            None,
        ));
    }
    let n = a.len() as f64;
    let pa = a.iter().map(|&x| x as f64).sum::<f64>() / n; // freq of allele 1 at locus A
    let pb = b.iter().map(|&x| x as f64).sum::<f64>() / n; // freq of allele 1 at locus B
    // Frequency of haplotype (A=1, B=1)
    let p_ab = a
        .iter()
        .zip(b.iter())
        .filter(|(&ai, &bi)| ai == 1 && bi == 1)
        .count() as f64
        / n;
    // D = p_AB - p_A * p_B
    let d = p_ab - pa * pb;
    let denom = pa * (1.0 - pa) * pb * (1.0 - pb);
    if denom < 1e-12 {
        return Ok(Value::Float(0.0));
    }
    Ok(Value::Float((d * d / denom).min(1.0)))
}

// ── allele_freq_spectrum(allele_counts, n_sequences=None) → Table ─────
// Folded site frequency spectrum (SFS).
// allele_counts: List of derived allele counts per site.

fn builtin_allele_freq_spectrum(args: Vec<Value>) -> Result<Value> {
    let counts = require_int_list(&args[0], "allele_freq_spectrum")?;
    let n: i64 = if args.len() > 1 {
        val_to_i64(&args[1]).unwrap_or(0)
    } else {
        counts.iter().copied().max().unwrap_or(0) * 2
    };
    if n <= 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "allele_freq_spectrum() n_sequences must be > 0".to_string(),
            None,
        ));
    }
    let max_freq = (n / 2) as usize;
    let mut sfs = vec![0i64; max_freq + 1];
    for &c in &counts {
        if c <= 0 {
            continue;
        }
        // Fold: use minor allele count
        let minor = c.min(n - c) as usize;
        if minor <= max_freq {
            sfs[minor] += 1;
        }
    }
    let rows: Vec<Vec<Value>> = (1..=max_freq)
        .map(|i| {
            vec![
                Value::Int(i as i64),
                Value::Float(i as f64 / n as f64),
                Value::Int(sfs[i]),
            ]
        })
        .collect();
    Ok(Value::Table(Table::new(
        vec!["count".to_string(), "frequency".to_string(), "n_sites".to_string()],
        rows,
    )))
}

// ── nucleotide_diversity(sequences) → Float ───────────────────────────
// π = average pairwise nucleotide differences.
// sequences: List of Str (aligned DNA sequences of equal length).

fn builtin_nucleotide_diversity(args: Vec<Value>) -> Result<Value> {
    let seqs: Vec<Vec<u8>> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.bytes().collect()),
                Value::DNA(bio_seq) => Ok(bio_seq.to_string().into_bytes()),
                _ => Err(BioLangError::type_error(
                    "nucleotide_diversity() sequences must be Str or DNA",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "nucleotide_diversity() requires List of sequences",
                None,
            ))
        }
    };
    let n = seqs.len();
    if n < 2 {
        return Ok(Value::Float(0.0));
    }
    let len = seqs[0].len();
    if len == 0 {
        return Ok(Value::Float(0.0));
    }
    let mut total_diffs = 0u64;
    let mut comparisons = 0u64;
    for i in 0..n {
        for j in (i + 1)..n {
            let diffs = seqs[i]
                .iter()
                .zip(seqs[j].iter())
                .filter(|(a, b)| a != b && **a != b'-' && **b != b'-')
                .count() as u64;
            total_diffs += diffs;
            comparisons += 1;
        }
    }
    if comparisons == 0 {
        return Ok(Value::Float(0.0));
    }
    let pi = total_diffs as f64 / (comparisons as f64 * len as f64);
    Ok(Value::Float(pi))
}
