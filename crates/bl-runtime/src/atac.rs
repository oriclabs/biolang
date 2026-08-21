//! ATAC-seq specific quality control and fragment analysis builtins.
//!
//! Functions: fragment_size_dist, nfr_enrichment, nucleosome_fractions,
//! tss_enrichment_score, atac_qc, atac_fragment_qc, peak_matrix, peak_matrix_sparse,
//! atac_tfidf, atac_top_features, atac_depth_cor, gene_activity.
//!
//! Fragment counters return a cell x feature object shaped like
//! [`crate::singlecell`]'s `read_10x` result. Genome-scale peak analysis uses
//! `peak_matrix_sparse` followed by ATAC-specific TF-IDF and LSI; downstream
//! neighbours, Harmony, clustering, and UMAP reuse the single-cell workflow on
//! the resulting embedding.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

// ── Registry ─────────────────────────────────────────────────────────

pub fn atac_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("fragment_size_dist", Arity::Exact(1)),
        ("nfr_enrichment", Arity::Exact(1)),
        ("nucleosome_fractions", Arity::Exact(1)),
        ("tss_enrichment_score", Arity::Exact(3)),
        ("atac_qc", Arity::Exact(1)),
        ("atac_fragment_qc", Arity::Range(1, 3)),
        ("atac_tss_qc", Arity::Range(3, 4)),
        ("peak_matrix", Arity::Range(2, 3)),
        ("peak_matrix_sparse", Arity::Range(2, 3)),
        ("atac_tfidf", Arity::Range(1, 2)),
        ("atac_top_features", Arity::Range(1, 2)),
        ("atac_peak_qc", Arity::Range(2, 3)),
        ("atac_frip", Arity::Exact(2)),
        ("atac_ngs101_qc", Arity::Exact(5)),
        ("atac_detected_features", Arity::Exact(2)),
        ("atac_depth_cor", Arity::Range(2, 3)),
        ("atac_batch_mixing", Arity::Range(2, 3)),
        ("atac_filter_peaks", Arity::Range(1, 5)),
        ("gene_activity", Arity::Range(2, 5)),
    ]
}

pub fn is_atac_builtin(name: &str) -> bool {
    matches!(
        name,
        "fragment_size_dist"
            | "nfr_enrichment"
            | "nucleosome_fractions"
            | "tss_enrichment_score"
            | "atac_qc"
            | "atac_fragment_qc"
            | "atac_tss_qc"
            | "peak_matrix"
            | "peak_matrix_sparse"
            | "atac_tfidf"
            | "atac_top_features"
            | "atac_peak_qc"
            | "atac_frip"
            | "atac_ngs101_qc"
            | "atac_detected_features"
            | "atac_depth_cor"
            | "atac_batch_mixing"
            | "atac_filter_peaks"
            | "gene_activity"
    )
}

pub fn call_atac_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "fragment_size_dist" => builtin_fragment_size_dist(args),
        "nfr_enrichment" => builtin_nfr_enrichment(args),
        "nucleosome_fractions" => builtin_nucleosome_fractions(args),
        "tss_enrichment_score" => builtin_tss_enrichment_score(args),
        "atac_qc" => builtin_atac_qc(args),
        "atac_fragment_qc" => builtin_atac_fragment_qc(args),
        "atac_tss_qc" => builtin_atac_tss_qc(args),
        "peak_matrix" => builtin_peak_matrix(args),
        "peak_matrix_sparse" => builtin_peak_matrix_sparse(args),
        "atac_tfidf" => builtin_atac_tfidf(args),
        "atac_top_features" => builtin_atac_top_features(args),
        "atac_peak_qc" => builtin_atac_peak_qc(args),
        "atac_frip" => builtin_atac_frip(args),
        "atac_ngs101_qc" => builtin_atac_ngs101_qc(args),
        "atac_detected_features" => builtin_atac_detected_features(args),
        "atac_depth_cor" => builtin_atac_depth_cor(args),
        "atac_batch_mixing" => builtin_atac_batch_mixing(args),
        "atac_filter_peaks" => builtin_atac_filter_peaks(args),
        "gene_activity" => builtin_gene_activity(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown atac builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn to_i64(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        _ => 0,
    }
}

fn require_int_list(val: &Value, func: &str) -> Result<Vec<i64>> {
    match val {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n),
                Value::Float(f) => Ok(*f as i64),
                _ => Err(BioLangError::type_error(
                    format!("{func}() lengths must be List<Int>"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List of lengths"),
            None,
        )),
    }
}

// ── fragment_size_dist ───────────────────────────────────────────────

fn builtin_fragment_size_dist(args: Vec<Value>) -> Result<Value> {
    let lengths = require_int_list(&args[0], "fragment_size_dist")?;
    let total = lengths.len() as f64;
    if total == 0.0 {
        return Ok(Value::Table(Table::new(
            vec![
                "bin_start".into(),
                "bin_end".into(),
                "count".into(),
                "fraction".into(),
            ],
            vec![],
        )));
    }

    // 10-bp bins: [0,10), [10,20), ..., [990, 1000), [1000+]
    let n_bins = 100; // 0..1000 in steps of 10
    let mut counts = vec![0i64; n_bins + 1]; // +1 for overflow >= 1000

    for &len in &lengths {
        let len = len.max(0);
        if len >= 1000 {
            counts[n_bins] += 1;
        } else {
            let bin = (len / 10) as usize;
            counts[bin] += 1;
        }
    }

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n_bins {
        let start = (i * 10) as i64;
        let end = start + 10;
        let cnt = counts[i];
        rows.push(vec![
            Value::Int(start),
            Value::Int(end),
            Value::Int(cnt),
            Value::Float(cnt as f64 / total),
        ]);
    }
    // overflow bin
    let cnt = counts[n_bins];
    rows.push(vec![
        Value::Int(1000),
        Value::Int(i64::MAX),
        Value::Int(cnt),
        Value::Float(cnt as f64 / total),
    ]);

    Ok(Value::Table(Table::new(
        vec![
            "bin_start".into(),
            "bin_end".into(),
            "count".into(),
            "fraction".into(),
        ],
        rows,
    )))
}

// ── nfr_enrichment ───────────────────────────────────────────────────

fn builtin_nfr_enrichment(args: Vec<Value>) -> Result<Value> {
    let lengths = require_int_list(&args[0], "nfr_enrichment")?;
    let total = lengths.len() as f64;
    if total == 0.0 {
        return Ok(Value::Float(0.0));
    }

    let nfr_count = lengths.iter().filter(|&&l| l < 150).count() as f64;
    let mono_count = lengths.iter().filter(|&&l| (150..300).contains(&l)).count() as f64;

    if mono_count == 0.0 {
        return Ok(Value::Float(0.0));
    }

    Ok(Value::Float(nfr_count / mono_count))
}

// ── nucleosome_fractions ─────────────────────────────────────────────

fn builtin_nucleosome_fractions(args: Vec<Value>) -> Result<Value> {
    let lengths = require_int_list(&args[0], "nucleosome_fractions")?;
    let total = lengths.len() as f64;

    let mut sub_nfr = 0i64;
    let mut nfr = 0i64;
    let mut mono = 0i64;
    let mut di = 0i64;
    let mut tri = 0i64;
    let mut higher = 0i64;

    for &l in &lengths {
        match l {
            l if l < 100 => sub_nfr += 1,
            l if l < 150 => nfr += 1,
            l if l < 300 => mono += 1,
            l if l < 500 => di += 1,
            l if l < 750 => tri += 1,
            _ => higher += 1,
        }
    }

    let safe_frac = |n: i64| {
        if total == 0.0 {
            Value::Float(0.0)
        } else {
            Value::Float(n as f64 / total)
        }
    };

    let mut rec = HashMap::new();
    rec.insert("sub_nfr".to_string(), safe_frac(sub_nfr));
    rec.insert("nfr".to_string(), safe_frac(nfr));
    rec.insert("mono".to_string(), safe_frac(mono));
    rec.insert("di".to_string(), safe_frac(di));
    rec.insert("tri".to_string(), safe_frac(tri));
    rec.insert("higher".to_string(), safe_frac(higher));

    Ok(Value::Record((rec).into()))
}

// ── tss_enrichment_score ─────────────────────────────────────────────

fn builtin_tss_enrichment_score(args: Vec<Value>) -> Result<Value> {
    let lengths = require_int_list(&args[0], "tss_enrichment_score")?;
    let distances = require_int_list(&args[1], "tss_enrichment_score")?;
    let flank = to_i64(&args[2]).max(100);

    if lengths.len() != distances.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "tss_enrichment_score(): lengths ({}) and distances ({}) must have same length",
                lengths.len(),
                distances.len()
            ),
            None,
        ));
    }

    let bg_width = 100i64;
    let signal_half = 100i64;

    let mut signal_count = 0i64;
    let mut bg_count = 0i64;
    let mut signal_n = 0i64;
    let mut bg_n = 0i64;

    for (&len, &dist) in lengths.iter().zip(distances.iter()) {
        // Only use NFR fragments
        if len >= 150 {
            continue;
        }
        // Signal region: [-100, +100]
        if dist.abs() <= signal_half {
            signal_count += 1;
            signal_n += 1;
        }
        // Background: [-flank, -flank+bg_width]
        if dist >= -flank && dist <= -flank + bg_width {
            bg_count += 1;
            bg_n += 1;
        }
        let _ = signal_n;
        let _ = bg_n;
    }

    let signal_mean = signal_count as f64;
    let bg_mean = bg_count as f64;

    if bg_mean == 0.0 {
        return Ok(Value::Float(1.0));
    }

    let score = (signal_mean / bg_mean).max(1.0);
    Ok(Value::Float(score))
}

// ── atac_qc ──────────────────────────────────────────────────────────

fn builtin_atac_qc(args: Vec<Value>) -> Result<Value> {
    let lengths = require_int_list(&args[0], "atac_qc")?;
    let n = lengths.len();

    if n == 0 {
        let mut rec = HashMap::new();
        rec.insert("n_fragments".to_string(), Value::Int(0));
        rec.insert("nfr_fraction".to_string(), Value::Float(0.0));
        rec.insert("mono_fraction".to_string(), Value::Float(0.0));
        rec.insert("nfr_enrichment".to_string(), Value::Float(0.0));
        rec.insert("median_fragment_size".to_string(), Value::Float(0.0));
        rec.insert("fraction_large".to_string(), Value::Float(0.0));
        return Ok(Value::Record((rec).into()));
    }

    let total = n as f64;
    let nfr_count = lengths.iter().filter(|&&l| l < 150).count() as f64;
    let mono_count = lengths.iter().filter(|&&l| (150..300).contains(&l)).count() as f64;
    let large_count = lengths.iter().filter(|&&l| l >= 500).count() as f64;

    let nfr_enrichment = if mono_count == 0.0 {
        0.0
    } else {
        nfr_count / mono_count
    };

    let mut sorted = lengths.clone();
    sorted.sort_unstable();
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
    } else {
        sorted[n / 2] as f64
    };

    let mut rec = HashMap::new();
    rec.insert("n_fragments".to_string(), Value::Int(n as i64));
    rec.insert("nfr_fraction".to_string(), Value::Float(nfr_count / total));
    rec.insert(
        "mono_fraction".to_string(),
        Value::Float(mono_count / total),
    );
    rec.insert("nfr_enrichment".to_string(), Value::Float(nfr_enrichment));
    rec.insert("median_fragment_size".to_string(), Value::Float(median));
    rec.insert(
        "fraction_large".to_string(),
        Value::Float(large_count / total),
    );

    Ok(Value::Record((rec).into()))
}

// ── fragment counting: peak_matrix / gene_activity ───────────────────
//
// Both builtins walk a fragments file (or an in-memory Table) once, test each
// fragment against a bucketed interval index, and accumulate per-cell counts
// sparsely. Counts are densified only for the barcodes that survive filtering,
// so an unfiltered fragments file full of empty droplets costs memory
// proportional to observed counts rather than to barcodes x features.

/// Width of the interval index buckets. Genes and peaks are much smaller than
/// this, so a fragment touches one or two buckets in the common case.
const BIN_SIZE: i64 = 16_384;

/// Refuse to densify a matrix larger than this many entries (~800 MB at 8
/// bytes each). The scRNA object model is dense, so a genome-wide peak set has
/// to be subset before it can feed the pipeline.
const MAX_DENSE_ENTRIES: usize = 100_000_000;

/// One genomic interval and the output column it contributes to. Several
/// intervals may share a column — e.g. two GTF rows naming the same gene.
struct Region {
    start: i64,
    end: i64,
    col: u32,
}

/// Bucketed interval index, built once per call and queried once per fragment.
struct RegionIndex {
    /// chrom -> bin -> indices into `regions`
    bins: HashMap<String, HashMap<i64, Vec<u32>>>,
    regions: Vec<Region>,
    /// dedupe scratch: a region spanning several buckets must only count once
    seen: Vec<u32>,
    epoch: u32,
}

impl RegionIndex {
    fn new() -> Self {
        RegionIndex {
            bins: HashMap::new(),
            regions: Vec::new(),
            seen: Vec::new(),
            epoch: 0,
        }
    }

    fn push(&mut self, chrom: &str, start: i64, end: i64, col: u32) {
        if end <= start {
            return;
        }
        let idx = self.regions.len() as u32;
        self.regions.push(Region { start, end, col });
        let (lo, hi) = (start.div_euclid(BIN_SIZE), (end - 1).div_euclid(BIN_SIZE));
        let per_chrom = self.bins.entry(chrom.to_string()).or_default();
        for b in lo..=hi {
            per_chrom.entry(b).or_default().push(idx);
        }
    }

    fn finish(&mut self) {
        self.seen = vec![0; self.regions.len()];
    }

    /// Invoke `f(col)` once per distinct region overlapping `[start, end)`.
    /// A region spanning multiple buckets is reported once; two regions sharing
    /// a column are each reported, so a column can legitimately be hit twice.
    fn for_each_overlap(&mut self, chrom: &str, start: i64, end: i64, mut f: impl FnMut(u32)) {
        if end <= start {
            return;
        }
        let Some(per_chrom) = self.bins.get(chrom) else {
            return;
        };
        self.epoch += 1;
        let epoch = self.epoch;
        let (lo, hi) = (start.div_euclid(BIN_SIZE), (end - 1).div_euclid(BIN_SIZE));
        for b in lo..=hi {
            let Some(cands) = per_chrom.get(&b) else {
                continue;
            };
            for &ri in cands {
                let slot = &mut self.seen[ri as usize];
                if *slot == epoch {
                    continue;
                }
                *slot = epoch;
                let r = &self.regions[ri as usize];
                if start < r.end && r.start < end {
                    f(r.col);
                }
            }
        }
    }
}

/// Where fragments come from: a 10x `fragments.tsv[.gz]` path, or a Table with
/// chrom/start/end/barcode (and optional count) columns.
enum FragSource<'a> {
    Path(String),
    Table(&'a Table),
}

fn frag_source<'a>(v: &'a Value, func: &str) -> Result<FragSource<'a>> {
    match v {
        Value::Str(s) => Ok(FragSource::Path(s.to_string())),
        Value::Table(t) => Ok(FragSource::Table(t)),
        _ => Err(BioLangError::type_error(
            format!(
                "{func}() fragments must be a path Str or a Table with chrom/start/end/barcode"
            ),
            None,
        )),
    }
}

/// Stream fragments, invoking `f(chrom, start, end, barcode, count)`.
fn for_each_fragment(
    src: &FragSource,
    func: &str,
    max_lines: Option<usize>,
    mut f: impl FnMut(&str, i64, i64, &str, f64),
) -> Result<()> {
    match src {
        // Reading a fragments file needs filesystem + gzip, neither of which
        // exists in the browser build. Pass a Table instead there.
        #[cfg(not(feature = "native"))]
        FragSource::Path(path) => Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!(
                "{func}(): reading fragments from '{path}' needs the CLI;                  in the browser, pass a Table with chrom/start/end/barcode"
            ),
            None,
        )),
        #[cfg(feature = "native")]
        FragSource::Path(path) => {
            let file = std::fs::File::open(path).map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("{func}(): cannot open '{path}': {e}"),
                    None,
                )
            })?;
            let reader: Box<dyn BufRead> = if path.ends_with(".gz") {
                Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(file)))
            } else {
                Box::new(BufReader::new(file))
            };
            let mut valid_lines = 0usize;
            for line in reader.lines() {
                let line = line.map_err(|e| {
                    BioLangError::runtime(
                        ErrorKind::IOError,
                        format!("{func}(): read error in '{path}': {e}"),
                        None,
                    )
                })?;
                let line = line.trim_end();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut it = line.split('\t');
                let (Some(chrom), Some(start), Some(end), Some(bc)) =
                    (it.next(), it.next(), it.next(), it.next())
                else {
                    continue;
                };
                let (Ok(start), Ok(end)) = (start.parse::<i64>(), end.parse::<i64>()) else {
                    continue; // header or malformed line
                };
                if max_lines.is_some_and(|limit| valid_lines >= limit) {
                    break;
                }
                valid_lines += 1;
                let count = it.next().and_then(|c| c.parse::<f64>().ok()).unwrap_or(1.0);
                f(chrom, start, end, bc, count);
            }
            Ok(())
        }
        FragSource::Table(t) => {
            let ci_chrom = require_named_col(t, "chrom", func)?;
            let ci_start = require_named_col(t, "start", func)?;
            let ci_end = require_named_col(t, "end", func)?;
            let ci_bc = require_named_col(t, "barcode", func)?;
            let ci_count = t.columns.iter().position(|c| c == "count");
            for row in t.rows.iter().take(max_lines.unwrap_or(usize::MAX)) {
                let count = ci_count
                    .map(|i| match &row[i] {
                        Value::Int(n) => *n as f64,
                        Value::Float(x) => *x,
                        _ => 1.0,
                    })
                    .unwrap_or(1.0);
                f(
                    str_col(&row[ci_chrom]),
                    to_i64(&row[ci_start]),
                    to_i64(&row[ci_end]),
                    str_col(&row[ci_bc]),
                    count,
                );
            }
            Ok(())
        }
    }
}

fn require_named_col(t: &Table, name: &str, func: &str) -> Result<usize> {
    t.columns.iter().position(|c| c == name).ok_or_else(|| {
        BioLangError::type_error(
            format!(
                "{func}() requires column '{name}' (found: {})",
                t.columns.join(", ")
            ),
            None,
        )
    })
}

fn str_col(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn opt_barcode_list(v: Option<&Value>, func: &str) -> Result<Option<Vec<String>>> {
    match v {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::List(l)) => l
            .iter()
            .map(|x| match x {
                Value::Str(s) => Ok(s.to_string()),
                _ => Err(BioLangError::type_error(
                    format!("{func}() barcodes must be List<Str>"),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()
            .map(Some),
        Some(_) => Err(BioLangError::type_error(
            format!("{func}() barcodes must be List<Str>"),
            None,
        )),
    }
}

fn opt_positive_usize(v: Option<&Value>, func: &str, arg: &str) -> Result<Option<usize>> {
    match v {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::Int(n)) if *n > 0 => Ok(Some(*n as usize)),
        Some(_) => Err(BioLangError::type_error(
            format!("{func}() {arg} must be a positive Int or nil"),
            None,
        )),
    }
}

/// Signac-compatible per-cell nucleosome signal from a fragments file.
///
/// NucleosomeSignal samples the first `n` valid fragment rows, counts
/// nucleosome-free fragments (<147 bp) and mononucleosomal fragments
/// (147--294 bp, inclusive), then reports mono/NFR. The fifth fragments-file
/// column is PCR read support and is deliberately not used here: one row is
/// one unique fragment for this QC metric.
fn builtin_atac_fragment_qc(args: Vec<Value>) -> Result<Value> {
    let func = "atac_fragment_qc";
    let src = frag_source(&args[0], func)?;
    let whitelist = opt_barcode_list(args.get(1), func)?;
    let max_lines = opt_positive_usize(args.get(2), func, "max_lines")?;
    let allowed: Option<std::collections::HashSet<&str>> = whitelist
        .as_ref()
        .map(|items| items.iter().map(String::as_str).collect());

    #[derive(Default)]
    struct Counts {
        sampled: i64,
        nucleosome_free: i64,
        mononucleosomal: i64,
    }

    let mut counts: HashMap<String, Counts> = HashMap::new();
    for_each_fragment(
        &src,
        func,
        max_lines,
        |_chrom, start, end, barcode, _support| {
            if let Some(allowed) = &allowed {
                if !allowed.contains(barcode) {
                    return;
                }
            }
            let length = end.saturating_sub(start);
            let entry = counts.entry(barcode.to_string()).or_default();
            entry.sampled += 1;
            if length < 147 {
                entry.nucleosome_free += 1;
            } else if length <= 294 {
                entry.mononucleosomal += 1;
            }
        },
    )?;

    let barcodes = match whitelist {
        Some(items) => items,
        None => {
            let mut items: Vec<String> = counts.keys().cloned().collect();
            items.sort();
            items
        }
    };
    let signals: Vec<Option<f64>> = barcodes
        .iter()
        .map(|barcode| {
            let item = counts.get(barcode);
            let nfr = item.map_or(0, |value| value.nucleosome_free);
            (nfr > 0).then(|| item.map_or(0, |value| value.mononucleosomal) as f64 / nfr as f64)
        })
        .collect();
    let finite: Vec<f64> = signals.iter().filter_map(|value| *value).collect();

    let rows = barcodes
        .iter()
        .zip(signals.iter())
        .map(|(barcode, signal)| {
            let item = counts.get(barcode);
            let sampled = item.map_or(0, |value| value.sampled);
            let nfr = item.map_or(0, |value| value.nucleosome_free);
            let mono = item.map_or(0, |value| value.mononucleosomal);
            let (signal_value, percentile) = match signal {
                Some(signal) => {
                    let at_or_below = finite.iter().filter(|value| **value <= *signal).count();
                    let ecdf = if finite.is_empty() {
                        0.0
                    } else {
                        at_or_below as f64 / finite.len() as f64
                    };
                    (
                        Value::Float(*signal),
                        Value::Float((ecdf * 100.0).round() / 100.0),
                    )
                }
                None => (Value::Nil, Value::Nil),
            };
            vec![
                Value::Str(barcode.clone()),
                Value::Int(sampled),
                Value::Int(nfr),
                Value::Int(mono),
                signal_value,
                percentile,
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(
        vec![
            "barcode".into(),
            "fragments_sampled".into(),
            "nucleosome_free".into(),
            "mononucleosomal".into(),
            "nucleosome_signal".into(),
            "nucleosome_percentile".into(),
        ],
        rows,
    )))
}

/// Signac 1.17-compatible TSS enrichment for the `fast = FALSE` path used by
/// NGS101 Part 2. Each unique fragment contributes its two recorded insertion
/// coordinates. The slightly asymmetric interval endpoints below reproduce
/// Signac's GRanges-to-cut-matrix indexing, rather than an idealized window.
fn builtin_atac_tss_qc(args: Vec<Value>) -> Result<Value> {
    let func = "atac_tss_qc";
    let src = frag_source(&args[0], func)?;
    let Value::Table(tss) = &args[1] else {
        return Err(BioLangError::type_error(
            format!("{func}() TSS positions must be a Table"),
            None,
        ));
    };
    let Some(barcodes) = opt_barcode_list(args.get(2), func)? else {
        return Err(BioLangError::type_error(
            format!("{func}() requires a List<Str> of barcodes"),
            None,
        ));
    };
    let extension = match args.get(3) {
        None => 1000,
        Some(Value::Int(value)) if *value >= 500 => *value,
        Some(_) => {
            return Err(BioLangError::type_error(
                format!("{func}() region_extension must be an Int >= 500"),
                None,
            ))
        }
    };
    let ci_chrom = require_named_col(tss, "chrom", func)?;
    let ci_position = tss
        .columns
        .iter()
        .position(|column| matches!(column.as_str(), "position" | "tss"));
    let ci_start = tss.columns.iter().position(|column| column == "start");
    let ci_end = tss.columns.iter().position(|column| column == "end");
    let ci_strand = tss.columns.iter().position(|column| column == "strand");
    if ci_position.is_none() && (ci_start.is_none() || ci_end.is_none()) {
        return Err(BioLangError::type_error(
            format!("{func}() TSS table needs position/tss, or start/end columns"),
            None,
        ));
    }

    let mut center_index = RegionIndex::new();
    let mut flank_index = RegionIndex::new();
    for row in &tss.rows {
        let chrom = str_col(&row[ci_chrom]);
        if matches!(chrom, "chrM" | "MT" | "Mt") {
            continue;
        }
        let position = if let Some(column) = ci_position {
            to_i64(&row[column])
        } else {
            let start = to_i64(&row[ci_start.unwrap()]);
            let end = to_i64(&row[ci_end.unwrap()]);
            if ci_strand.is_some_and(|column| str_col(&row[column]) == "-") {
                end
            } else {
                start
            }
        };
        let minus_strand = ci_strand.is_some_and(|column| str_col(&row[column]) == "-");
        // TSSEnrichment(fast=FALSE) selects cut-matrix columns 500:1500.
        // SingleFileCutMatrix's coordinate transform makes these raw fragment
        // coordinates p-499..p+501 on +/* strands. Reversing the minus-strand
        // matrix makes the corresponding range p-497..p+503.
        let (center_start, center_end) = if minus_strand {
            (position.saturating_sub(497), position.saturating_add(504))
        } else {
            (position.saturating_sub(499), position.saturating_add(502))
        };
        center_index.push(chrom, center_start, center_end, 0);
        // The first and last 100 cut-matrix columns become these raw fragment
        // coordinate windows after Signac's start+1 transform.
        flank_index.push(
            chrom,
            position.saturating_sub(extension).saturating_add(2),
            position.saturating_sub(extension).saturating_add(102),
            0,
        );
        flank_index.push(
            chrom,
            position.saturating_add(extension).saturating_sub(97),
            position.saturating_add(extension).saturating_add(3),
            0,
        );
    }
    center_index.finish();
    flank_index.finish();

    let barcode_index: HashMap<&str, usize> = barcodes
        .iter()
        .enumerate()
        .map(|(index, barcode)| (barcode.as_str(), index))
        .collect();
    let mut center_counts = vec![0i64; barcodes.len()];
    let mut flank_counts = vec![0i64; barcodes.len()];
    for_each_fragment(&src, func, None, |chrom, start, end, barcode, _support| {
        let Some(&cell) = barcode_index.get(barcode) else {
            return;
        };
        if end <= start {
            return;
        }
        // Signac's SingleFileCutMatrix treats the two coordinates recorded in
        // fragments.tsv.gz as the insertion sites directly. In particular,
        // the second cut is `end`, not the last base of the half-open
        // fragment interval (`end - 1`).
        for cut in [start, end] {
            center_index.for_each_overlap(chrom, cut, cut + 1, |_| center_counts[cell] += 1);
            flank_index.for_each_overlap(chrom, cut, cut + 1, |_| flank_counts[cell] += 1);
        }
    })?;

    let flank_means: Vec<f64> = flank_counts
        .iter()
        .map(|count| *count as f64 / 200.0)
        .collect();
    let population_flank_mean = if flank_means.is_empty() {
        0.0
    } else {
        flank_means.iter().sum::<f64>() / flank_means.len() as f64
    };
    let scores: Vec<Option<f64>> = center_counts
        .iter()
        .zip(flank_means.iter())
        .map(|(center, flank)| {
            let denominator = if *flank == 0.0 {
                population_flank_mean
            } else {
                *flank
            };
            (denominator > 0.0).then(|| *center as f64 / denominator / 1001.0)
        })
        .collect();
    let finite: Vec<f64> = scores.iter().filter_map(|value| *value).collect();
    let rows = barcodes
        .iter()
        .enumerate()
        .map(|(index, barcode)| {
            let (score, percentile) = match scores[index] {
                Some(score) => {
                    let rank = finite.iter().filter(|value| **value <= score).count();
                    let ecdf = if finite.is_empty() {
                        0.0
                    } else {
                        rank as f64 / finite.len() as f64
                    };
                    (
                        Value::Float(score),
                        Value::Float((ecdf * 100.0).round() / 100.0),
                    )
                }
                None => (Value::Nil, Value::Nil),
            };
            vec![
                Value::Str(barcode.clone()),
                Value::Int(center_counts[index]),
                Value::Int(flank_counts[index]),
                score,
                percentile,
            ]
        })
        .collect();
    Ok(Value::Table(Table::new(
        vec![
            "barcode".into(),
            "tss_center_insertions".into(),
            "tss_flank_insertions".into(),
            "TSS.enrichment".into(),
            "TSS.percentile".into(),
        ],
        rows,
    )))
}

/// Count fragments into a cell x feature matrix and package it as an sc object
/// with the same field names `read_10x` produces.
fn count_into_object(
    src: &FragSource,
    func: &str,
    mut index: RegionIndex,
    feature_names: Vec<String>,
    whitelist: Option<Vec<String>>,
    sparse: bool,
) -> Result<Value> {
    index.finish();
    let n_features = feature_names.len();

    let allowed: Option<std::collections::HashSet<&str>> = whitelist
        .as_ref()
        .map(|w| w.iter().map(String::as_str).collect());

    let mut per_cell: HashMap<String, HashMap<u32, f64>> = HashMap::new();
    let mut cols_buf: Vec<u32> = Vec::new();
    let mut n_frag: u64 = 0;
    let mut n_overlapping: u64 = 0;

    for_each_fragment(src, func, None, |chrom, start, end, bc, count| {
        n_frag += 1;
        if let Some(a) = &allowed {
            if !a.contains(bc) {
                return;
            }
        }
        cols_buf.clear();
        index.for_each_overlap(chrom, start, end, |col| cols_buf.push(col));
        if cols_buf.is_empty() {
            return;
        }
        n_overlapping += 1;
        let cell = per_cell.entry(bc.to_string()).or_default();
        for &c in &cols_buf {
            *cell.entry(c).or_insert(0.0) += count;
        }
    })?;

    // Output barcodes: the whitelist in its given order (so callers can align
    // with an RNA object), otherwise every observed barcode, sorted.
    let barcodes: Vec<String> = match whitelist {
        Some(w) => w,
        None => {
            let mut b: Vec<String> = per_cell.keys().cloned().collect();
            b.sort();
            b
        }
    };

    let n_cells = barcodes.len();
    let entries = n_cells.saturating_mul(n_features);
    if !sparse && entries > MAX_DENSE_ENTRIES {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!(
                "{func}(): {n_cells} cells x {n_features} features is {entries} matrix entries, \
                 over the {MAX_DENSE_ENTRIES} limit. Subset first or use \
                 peak_matrix_sparse() for genome-scale peak matrices."
            ),
            None,
        ));
    }

    let matrix = if sparse {
        let mut indptr = Vec::with_capacity(n_cells + 1);
        let mut indices = Vec::new();
        let mut data = Vec::new();
        indptr.push(0);
        for bc in &barcodes {
            if let Some(counts) = per_cell.get(bc) {
                let mut row: Vec<(usize, f64)> = counts
                    .iter()
                    .filter_map(|(&column, &value)| {
                        (value != 0.0).then_some((column as usize, value))
                    })
                    .collect();
                row.sort_unstable_by_key(|&(column, _)| column);
                for (column, value) in row {
                    indices.push(column);
                    data.push(value);
                }
            }
            indptr.push(indices.len());
        }
        Value::SparseMatrix(
            SparseMatrix {
                indptr,
                indices,
                data,
                nrow: n_cells,
                ncol: n_features,
                row_names: Some(barcodes.clone()),
                col_names: Some(feature_names.clone()),
            }
            .into(),
        )
    } else {
        let rows: Vec<Value> = barcodes
            .iter()
            .map(|bc| {
                let mut dense = vec![0.0f64; n_features];
                if let Some(counts) = per_cell.get(bc) {
                    for (&col, &v) in counts {
                        dense[col as usize] += v;
                    }
                }
                Value::List(
                    dense
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                )
            })
            .collect();
        Value::List(rows.into())
    };

    let obs = Table::new(
        vec!["barcode".to_string()],
        barcodes
            .iter()
            .cloned()
            .map(|barcode| vec![Value::Str(barcode)])
            .collect(),
    );
    let var = Table::new(
        vec!["gene".to_string()],
        feature_names
            .iter()
            .cloned()
            .map(|gene| vec![Value::Str(gene)])
            .collect(),
    );
    let layers = HashMap::from([("counts".to_string(), matrix.clone())]);

    let mut rec: HashMap<String, Value> = HashMap::new();
    rec.insert("matrix".to_string(), matrix);
    rec.insert(
        "genes".to_string(),
        Value::List(
            feature_names
                .into_iter()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    rec.insert(
        "barcodes".to_string(),
        Value::List(
            barcodes
                .into_iter()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    rec.insert("n_cells".to_string(), Value::Int(n_cells as i64));
    rec.insert("n_genes".to_string(), Value::Int(n_features as i64));
    rec.insert("obs".to_string(), Value::Table(obs));
    rec.insert("var".to_string(), Value::Table(var));
    rec.insert("layers".to_string(), Value::Record(layers.into()));
    rec.insert("is_sparse".to_string(), Value::Bool(sparse));
    rec.insert("n_fragments".to_string(), Value::Int(n_frag as i64));
    rec.insert(
        "n_fragments_in_features".to_string(),
        Value::Int(n_overlapping as i64),
    );
    Ok(Value::Record((rec).into()))
}

// ── peak_matrix(fragments, peaks[, barcodes]) ────────────────────────

fn builtin_peak_matrix(args: Vec<Value>) -> Result<Value> {
    let func = "peak_matrix";
    let src = frag_source(&args[0], func)?;
    let peaks = match &args[1] {
        Value::Table(t) => t,
        _ => {
            return Err(BioLangError::type_error(
                format!("{func}() peaks must be a Table with chrom/start/end"),
                None,
            ))
        }
    };
    let whitelist = opt_barcode_list(args.get(2), func)?;

    let ci_chrom = require_named_col(peaks, "chrom", func)?;
    let ci_start = require_named_col(peaks, "start", func)?;
    let ci_end = require_named_col(peaks, "end", func)?;
    let ci_name = peaks
        .columns
        .iter()
        .position(|c| c == "name" || c == "peak" || c == "peak_id");

    let mut index = RegionIndex::new();
    let mut names: Vec<String> = Vec::with_capacity(peaks.rows.len());
    for (i, row) in peaks.rows.iter().enumerate() {
        let chrom = str_col(&row[ci_chrom]);
        let start = to_i64(&row[ci_start]);
        let end = to_i64(&row[ci_end]);
        let name = match ci_name {
            Some(ni) if !str_col(&row[ni]).is_empty() => str_col(&row[ni]).to_string(),
            _ => format!("{chrom}:{start}-{end}"),
        };
        index.push(chrom, start, end, i as u32);
        names.push(name);
    }

    count_into_object(&src, func, index, names, whitelist, false)
}

fn builtin_peak_matrix_sparse(args: Vec<Value>) -> Result<Value> {
    let func = "peak_matrix_sparse";
    let src = frag_source(&args[0], func)?;
    let peaks = match &args[1] {
        Value::Table(table) => table,
        _ => {
            return Err(BioLangError::type_error(
                format!("{func}() peaks must be a Table with chrom/start/end"),
                None,
            ))
        }
    };
    let whitelist = opt_barcode_list(args.get(2), func)?;
    let ci_chrom = require_named_col(peaks, "chrom", func)?;
    let ci_start = require_named_col(peaks, "start", func)?;
    let ci_end = require_named_col(peaks, "end", func)?;
    let ci_name = peaks
        .columns
        .iter()
        .position(|column| column == "name" || column == "peak" || column == "peak_id");
    let mut index = RegionIndex::new();
    let mut names = Vec::with_capacity(peaks.rows.len());
    for (row_index, row) in peaks.rows.iter().enumerate() {
        let chrom = str_col(&row[ci_chrom]);
        let start = to_i64(&row[ci_start]);
        let end = to_i64(&row[ci_end]);
        let name = match ci_name {
            Some(column) if !str_col(&row[column]).is_empty() => str_col(&row[column]).to_string(),
            _ => format!("{chrom}:{start}-{end}"),
        };
        index.push(chrom, start, end, row_index as u32);
        names.push(name);
    }
    count_into_object(&src, func, index, names, whitelist, true)
}

// BioLang matrices are cells x features, the transpose of Signac's internal
// feature x cell representation. This is Signac RunTFIDF method 1 exactly:
// log1p((count / total counts in cell) * (n_cells / total feature counts) * scale).
fn builtin_atac_tfidf(args: Vec<Value>) -> Result<Value> {
    let func = "atac_tfidf";
    let Value::SparseMatrix(matrix) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{func}() requires a SparseMatrix (use peak_matrix_sparse())"),
            None,
        ));
    };
    let scale = args
        .get(1)
        .map(to_f64_value)
        .transpose()?
        .unwrap_or(10_000.0);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(BioLangError::type_error(
            format!("{func}() scale_factor must be a positive finite number"),
            None,
        ));
    }
    let cell_totals = matrix.row_sums();
    let feature_totals = matrix.col_sums();
    let idf: Vec<f64> = feature_totals
        .iter()
        .map(|&total| {
            if total > 0.0 {
                matrix.nrow as f64 / total
            } else {
                0.0
            }
        })
        .collect();
    let mut output = (**matrix).clone();
    for row in 0..matrix.nrow {
        let total = cell_totals[row];
        for position in matrix.indptr[row]..matrix.indptr[row + 1] {
            let value = if total > 0.0 {
                (matrix.data[position] / total * idf[matrix.indices[position]] * scale).ln_1p()
            } else {
                0.0
            };
            output.data[position] = if value.is_finite() { value } else { 0.0 };
        }
    }
    Ok(Value::SparseMatrix(output.into()))
}

// Signac FindTopFeatures with a numeric min.cutoff retains features whose
// total count is strictly greater than the cutoff (not detection prevalence).
fn builtin_atac_top_features(args: Vec<Value>) -> Result<Value> {
    let func = "atac_top_features";
    let Value::SparseMatrix(matrix) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{func}() requires a SparseMatrix"),
            None,
        ));
    };
    let cutoff = args.get(1).map(to_f64_value).transpose()?.unwrap_or(0.0);
    if !cutoff.is_finite() || cutoff < 0.0 {
        return Err(BioLangError::type_error(
            format!("{func}() min_cutoff must be a non-negative finite number"),
            None,
        ));
    }
    Ok(Value::List(
        matrix
            .col_sums()
            .into_iter()
            .enumerate()
            .filter_map(|(index, total)| (total > cutoff).then_some(Value::Int(index as i64)))
            .collect::<Vec<_>>()
            .into(),
    ))
}

/// Per-cell matrix metrics used by Signac/NGS101 QC. Blacklist counts are the
/// counts in whole peak features that overlap any blacklist interval, divided
/// by all peak counts. This intentionally runs before blacklist peak removal.
fn builtin_atac_peak_qc(args: Vec<Value>) -> Result<Value> {
    let func = "atac_peak_qc";
    let Value::SparseMatrix(matrix) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{func}() requires a SparseMatrix"),
            None,
        ));
    };
    let Value::Table(peaks) = &args[1] else {
        return Err(BioLangError::type_error(
            format!("{func}() peaks must be a Table with chrom/start/end"),
            None,
        ));
    };
    if peaks.rows.len() != matrix.ncol {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!(
                "{func}(): {} peak rows do not match {} matrix columns",
                peaks.rows.len(),
                matrix.ncol
            ),
            None,
        ));
    }
    let ci_chrom = require_named_col(peaks, "chrom", func)?;
    let ci_start = require_named_col(peaks, "start", func)?;
    let ci_end = require_named_col(peaks, "end", func)?;

    let mut blacklist = RegionIndex::new();
    match args.get(2) {
        None | Some(Value::Nil) => {}
        Some(Value::Table(regions)) => {
            let bc = require_named_col(regions, "chrom", func)?;
            let bs = require_named_col(regions, "start", func)?;
            let be = require_named_col(regions, "end", func)?;
            for row in &regions.rows {
                blacklist.push(str_col(&row[bc]), to_i64(&row[bs]), to_i64(&row[be]), 0);
            }
        }
        Some(_) => {
            return Err(BioLangError::type_error(
                format!("{func}() blacklist must be a Table or nil"),
                None,
            ))
        }
    }
    blacklist.finish();
    let mut is_blacklisted = vec![false; matrix.ncol];
    for (column, row) in peaks.rows.iter().enumerate() {
        blacklist.for_each_overlap(
            str_col(&row[ci_chrom]),
            to_i64(&row[ci_start]),
            to_i64(&row[ci_end]),
            |_| is_blacklisted[column] = true,
        );
    }

    let rows = (0..matrix.nrow)
        .map(|row| {
            let start = matrix.indptr[row];
            let end = matrix.indptr[row + 1];
            let mut total = 0.0;
            let mut blacklist_total = 0.0;
            for position in start..end {
                let value = matrix.data[position];
                total += value;
                if is_blacklisted[matrix.indices[position]] {
                    blacklist_total += value;
                }
            }
            let barcode = matrix
                .row_names
                .as_ref()
                .and_then(|names| names.get(row).cloned())
                .unwrap_or_else(|| format!("cell-{}", row + 1));
            vec![
                Value::Str(barcode),
                Value::Float(total),
                Value::Int((end - start) as i64),
                Value::Float(blacklist_total),
                Value::Float(if total > 0.0 {
                    blacklist_total / total
                } else {
                    0.0
                }),
            ]
        })
        .collect();
    Ok(Value::Table(Table::new(
        vec![
            "barcode".into(),
            "nCount_peaks".into(),
            "nFeature_peaks".into(),
            "blacklist_peak_counts".into(),
            "blacklist_ratio".into(),
        ],
        rows,
    )))
}

fn builtin_atac_frip(args: Vec<Value>) -> Result<Value> {
    let func = "atac_frip";
    let as_numbers = |value: &Value, name: &str| -> Result<Vec<f64>> {
        let Value::List(values) = value else {
            return Err(BioLangError::type_error(
                format!("{func}() {name} must be a List<Number>"),
                None,
            ));
        };
        values.iter().map(to_f64_value).collect()
    };
    let peak_fragments = as_numbers(&args[0], "peak_region_fragments")?;
    let passed_filters = as_numbers(&args[1], "passed_filters")?;
    if peak_fragments.len() != passed_filters.len() {
        return Err(BioLangError::type_error(
            format!("{func}() inputs must have the same length"),
            None,
        ));
    }
    Ok(Value::List(
        peak_fragments
            .into_iter()
            .zip(passed_filters)
            .map(|(peak, total)| {
                if total > 0.0 {
                    Value::Float(peak / total * 100.0)
                } else {
                    Value::Nil
                }
            })
            .collect::<Vec<_>>()
            .into(),
    ))
}

fn numeric_list(value: &Value, func: &str, name: &str) -> Result<Vec<f64>> {
    let Value::List(values) = value else {
        return Err(BioLangError::type_error(
            format!("{func}() {name} must be a List<Number>"),
            None,
        ));
    };
    values.iter().map(to_f64_value).collect()
}

/// The six fixed cell filters printed in NGS101 Part 2. Keeping this named for
/// the article avoids presenting dataset-specific thresholds as general ATAC
/// defaults.
fn builtin_atac_ngs101_qc(args: Vec<Value>) -> Result<Value> {
    let func = "atac_ngs101_qc";
    let passed = numeric_list(&args[0], func, "passed_filters")?;
    let tss = numeric_list(&args[1], func, "TSS.enrichment")?;
    let frip = numeric_list(&args[2], func, "pct_reads_in_peaks")?;
    let nucleosome = numeric_list(&args[3], func, "nucleosome_signal")?;
    let blacklist = numeric_list(&args[4], func, "blacklist_ratio")?;
    let n = passed.len();
    if [tss.len(), frip.len(), nucleosome.len(), blacklist.len()]
        .iter()
        .any(|length| *length != n)
    {
        return Err(BioLangError::type_error(
            format!("{func}() all metric lists must have the same length"),
            None,
        ));
    }
    let mut failures = [0i64; 6];
    let mut keep = Vec::with_capacity(n);
    let mut indices = Vec::new();
    for index in 0..n {
        let failed = [
            passed[index] <= 3000.0,
            passed[index] >= 100_000.0,
            tss[index] <= 2.0,
            frip[index] <= 15.0,
            nucleosome[index] >= 4.0 || !nucleosome[index].is_finite(),
            blacklist[index] >= 0.05,
        ];
        for (failure, count) in failed.iter().zip(failures.iter_mut()) {
            if *failure {
                *count += 1;
            }
        }
        let retained = !failed.iter().any(|failure| *failure);
        keep.push(Value::Bool(retained));
        if retained {
            indices.push(Value::Int(index as i64));
        }
    }
    Ok(Value::Record(
        HashMap::from([
            ("keep".into(), Value::List(keep.into())),
            ("indices".into(), Value::List(indices.clone().into())),
            ("n_before".into(), Value::Int(n as i64)),
            ("n_after".into(), Value::Int(indices.len() as i64)),
            ("failed_low_depth".into(), Value::Int(failures[0])),
            ("failed_high_depth".into(), Value::Int(failures[1])),
            ("failed_low_tss".into(), Value::Int(failures[2])),
            ("failed_low_frip".into(), Value::Int(failures[3])),
            ("failed_high_nucleosome".into(), Value::Int(failures[4])),
            ("failed_high_blacklist".into(), Value::Int(failures[5])),
            (
                "profile".into(),
                Value::Str("ngs101_part2_severe_pbmc".into()),
            ),
        ])
        .into(),
    ))
}

fn builtin_atac_detected_features(args: Vec<Value>) -> Result<Value> {
    let func = "atac_detected_features";
    let Value::SparseMatrix(matrix) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{func}() requires a SparseMatrix"),
            None,
        ));
    };
    let min_cells = match &args[1] {
        Value::Int(number) if *number > 0 => *number as usize,
        _ => {
            return Err(BioLangError::type_error(
                format!("{func}() min_cells must be a positive Int"),
                None,
            ))
        }
    };
    let mut detected = vec![0usize; matrix.ncol];
    for row in 0..matrix.nrow {
        for position in matrix.indptr[row]..matrix.indptr[row + 1] {
            if matrix.data[position] > 0.0 {
                detected[matrix.indices[position]] += 1;
            }
        }
    }
    Ok(Value::List(
        detected
            .into_iter()
            .enumerate()
            .filter_map(|(index, cells)| (cells >= min_cells).then_some(Value::Int(index as i64)))
            .collect::<Vec<_>>()
            .into(),
    ))
}

fn to_f64_value(value: &Value) -> Result<f64> {
    match value {
        Value::Int(number) => Ok(*number as f64),
        Value::Float(number) => Ok(*number),
        _ => Err(BioLangError::type_error("expected a Number", None)),
    }
}

fn count_depths(value: &Value, func: &str) -> Result<Vec<f64>> {
    match value {
        Value::SparseMatrix(matrix) => Ok(matrix.row_sums()),
        Value::Matrix(matrix) => Ok(matrix
            .data
            .chunks(matrix.ncol.max(1))
            .map(|row| row.iter().sum())
            .collect()),
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::List(values) => values.iter().map(to_f64_value).sum(),
                _ => Err(BioLangError::type_error(
                    format!("{func}() counts must be a numeric matrix"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() counts must be a matrix"),
            None,
        )),
    }
}

fn embedding_columns(value: &Value, func: &str) -> Result<(usize, usize, Vec<f64>)> {
    match value {
        Value::Matrix(matrix) => Ok((matrix.nrow, matrix.ncol, matrix.data.clone())),
        Value::List(rows) => {
            let nrow = rows.len();
            let ncol = rows
                .first()
                .and_then(|row| match row {
                    Value::List(values) => Some(values.len()),
                    _ => None,
                })
                .unwrap_or(0);
            let mut data = Vec::with_capacity(nrow * ncol);
            for row in rows.iter() {
                let Value::List(values) = row else {
                    return Err(BioLangError::type_error(
                        format!("{func}() embedding must be a numeric matrix"),
                        None,
                    ));
                };
                if values.len() != ncol {
                    return Err(BioLangError::type_error(
                        format!("{func}() embedding rows must have equal length"),
                        None,
                    ));
                }
                for value in values.iter() {
                    data.push(to_f64_value(value)?);
                }
            }
            Ok((nrow, ncol, data))
        }
        _ => Err(BioLangError::type_error(
            format!("{func}() embedding must be a matrix"),
            None,
        )),
    }
}

fn builtin_atac_depth_cor(args: Vec<Value>) -> Result<Value> {
    let func = "atac_depth_cor";
    let depths = count_depths(&args[0], func)?;
    let (nrow, ncol, embedding) = embedding_columns(&args[1], func)?;
    if depths.len() != nrow {
        return Err(BioLangError::type_error(
            format!(
                "{func}() has {} cells in counts but {nrow} rows in the embedding",
                depths.len()
            ),
            None,
        ));
    }
    let wanted = match args.get(2) {
        None => ncol,
        Some(value) => {
            let number = to_f64_value(value)?;
            if !number.is_finite() || number < 0.0 {
                return Err(BioLangError::type_error(
                    format!("{func}() n_components must be a non-negative finite number"),
                    None,
                ));
            }
            (number as usize).min(ncol)
        }
    };
    let mean_depth = if nrow == 0 {
        0.0
    } else {
        depths.iter().sum::<f64>() / nrow as f64
    };
    let depth_ss = depths
        .iter()
        .map(|value| (value - mean_depth).powi(2))
        .sum::<f64>();
    let mut correlations = Vec::with_capacity(wanted);
    for column in 0..wanted {
        let mean = if nrow == 0 {
            0.0
        } else {
            (0..nrow)
                .map(|row| embedding[row * ncol + column])
                .sum::<f64>()
                / nrow as f64
        };
        let mut cross = 0.0;
        let mut component_ss = 0.0;
        for row in 0..nrow {
            let centered = embedding[row * ncol + column] - mean;
            cross += (depths[row] - mean_depth) * centered;
            component_ss += centered * centered;
        }
        let denominator = (depth_ss * component_ss).sqrt();
        correlations.push(Value::Float(if denominator > 0.0 {
            cross / denominator
        } else {
            0.0
        }));
    }
    Ok(Value::Record(
        HashMap::from([
            ("correlations".to_string(), Value::List(correlations.into())),
            ("n_components".to_string(), Value::Int(wanted as i64)),
            (
                "depth_metric".to_string(),
                Value::Str("total_counts".to_string()),
            ),
        ])
        .into(),
    ))
}

/// Mean fraction of neighbours belonging to another batch, matching the
/// NGS101 scATAC tutorial's `mixing_score()` helper. Its `k` includes the query
/// cell itself, so the comparison uses `k - 1` actual neighbours. The random
/// ceiling is `1 - sum(batch_proportion^2)` and therefore reflects imbalance.
fn builtin_atac_batch_mixing(args: Vec<Value>) -> Result<Value> {
    let func = "atac_batch_mixing";
    let (n_cells, n_dimensions, flat) = embedding_columns(&args[0], func)?;
    let labels: Vec<String> = match &args[1] {
        Value::List(values) => values.iter().map(|value| format!("{value}")).collect(),
        _ => {
            return Err(BioLangError::type_error(
                format!("{func}() batch_ids must be a List"),
                None,
            ))
        }
    };
    if labels.len() != n_cells {
        return Err(BioLangError::type_error(
            format!(
                "{func}() has {} batch labels but {n_cells} embedding rows",
                labels.len()
            ),
            None,
        ));
    }
    if n_cells < 2 || n_dimensions == 0 {
        return Err(BioLangError::type_error(
            format!("{func}() needs at least two cells and one embedding dimension"),
            None,
        ));
    }
    let k = match args.get(2) {
        None => 30.min(n_cells),
        Some(value) => {
            let number = to_f64_value(value)?;
            if !number.is_finite() || number < 2.0 {
                return Err(BioLangError::type_error(
                    format!("{func}() k must be a finite number of at least 2"),
                    None,
                ));
            }
            (number as usize).min(n_cells)
        }
    };
    let neighbour_count = k.saturating_sub(1).min(n_cells - 1);
    let embedding: Vec<Vec<f64>> = flat.chunks(n_dimensions).map(|row| row.to_vec()).collect();
    let neighbours =
        crate::singlecell::neighbour_rows_metric(&embedding, neighbour_count, "euclidean");

    let mut batch_order: Vec<String> = Vec::new();
    let batch_of: Vec<usize> = labels
        .iter()
        .map(|label| {
            batch_order
                .iter()
                .position(|known| known == label)
                .unwrap_or_else(|| {
                    batch_order.push(label.clone());
                    batch_order.len() - 1
                })
        })
        .collect();
    let n_batches = batch_order.len();
    let per_cell: Vec<f64> = neighbours
        .iter()
        .enumerate()
        .map(|(cell, row)| {
            let other_batches = row
                .iter()
                .filter(|&&(neighbour, _)| batch_of[neighbour] != batch_of[cell])
                .count();
            other_batches as f64 / row.len().max(1) as f64
        })
        .collect();
    let mean = per_cell.iter().sum::<f64>() / per_cell.len().max(1) as f64;
    let mut global_counts = vec![0usize; n_batches];
    for batch in batch_of {
        global_counts[batch] += 1;
    }
    let random_same_batch = global_counts
        .into_iter()
        .map(|count| (count as f64 / n_cells as f64).powi(2))
        .sum::<f64>();
    let expected_random = 1.0 - random_same_batch;
    let fraction_of_random = if expected_random > 0.0 {
        mean / expected_random
    } else {
        0.0
    };
    Ok(Value::Record(
        HashMap::from([
            ("mixing_score".to_string(), Value::Float(mean)),
            (
                "expected_random_mixing".to_string(),
                Value::Float(expected_random),
            ),
            (
                "fraction_of_random".to_string(),
                Value::Float(fraction_of_random),
            ),
            (
                "per_cell".to_string(),
                Value::List(
                    per_cell
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("k".to_string(), Value::Int(k as i64)),
            (
                "n_compared_neighbours".to_string(),
                Value::Int(neighbour_count as i64),
            ),
            ("n_batches".to_string(), Value::Int(n_batches as i64)),
            (
                "metric".to_string(),
                Value::Str("fraction_other_batch".to_string()),
            ),
        ])
        .into(),
    ))
}

fn is_human_standard_chromosome(chrom: &str) -> bool {
    let bare = chrom.strip_prefix("chr").unwrap_or(chrom);
    matches!(bare, "X" | "Y" | "M" | "MT")
        || bare
            .parse::<u8>()
            .map(|number| (1..=22).contains(&number))
            .unwrap_or(false)
}

/// Width/standard-chromosome/blacklist filtering used after constructing the
/// union peak set. A blacklist overlap removes the whole peak, matching
/// subsetByOverlaps(..., invert=TRUE); it does not split a peak around the mask.
fn builtin_atac_filter_peaks(args: Vec<Value>) -> Result<Value> {
    let func = "atac_filter_peaks";
    let Value::Table(peaks) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{func}() peaks must be a Table"),
            None,
        ));
    };
    let min_width = args.get(1).map(to_f64_value).transpose()?.unwrap_or(20.0);
    let max_width = args
        .get(2)
        .map(to_f64_value)
        .transpose()?
        .unwrap_or(10_000.0);
    if !min_width.is_finite() || !max_width.is_finite() {
        return Err(BioLangError::type_error(
            format!("{func}() width bounds must be finite numbers"),
            None,
        ));
    }
    if min_width < 0.0 || max_width <= min_width {
        return Err(BioLangError::type_error(
            format!("{func}() requires 0 <= min_width < max_width"),
            None,
        ));
    }
    let standard_only = match args.get(4) {
        None | Some(Value::Nil) => true,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return Err(BioLangError::type_error(
                format!("{func}() standard_chromosomes must be Bool"),
                None,
            ))
        }
    };
    let mut blacklist = RegionIndex::new();
    if let Some(value) = args.get(3) {
        match value {
            Value::Nil => {}
            Value::Table(table) => {
                let chrom_column = require_named_col(table, "chrom", func)?;
                let start_column = require_named_col(table, "start", func)?;
                let end_column = require_named_col(table, "end", func)?;
                for row in &table.rows {
                    blacklist.push(
                        str_col(&row[chrom_column]),
                        to_i64(&row[start_column]),
                        to_i64(&row[end_column]),
                        0,
                    );
                }
            }
            _ => {
                return Err(BioLangError::type_error(
                    format!("{func}() blacklist must be a Table or nil"),
                    None,
                ))
            }
        }
    }
    blacklist.finish();
    let chrom_column = require_named_col(peaks, "chrom", func)?;
    let start_column = require_named_col(peaks, "start", func)?;
    let end_column = require_named_col(peaks, "end", func)?;
    let mut rows = Vec::new();
    for row in &peaks.rows {
        let chrom = str_col(&row[chrom_column]);
        let start = to_i64(&row[start_column]);
        let end = to_i64(&row[end_column]);
        let width = (end - start) as f64;
        if width <= min_width || width >= max_width {
            continue;
        }
        if standard_only && !is_human_standard_chromosome(chrom) {
            continue;
        }
        let mut blocked = false;
        blacklist.for_each_overlap(chrom, start, end, |_| blocked = true);
        if !blocked {
            rows.push(row.clone());
        }
    }
    Ok(Value::Table(Table::new(peaks.columns.clone(), rows)))
}

// ── gene_activity(fragments, genes[, upstream[, downstream[, barcodes]]]) ──

fn builtin_gene_activity(args: Vec<Value>) -> Result<Value> {
    let func = "gene_activity";
    let src = frag_source(&args[0], func)?;
    let genes = match &args[1] {
        Value::Table(t) => t,
        _ => {
            return Err(BioLangError::type_error(
                format!("{func}() genes must be a Table (e.g. from gene_bodies())"),
                None,
            ))
        }
    };
    // Signac's GeneActivity() defaults: 2 kb upstream of the TSS, nothing past
    // the 3' end.
    let upstream = if args.len() > 2 {
        to_i64(&args[2])
    } else {
        2000
    };
    let downstream = if args.len() > 3 { to_i64(&args[3]) } else { 0 };
    let whitelist = opt_barcode_list(args.get(4), func)?;

    let ci_chrom = require_named_col(genes, "chrom", func)?;
    let ci_start = require_named_col(genes, "start", func)?;
    let ci_end = require_named_col(genes, "end", func)?;
    let ci_strand = genes.columns.iter().position(|c| c == "strand");
    let ci_name = genes
        .columns
        .iter()
        .position(|c| c == "gene_name")
        .filter(|&i| genes.rows.iter().any(|r| !str_col(&r[i]).is_empty()));
    let ci_id = genes.columns.iter().position(|c| c == "gene_id");
    let ci_label = ci_name.or(ci_id).ok_or_else(|| {
        BioLangError::type_error(
            format!("{func}() genes table needs a 'gene_name' or 'gene_id' column"),
            None,
        )
    })?;

    // Rows naming the same gene collapse into one column, so a gene split
    // across several annotation rows is counted once.
    let mut col_of: HashMap<String, u32> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut index = RegionIndex::new();

    for row in &genes.rows {
        let label = str_col(&row[ci_label]);
        if label.is_empty() {
            continue;
        }
        let chrom = str_col(&row[ci_chrom]);
        let start = to_i64(&row[ci_start]);
        let end = to_i64(&row[ci_end]);
        let minus = ci_strand.map(|i| str_col(&row[i]) == "-").unwrap_or(false);

        // Extend upstream of the TSS, which is the 3' coordinate on the minus
        // strand.
        let (rstart, rend) = if minus {
            (start - downstream, end + upstream)
        } else {
            (start - upstream, end + downstream)
        };

        let next = names.len() as u32;
        let col = *col_of.entry(label.to_string()).or_insert_with(|| {
            names.push(label.to_string());
            next
        });
        index.push(chrom, rstart.max(0), rend, col);
    }

    count_into_object(&src, func, index, names, whitelist, false)
}
