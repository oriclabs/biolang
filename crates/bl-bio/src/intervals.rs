use bio_core::{GenomicInterval, Strand};
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

/// Returns interval-related builtin registrations.
pub fn interval_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("intersect", Arity::Exact(2)),
        ("merge_intervals", Arity::Exact(1)),
        ("subtract", Arity::Exact(2)),
        ("closest", Arity::Exact(2)),
        ("flank", Arity::Exact(2)),
    ]
}

pub fn is_interval_builtin(name: &str) -> bool {
    matches!(
        name,
        "intersect" | "merge_intervals" | "subtract" | "closest" | "flank"
    )
}

pub fn call_interval_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "intersect" => builtin_intersect(args),
        "merge_intervals" => builtin_merge_intervals(args),
        "subtract" => builtin_subtract(args),
        "closest" => builtin_closest(args),
        "flank" => builtin_flank(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown interval builtin '{name}'"),
            None,
        )),
    }
}

/// Extract (chrom_idx, start_idx, end_idx) from a table with those columns.
fn get_interval_cols(table: &Table, func: &str) -> Result<(usize, usize, usize)> {
    let ci = table.col_index("chrom").or_else(|| table.col_index("seqid"));
    let si = table.col_index("start");
    let ei = table.col_index("end");

    match (ci, si, ei) {
        (Some(c), Some(s), Some(e)) => Ok((c, s, e)),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table with chrom/start/end columns"),
            None,
        )),
    }
}

/// Extract interval data from a row.
fn row_interval(row: &[Value], ci: usize, si: usize, ei: usize) -> Option<(String, i64, i64)> {
    let chrom = match &row[ci] {
        Value::Str(s) => s.clone(),
        _ => return None,
    };
    let start = row[si].as_int()?;
    let end = row[ei].as_int()?;
    Some((chrom, start, end))
}

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn to_genomic(chrom: &str, start: i64, end: i64) -> GenomicInterval {
    GenomicInterval {
        chrom: chrom.to_string(),
        start,
        end,
        strand: Strand::Unknown,
    }
}

/// Collect all intervals from a table as GenomicIntervals.
fn table_to_intervals(
    table: &Table,
    ci: usize,
    si: usize,
    ei: usize,
) -> Vec<GenomicInterval> {
    table
        .rows
        .iter()
        .filter_map(|row| {
            row_interval(row, ci, si, ei)
                .map(|(c, s, e)| to_genomic(&c, s, e))
        })
        .collect()
}

fn intervals_to_table(intervals: Vec<GenomicInterval>) -> Value {
    let columns = vec!["chrom".to_string(), "start".to_string(), "end".to_string()];
    let rows: Vec<Vec<Value>> = intervals
        .into_iter()
        .map(|iv| vec![Value::Str(iv.chrom), Value::Int(iv.start), Value::Int(iv.end)])
        .collect();
    Value::Table(Table::new(columns, rows))
}

// ── intersect(table_a, table_b) ─────────────────────────────────────

fn builtin_intersect(args: Vec<Value>) -> Result<Value> {
    let a = require_table(&args[0], "intersect")?;
    let b = require_table(&args[1], "intersect")?;
    let (aci, asi, aei) = get_interval_cols(a, "intersect")?;
    let (bci, bsi, bei) = get_interval_cols(b, "intersect")?;

    let b_intervals = table_to_intervals(b, bci, bsi, bei);

    let mut result_rows: Vec<Vec<Value>> = Vec::new();
    for arow in &a.rows {
        if let Some((achrom, astart, aend)) = row_interval(arow, aci, asi, aei) {
            let a_iv = to_genomic(&achrom, astart, aend);
            if b_intervals.iter().any(|bi| bio_core::interval_ops::overlaps(&a_iv, bi)) {
                result_rows.push(arow.to_vec());
            }
        }
    }

    Ok(Value::Table(Table::new(a.columns.clone(), result_rows)))
}

// ── merge_intervals(table) ──────────────────────────────────────────

fn builtin_merge_intervals(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "merge_intervals")?;
    let (ci, si, ei) = get_interval_cols(table, "merge_intervals")?;

    let intervals = table_to_intervals(table, ci, si, ei);
    let merged = bio_core::interval_ops::merge(&intervals);
    Ok(intervals_to_table(merged))
}

// ── subtract(table_a, table_b) ──────────────────────────────────────

fn builtin_subtract(args: Vec<Value>) -> Result<Value> {
    let a = require_table(&args[0], "subtract")?;
    let b = require_table(&args[1], "subtract")?;
    let (aci, asi, aei) = get_interval_cols(a, "subtract")?;
    let (bci, bsi, bei) = get_interval_cols(b, "subtract")?;

    let b_intervals = table_to_intervals(b, bci, bsi, bei);

    let mut result_rows: Vec<Vec<Value>> = Vec::new();
    for arow in &a.rows {
        if let Some((achrom, astart, aend)) = row_interval(arow, aci, asi, aei) {
            let a_iv = to_genomic(&achrom, astart, aend);
            if !b_intervals.iter().any(|bi| bio_core::interval_ops::overlaps(&a_iv, bi)) {
                result_rows.push(arow.to_vec());
            }
        }
    }

    Ok(Value::Table(Table::new(a.columns.clone(), result_rows)))
}

// ── closest(table_a, table_b) ───────────────────────────────────────

fn builtin_closest(args: Vec<Value>) -> Result<Value> {
    let a = require_table(&args[0], "closest")?;
    let b = require_table(&args[1], "closest")?;
    let (aci, asi, aei) = get_interval_cols(a, "closest")?;
    let (bci, bsi, bei) = get_interval_cols(b, "closest")?;

    let a_intervals = table_to_intervals(a, aci, asi, aei);
    let b_intervals = table_to_intervals(b, bci, bsi, bei);

    let distances = bio_core::interval_ops::closest_distance(&a_intervals, &b_intervals);

    let mut out_cols = a.columns.clone();
    out_cols.push("distance".to_string());

    let mut result_rows: Vec<Vec<Value>> = Vec::new();
    // Map distances back to rows (distances are indexed by position in a_intervals,
    // which matches filtered rows that had valid intervals)
    let mut dist_iter = distances.into_iter();
    for arow in &a.rows {
        if row_interval(arow, aci, asi, aei).is_some() {
            let (_idx, dist) = dist_iter.next().unwrap();
            let mut row = arow.to_vec();
            row.push(dist.map(Value::Int).unwrap_or(Value::Nil));
            result_rows.push(row);
        }
    }

    Ok(Value::Table(Table::new(out_cols, result_rows)))
}

// ── flank(table, size) ──────────────────────────────────────────────

fn builtin_flank(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "flank")?;
    let size = match &args[1] {
        Value::Int(n) => *n,
        other => {
            return Err(BioLangError::type_error(
                format!("flank() size must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    let (ci, si, ei) = get_interval_cols(table, "flank")?;

    let intervals = table_to_intervals(table, ci, si, ei);
    let flank_pairs = bio_core::interval_ops::flanks(&intervals, size, size);

    let columns = vec!["chrom".to_string(), "start".to_string(), "end".to_string()];
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for (up, down) in flank_pairs {
        // Only include upstream flank if it has positive width
        if up.start < up.end {
            rows.push(vec![
                Value::Str(up.chrom),
                Value::Int(up.start),
                Value::Int(up.end),
            ]);
        }
        rows.push(vec![
            Value::Str(down.chrom),
            Value::Int(down.start),
            Value::Int(down.end),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

