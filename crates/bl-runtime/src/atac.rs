//! ATAC-seq specific quality control and fragment analysis builtins.
//!
//! Functions: fragment_size_dist, nfr_enrichment, nucleosome_fractions,
//! tss_enrichment_score, atac_qc, peak_matrix, gene_activity.
//!
//! `peak_matrix` and `gene_activity` turn a fragments file into a cell x
//! feature count matrix shaped exactly like the object [`crate::singlecell`]'s
//! `read_10x` returns, so the whole scRNA workflow — normalize, variable_genes,
//! neighbors, cluster_leiden, marker_table — applies to chromatin data
//! unchanged.

use bl_core::error::{BioLangError, ErrorKind, Result};
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
        ("peak_matrix", Arity::Range(2, 3)),
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
            | "peak_matrix"
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
        "peak_matrix" => builtin_peak_matrix(args),
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
            for row in &t.rows {
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

/// Count fragments into a cell x feature matrix and package it as an sc object
/// with the same field names `read_10x` produces.
fn count_into_object(
    src: &FragSource,
    func: &str,
    mut index: RegionIndex,
    feature_names: Vec<String>,
    whitelist: Option<Vec<String>>,
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

    for_each_fragment(src, func, |chrom, start, end, bc, count| {
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
    if entries > MAX_DENSE_ENTRIES {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!(
                "{func}(): {n_cells} cells x {n_features} features is {entries} matrix entries, \
                 over the {MAX_DENSE_ENTRIES} limit. Subset the features (or the barcodes) first \
                 — gene_activity() over gene bodies is the usual way to get a matrix small enough \
                 for the single-cell pipeline."
            ),
            None,
        ));
    }

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

    let mut rec: HashMap<String, Value> = HashMap::new();
    rec.insert("matrix".to_string(), Value::List(rows.into()));
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

    count_into_object(&src, func, index, names, whitelist)
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

    count_into_object(&src, func, index, names, whitelist)
}
