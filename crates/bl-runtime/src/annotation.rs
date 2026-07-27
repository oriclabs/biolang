//! Genomic annotation builtins.
//!
//! Functions: parse_gtf, gene_bodies, promoters, interval_overlap,
//! annotate_peaks, gene_id_map.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ─────────────────────────────────────────────────────────

pub fn annotation_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("parse_gtf", Arity::Exact(1)),
        ("gene_bodies", Arity::Exact(1)),
        ("promoters", Arity::Range(1, 3)),
        ("interval_overlap", Arity::Exact(2)),
        ("annotate_peaks", Arity::Exact(2)),
        ("gene_id_map", Arity::Exact(1)),
    ]
}

pub fn is_annotation_builtin(name: &str) -> bool {
    matches!(
        name,
        "parse_gtf"
            | "gene_bodies"
            | "promoters"
            | "interval_overlap"
            | "annotate_peaks"
            | "gene_id_map"
    )
}

pub fn call_annotation_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "parse_gtf"         => builtin_parse_gtf(args),
        "gene_bodies"       => builtin_gene_bodies(args),
        "promoters"         => builtin_promoters(args),
        "interval_overlap"  => builtin_interval_overlap(args),
        "annotate_peaks"    => builtin_annotate_peaks(args),
        "gene_id_map"       => builtin_gene_id_map(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown annotation builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(format!("{func}() requires Table"), None)),
    }
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

fn col_idx(t: &Table, name: &str) -> Option<usize> {
    t.columns.iter().position(|c| c == name)
}

fn require_col(t: &Table, name: &str, func: &str) -> Result<usize> {
    col_idx(t, name).ok_or_else(|| {
        BioLangError::type_error(format!("{func}() requires column '{name}'"), None)
    })
}

fn str_val(v: &Value) -> &str {
    match v { Value::Str(s) => s.as_str(), _ => "" }
}

/// Parse GTF attribute string: `key "value"; key2 "value2";`
fn parse_gtf_attrs(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() { continue; }
        let mut it = part.splitn(2, ' ');
        let key = it.next().unwrap_or("").trim().to_string();
        let val = it.next().unwrap_or("").trim().trim_matches('"').to_string();
        if !key.is_empty() { map.insert(key, val); }
    }
    map
}

/// Parse GFF3 attribute string: `key=value;key2=value2`
fn parse_gff3_attrs(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() { continue; }
        let mut it = part.splitn(2, '=');
        let key = it.next().unwrap_or("").trim().to_lowercase();
        let val = it.next().unwrap_or("").trim().to_string();
        if !key.is_empty() { map.insert(key, val); }
    }
    map
}

// ── parse_gtf ─────────────────────────────────────────────────────────

fn builtin_parse_gtf(args: Vec<Value>) -> Result<Value> {
    let text = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(BioLangError::type_error("parse_gtf() requires Str", None)),
    };

    let cols = vec![
        "chrom", "source", "feature", "start", "end", "strand",
        "gene_id", "transcript_id", "gene_name", "gene_type",
    ];
    let out_cols: Vec<String> = cols.iter().map(|s| s.to_string()).collect();

    // Auto-detect GTF vs GFF3 from first non-comment data line
    let first_data = text.lines()
        .find(|l| !l.trim_start().starts_with('#') && l.contains('\t'));
    let is_gff3 = first_data
        .and_then(|l| l.split('\t').nth(8))
        .map(|attr| attr.contains('='))
        .unwrap_or(false);

    let mut rows: Vec<Vec<Value>> = vec![];
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 9 { continue; }

        let chrom    = fields[0].to_string();
        let source   = fields[1].to_string();
        let feature  = fields[2].to_string();
        let start: i64 = fields[3].parse().unwrap_or(0);
        let end: i64   = fields[4].parse().unwrap_or(0);
        let strand   = fields[6].to_string();
        let attr_str = fields[8];

        let attrs = if is_gff3 {
            parse_gff3_attrs(attr_str)
        } else {
            parse_gtf_attrs(attr_str)
        };

        let get = |k: &str| attrs.get(k).cloned().unwrap_or_default();
        let get_or = |k1: &str, k2: &str| -> String {
            if let Some(v) = attrs.get(k1) { v.clone() }
            else { attrs.get(k2).cloned().unwrap_or_default() }
        };
        let gene_id      = get_or("gene_id", "ID");
        let transcript_id = get("transcript_id");
        let gene_name    = get_or("gene_name", "Name");
        let gene_type    = get_or("gene_type", "gene_biotype");

        rows.push(vec![
            Value::Str(chrom),
            Value::Str(source),
            Value::Str(feature),
            Value::Int(start),
            Value::Int(end),
            Value::Str(strand),
            Value::Str(gene_id),
            Value::Str(transcript_id),
            Value::Str(gene_name),
            Value::Str(gene_type),
        ]);
    }

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── gene_bodies ───────────────────────────────────────────────────────

fn builtin_gene_bodies(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "gene_bodies")?;
    let ci_gene_id   = require_col(t, "gene_id",   "gene_bodies")?;
    let ci_chrom     = require_col(t, "chrom",     "gene_bodies")?;
    let ci_start     = require_col(t, "start",     "gene_bodies")?;
    let ci_end       = require_col(t, "end",       "gene_bodies")?;
    let ci_strand    = require_col(t, "strand",    "gene_bodies")?;
    let ci_gene_name = col_idx(t, "gene_name");

    struct GeneInfo {
        chrom: String,
        strand: String,
        gene_name: String,
        min_start: i64,
        max_end: i64,
    }

    let mut genes: HashMap<String, GeneInfo> = HashMap::new();
    for row in &t.rows {
        let gene_id = str_val(&row[ci_gene_id]).to_string();
        if gene_id.is_empty() { continue; }
        let chrom  = str_val(&row[ci_chrom]).to_string();
        let start  = to_i64(&row[ci_start]);
        let end    = to_i64(&row[ci_end]);
        let strand = str_val(&row[ci_strand]).to_string();
        let name   = ci_gene_name.map(|i| str_val(&row[i]).to_string()).unwrap_or_default();
        let e = genes.entry(gene_id).or_insert(GeneInfo {
            chrom: chrom.clone(), strand: strand.clone(), gene_name: name.clone(),
            min_start: start, max_end: end,
        });
        if start < e.min_start { e.min_start = start; }
        if end > e.max_end { e.max_end = end; }
    }

    let mut gene_ids: Vec<String> = genes.keys().cloned().collect();
    gene_ids.sort();

    let out_cols = vec!["gene_id","gene_name","chrom","start","end","strand"]
        .into_iter().map(|s| s.to_string()).collect();
    let rows: Vec<Vec<Value>> = gene_ids.iter().map(|gid| {
        let g = &genes[gid];
        vec![
            Value::Str(gid.clone()),
            Value::Str(g.gene_name.clone()),
            Value::Str(g.chrom.clone()),
            Value::Int(g.min_start),
            Value::Int(g.max_end),
            Value::Str(g.strand.clone()),
        ]
    }).collect();

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── promoters ─────────────────────────────────────────────────────────

fn builtin_promoters(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "promoters")?;
    let upstream   = if args.len() > 1 { to_i64(&args[1]) } else { 2000 };
    let downstream = if args.len() > 2 { to_i64(&args[2]) } else { 200 };

    let ci_gene_id   = require_col(t, "gene_id",  "promoters")?;
    let ci_chrom     = require_col(t, "chrom",    "promoters")?;
    let ci_start     = require_col(t, "start",    "promoters")?;
    let ci_end       = require_col(t, "end",      "promoters")?;
    let ci_strand    = require_col(t, "strand",   "promoters")?;
    let ci_gene_name = col_idx(t, "gene_name");
    let ci_feature   = col_idx(t, "feature");

    let out_cols: Vec<String> = vec!["gene_id","gene_name","chrom","start","end","strand","tss"]
        .into_iter().map(|s| s.to_string()).collect();

    let rows: Vec<Vec<Value>> = t.rows.iter().filter_map(|row| {
        // Filter to transcript features if available
        if let Some(fi) = ci_feature {
            let feat = str_val(&row[fi]);
            if feat != "transcript" && feat != "gene" { return None; }
        }
        let gene_id  = str_val(&row[ci_gene_id]).to_string();
        let chrom    = str_val(&row[ci_chrom]).to_string();
        let start    = to_i64(&row[ci_start]);
        let end      = to_i64(&row[ci_end]);
        let strand   = str_val(&row[ci_strand]).to_string();
        let gene_name = ci_gene_name.map(|i| str_val(&row[i]).to_string()).unwrap_or_default();

        let (tss, prom_start, prom_end) = if strand == "-" {
            let tss = end;
            (tss, tss - downstream, tss + upstream)
        } else {
            let tss = start;
            (tss, tss - upstream, tss + downstream)
        };

        Some(vec![
            Value::Str(gene_id),
            Value::Str(gene_name),
            Value::Str(chrom),
            Value::Int(prom_start.max(1)),
            Value::Int(prom_end),
            Value::Str(strand),
            Value::Int(tss),
        ])
    }).collect();

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── interval_overlap ──────────────────────────────────────────────────

fn builtin_interval_overlap(args: Vec<Value>) -> Result<Value> {
    let query   = require_table(&args[0], "interval_overlap")?;
    let subject = require_table(&args[1], "interval_overlap")?;

    let qc_chrom = require_col(query,   "chrom", "interval_overlap")?;
    let qc_start = require_col(query,   "start", "interval_overlap")?;
    let qc_end   = require_col(query,   "end",   "interval_overlap")?;
    let sc_chrom = require_col(subject, "chrom", "interval_overlap")?;
    let sc_start = require_col(subject, "start", "interval_overlap")?;
    let sc_end   = require_col(subject, "end",   "interval_overlap")?;

    let mut out_cols: Vec<String> = query.columns.clone();
    for c in &subject.columns {
        out_cols.push(format!("{c}_subject"));
    }

    let mut rows: Vec<Vec<Value>> = vec![];
    for qrow in &query.rows {
        let qchrom = str_val(&qrow[qc_chrom]);
        let qstart = to_i64(&qrow[qc_start]);
        let qend   = to_i64(&qrow[qc_end]);
        for srow in &subject.rows {
            let schrom = str_val(&srow[sc_chrom]);
            let sstart = to_i64(&srow[sc_start]);
            let send   = to_i64(&srow[sc_end]);
            if qchrom == schrom && qstart < send && sstart < qend {
                let mut combined = qrow.clone();
                combined.extend_from_slice(srow);
                rows.push(combined);
            }
        }
    }

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── annotate_peaks ────────────────────────────────────────────────────

fn builtin_annotate_peaks(args: Vec<Value>) -> Result<Value> {
    let peaks = require_table(&args[0], "annotate_peaks")?;
    let gtf   = require_table(&args[1], "annotate_peaks")?;

    let pc_chrom = require_col(peaks, "chrom", "annotate_peaks")?;
    let pc_start = require_col(peaks, "start", "annotate_peaks")?;
    let pc_end   = require_col(peaks, "end",   "annotate_peaks")?;

    let gc_chrom     = require_col(gtf, "chrom",     "annotate_peaks")?;
    let gc_start     = require_col(gtf, "start",     "annotate_peaks")?;
    let gc_end       = require_col(gtf, "end",       "annotate_peaks")?;
    let gc_gene_id   = require_col(gtf, "gene_id",   "annotate_peaks")?;
    let gc_gene_name = col_idx(gtf, "gene_name");
    let gc_strand    = col_idx(gtf, "strand");

    // Build gene body list (for same-chrom quick lookup)
    struct Gene {
        chrom: String,
        start: i64,
        end: i64,
        gene_id: String,
        gene_name: String,
        tss: i64,
    }

    let genes: Vec<Gene> = gtf.rows.iter().filter_map(|row| {
        let gene_id = str_val(&row[gc_gene_id]).to_string();
        if gene_id.is_empty() { return None; }
        let chrom = str_val(&row[gc_chrom]).to_string();
        let start = to_i64(&row[gc_start]);
        let end   = to_i64(&row[gc_end]);
        let gene_name = gc_gene_name.map(|i| str_val(&row[i]).to_string()).unwrap_or_default();
        let strand = gc_strand.map(|i| str_val(&row[i]).to_string()).unwrap_or("+".to_string());
        let tss = if strand == "-" { end } else { start };
        Some(Gene { chrom, start, end, gene_id, gene_name, tss })
    }).collect();

    let mut out_cols = peaks.columns.clone();
    out_cols.extend(["nearest_gene_id","nearest_gene_name","distance_to_gene","annotation"]
        .iter().map(|s| s.to_string()));

    let rows: Vec<Vec<Value>> = peaks.rows.iter().map(|prow| {
        let pchrom = str_val(&prow[pc_chrom]);
        let pstart = to_i64(&prow[pc_start]);
        let pend   = to_i64(&prow[pc_end]);
        let pmid   = (pstart + pend) / 2;

        let mut best_gene_id   = String::new();
        let mut best_gene_name = String::new();
        let mut best_dist = i64::MAX;
        let mut best_start = 0i64;
        let mut best_end   = 0i64;

        for g in &genes {
            if g.chrom != pchrom { continue; }
            let gmid = (g.start + g.end) / 2;
            let dist = (pmid - gmid).abs();
            if dist < best_dist {
                best_dist = dist;
                best_gene_id   = g.gene_id.clone();
                best_gene_name = g.gene_name.clone();
                best_start = g.start;
                best_end   = g.end;
            }
        }

        // Compute actual distance (0 if overlapping)
        let distance = if pstart < best_end && best_start < pend {
            0i64
        } else if pmid < best_start {
            best_start - pend
        } else {
            pstart - best_end
        };

        let annotation = if distance.abs() <= 2000 {
            "Promoter"
        } else if pstart >= best_start && pend <= best_end {
            "Intragenic"
        } else {
            "Distal"
        };

        let mut row = prow.clone();
        row.push(Value::Str(best_gene_id));
        row.push(Value::Str(best_gene_name));
        row.push(Value::Int(distance.abs()));
        row.push(Value::Str(annotation.to_string()));
        row
    }).collect();

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── gene_id_map ───────────────────────────────────────────────────────

fn builtin_gene_id_map(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "gene_id_map")?;
    let ci_gene_id   = require_col(t, "gene_id",   "gene_id_map")?;
    let ci_gene_name = require_col(t, "gene_name", "gene_id_map")?;

    let mut seen: HashMap<String, String> = HashMap::new();
    for row in &t.rows {
        let gid  = str_val(&row[ci_gene_id]).to_string();
        let name = str_val(&row[ci_gene_name]).to_string();
        if !gid.is_empty() { seen.entry(gid).or_insert(name); }
    }

    let mut ids: Vec<String> = seen.keys().cloned().collect();
    ids.sort();
    let out_cols = vec!["gene_id".to_string(), "gene_name".to_string()];
    let rows: Vec<Vec<Value>> = ids.iter().map(|gid| {
        vec![Value::Str(gid.clone()), Value::Str(seen[gid].clone())]
    }).collect();

    Ok(Value::Table(Table::new(out_cols, rows)))
}
