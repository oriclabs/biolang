use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for all table operation builtins.
pub fn table_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // Structure
        ("select", Arity::AtLeast(2)),
        ("drop_cols", Arity::AtLeast(2)),
        ("rename", Arity::Exact(3)),
        ("ncol", Arity::Exact(1)),
        ("nrow", Arity::Exact(1)),
        ("colnames", Arity::Exact(1)),
        ("to_records", Arity::Exact(1)),
        ("from_records", Arity::Exact(1)),
        ("to_table", Arity::Exact(1)),   // alias for from_records
        ("from_table", Arity::Exact(1)), // alias for to_records
        // Row operations
        ("head", Arity::Range(1, 2)),
        ("tail", Arity::Range(1, 2)),
        ("slice", Arity::Range(2, 3)),
        ("sample", Arity::Range(1, 2)),
        ("arrange", Arity::AtLeast(2)),
        ("distinct", Arity::Range(1, 2)),
        // Null handling
        ("fill_null", Arity::Exact(2)),
        ("drop_null", Arity::Range(1, 2)),
        // Aggregation
        ("group_by", Arity::Exact(2)),
        ("count_by", Arity::Exact(2)),
        ("filter_by", Arity::Exact(4)),
        ("multi_filter_by", Arity::AtLeast(4)),
        ("count_where", Arity::Exact(4)),
        ("variant_summary", Arity::Range(1, 2)),
        ("classify_variants", Arity::Exact(1)),
        ("value_counts", Arity::Exact(2)),
        ("describe", Arity::Exact(1)),
        // Combine
        ("bio_join", Arity::Range(2, 4)),
        ("concat", Arity::AtLeast(2)),
        ("bind_cols", Arity::AtLeast(2)),
        ("inner_join", Arity::Exact(3)),
        ("left_join", Arity::Exact(3)),
        ("right_join", Arity::Exact(3)),
        ("outer_join", Arity::Exact(3)),
        ("cross_join", Arity::Exact(2)),
        ("anti_join", Arity::Exact(3)),
        ("semi_join", Arity::Exact(3)),
        // Reshape
        ("explode", Arity::Exact(2)),
        ("pivot_longer", Arity::Exact(4)),
        ("pivot_wider", Arity::Exact(3)),
        // Window functions
        ("row_number", Arity::Exact(1)),
        ("rank", Arity::Exact(2)),
        ("lag", Arity::Range(2, 3)),
        ("lead", Arity::Range(2, 3)),
        ("cumsum", Arity::Range(1, 2)),
        ("cummax", Arity::Exact(2)),
        ("cummin", Arity::Exact(2)),
        ("rolling_mean", Arity::Exact(3)),
        ("rolling_sum", Arity::Exact(3)),
        // Column extraction & aggregation
        ("col_values", Arity::Exact(2)),
        ("col_mean", Arity::Exact(2)),
        ("col_sum", Arity::Exact(2)),
        ("col_stdev", Arity::Exact(2)),
        ("col_min", Arity::Exact(2)),
        ("col_max", Arity::Exact(2)),
        ("group_stats", Arity::Exact(3)),
        // Display
        ("col_width", Arity::Exact(2)),
        // I/O
        ("csv", Arity::Exact(1)),
        ("tsv", Arity::Exact(1)),
        ("write_csv", Arity::Exact(2)),
        ("write_tsv", Arity::Exact(2)),
    ]
}

/// Check if a name is a known table builtin.
pub fn is_table_builtin(name: &str) -> bool {
    matches!(
        name,
        "select"
            | "drop_cols"
            | "rename"
            | "ncol"
            | "nrow"
            | "colnames"
            | "to_records"
            | "from_records"
            | "to_table"
            | "from_table"
            | "head"
            | "tail"
            | "slice"
            | "sample"
            | "arrange"
            | "distinct"
            | "fill_null"
            | "drop_null"
            | "group_by"
            | "count_by"
            | "filter_by"
            | "multi_filter_by"
            | "count_where"
            | "variant_summary"
            | "classify_variants"
            | "value_counts"
            | "describe"
            | "bio_join"
            | "concat"
            | "bind_cols"
            | "inner_join"
            | "left_join"
            | "right_join"
            | "outer_join"
            | "cross_join"
            | "anti_join"
            | "semi_join"
            | "explode"
            | "pivot_longer"
            | "pivot_wider"
            | "row_number"
            | "rank"
            | "lag"
            | "lead"
            | "cumsum"
            | "cummax"
            | "cummin"
            | "rolling_mean"
            | "rolling_sum"
            | "col_values"
            | "col_mean"
            | "col_sum"
            | "col_stdev"
            | "col_min"
            | "col_max"
            | "group_stats"
            | "col_width"
            | "csv"
            | "tsv"
            | "write_csv"
            | "write_tsv"
    )
}

/// Execute a table builtin by name.
pub fn call_table_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "select" => builtin_select(args),
        "drop_cols" => builtin_drop_cols(args),
        "rename" => builtin_rename(args),
        "ncol" => builtin_ncol(args),
        "nrow" => builtin_nrow(args),
        "colnames" => builtin_colnames(args),
        "to_records" => builtin_to_records(args),
        "from_records" => builtin_from_records(args),
        "to_table" => builtin_from_records(args),   // alias
        "from_table" => builtin_to_records(args),   // alias
        "head" => builtin_head(args),
        "tail" => builtin_tail(args),
        "slice" => builtin_slice(args),
        "sample" => builtin_sample(args),
        "arrange" => builtin_arrange(args),
        "distinct" => builtin_distinct(args),
        "fill_null" => builtin_fill_null(args),
        "drop_null" => builtin_drop_null(args),
        "group_by" => builtin_group_by(args),
        "count_by" => builtin_count_by(args),
        "filter_by" => builtin_filter_by(args),
        "multi_filter_by" => builtin_multi_filter_by(args),
        "count_where" => builtin_count_where(args),
        "variant_summary" => builtin_variant_summary(args),
        "classify_variants" => builtin_classify_variants(args),
        "value_counts" => builtin_value_counts(args),
        "describe" => builtin_describe(args),
        "bio_join" => builtin_bio_join(args),
        "concat" => builtin_concat(args),
        "bind_cols" => builtin_bind_cols(args),
        "inner_join" => builtin_inner_join(args),
        "left_join" => builtin_left_join(args),
        "right_join" => builtin_right_join(args),
        "outer_join" => builtin_outer_join(args),
        "cross_join" => builtin_cross_join(args),
        "anti_join" => builtin_anti_join(args),
        "semi_join" => builtin_semi_join(args),
        "explode" => builtin_explode(args),
        "pivot_longer" => builtin_pivot_longer(args),
        "pivot_wider" => builtin_pivot_wider(args),
        "row_number" => builtin_row_number(args),
        "rank" => builtin_rank(args),
        "lag" => builtin_lag(args),
        "lead" => builtin_lead(args),
        "cumsum" => builtin_cumsum(args),
        "cummax" => builtin_cummax(args),
        "cummin" => builtin_cummin(args),
        "rolling_mean" => builtin_rolling_mean(args),
        "rolling_sum" => builtin_rolling_sum(args),
        "col_values" => builtin_col_values(args),
        "col_mean" => builtin_col_mean(args),
        "col_sum" => builtin_col_sum(args),
        "col_stdev" => builtin_col_stdev(args),
        "col_min" => builtin_col_min(args),
        "col_max" => builtin_col_max(args),
        "group_stats" => builtin_group_stats(args),
        "col_width" => builtin_col_width(args),
        "csv" => builtin_read_delimited(args, ','),
        "tsv" => builtin_read_delimited(args, '\t'),
        "write_csv" => builtin_write_delimited(args, ','),
        "write_tsv" => builtin_write_delimited(args, '\t'),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown table builtin '{name}'"),
            None,
        )),
    }
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

fn require_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::Str(s) => Ok(s.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

// ── select(table, col1, col2, ...) ──────────────────────────────────

fn builtin_select(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "select")?;
    let mut col_names = Vec::new();
    for arg in &args[1..] {
        col_names.push(require_str(arg, "select")?);
    }

    let mut indices = Vec::new();
    for name in &col_names {
        match table.col_index(name) {
            Some(i) => indices.push(i),
            None => {
                return Err(BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("select(): column '{name}' not found"),
                    None,
                ))
            }
        }
    }

    let new_rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .map(|row| indices.iter().map(|&i| row[i].clone()).collect())
        .collect();

    Ok(Value::Table(Table::new(col_names, new_rows)))
}

// ── arrange(table, col, ...) — prefix "-" for desc, or trailing "desc"/"asc" ─

fn builtin_arrange(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "arrange")?;

    // Parse sort specs: column name (optionally prefixed with "-"),
    // optionally followed by "desc" or "asc" modifier
    let mut specs: Vec<(usize, bool)> = Vec::new(); // (col_index, ascending)
    let rest = &args[1..];
    let mut i = 0;
    while i < rest.len() {
        let s = require_str(&rest[i], "arrange")?;
        let lower = s.to_ascii_lowercase();

        // Skip standalone "desc"/"asc" that already got applied (shouldn't happen, but guard)
        if (lower == "desc" || lower == "asc") && !specs.is_empty() {
            i += 1;
            continue;
        }

        let (name, asc) = if let Some(stripped) = s.strip_prefix('-') {
            (stripped, false)
        } else {
            (s.as_str(), true)
        };
        match table.col_index(name) {
            Some(ci) => {
                // Check if next arg is a "desc" or "asc" modifier
                let mut final_asc = asc;
                if i + 1 < rest.len() {
                    if let Ok(next) = require_str(&rest[i + 1], "arrange") {
                        let next_lower = next.to_ascii_lowercase();
                        if next_lower == "desc" {
                            final_asc = false;
                            i += 1; // consume the modifier
                        } else if next_lower == "asc" {
                            final_asc = true;
                            i += 1; // consume the modifier
                        }
                    }
                }
                specs.push((ci, final_asc));
            }
            None => {
                return Err(BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("arrange(): column '{name}' not found"),
                    None,
                ))
            }
        }
        i += 1;
    }

    let mut rows = table.rows.clone();
    rows.sort_by(|a, b| {
        for &(ci, asc) in &specs {
            let cmp = val_cmp(&a[ci], &b[ci]);
            let cmp = if asc { cmp } else { cmp.reverse() };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });

    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

fn val_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => {
            (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Float(a), Value::Int(b)) => {
            a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Nil, Value::Nil) => std::cmp::Ordering::Equal,
        (Value::Nil, _) => std::cmp::Ordering::Less,
        (_, Value::Nil) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

// ── distinct(table) or distinct(table, col) ─────────────────────────

fn builtin_distinct(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "distinct")?;

    if args.len() > 1 {
        // Distinct by specific column
        let col_name = require_str(&args[1], "distinct")?;
        let ci = table.col_index(&col_name).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                format!("distinct(): column '{col_name}' not found"),
                None,
            )
        })?;
        let mut seen: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<Value>> = Vec::new();
        for row in &table.rows {
            let key = format!("{}", row[ci]);
            if !seen.contains(&key) {
                seen.push(key);
                rows.push(row.clone());
            }
        }
        Ok(Value::Table(Table::new(table.columns.clone(), rows)))
    } else {
        // Distinct entire rows
        let mut seen: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<Value>> = Vec::new();
        for row in &table.rows {
            let key: String = row.iter().map(|v| format!("{v}")).collect::<Vec<String>>().join("\t");
            if !seen.contains(&key) {
                seen.push(key);
                rows.push(row.clone());
            }
        }
        Ok(Value::Table(Table::new(table.columns.clone(), rows)))
    }
}

// ── rename(table, old, new) ─────────────────────────────────────────

fn builtin_rename(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "rename")?;
    let old_name = require_str(&args[1], "rename")?;
    let new_name = require_str(&args[2], "rename")?;

    let ci = table.col_index(&old_name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("rename(): column '{old_name}' not found"),
            None,
        )
    })?;

    let mut columns = table.columns.clone();
    columns[ci] = new_name;
    Ok(Value::Table(Table::new(columns, table.rows.clone())))
}

// ── ncol(table) ─────────────────────────────────────────────────────

fn builtin_ncol(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "ncol")?;
    Ok(Value::Int(table.num_cols() as i64))
}

// ── nrow(table) ─────────────────────────────────────────────────────

fn builtin_nrow(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "nrow")?;
    Ok(Value::Int(table.num_rows() as i64))
}

// ── colnames(table) ─────────────────────────────────────────────────

fn builtin_colnames(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "colnames")?;
    let names: Vec<Value> = table.columns.iter().map(|c| Value::Str(c.clone())).collect();
    Ok(Value::List(names))
}

// ── to_records(table) ───────────────────────────────────────────────

fn builtin_to_records(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "to_records")?;
    let records: Vec<Value> = (0..table.num_rows())
        .map(|i| Value::Record(table.row_to_record(i)))
        .collect();
    Ok(Value::List(records))
}

// ── group_by(collection, col) → Map<String, Table|List> ─────────────

fn builtin_group_by(args: Vec<Value>) -> Result<Value> {
    let col_name = require_str(&args[1], "group_by")?;

    match &args[0] {
        Value::Table(table) => {
            let ci = table.col_index(&col_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("group_by(): column '{col_name}' not found"),
                    None,
                )
            })?;

            let mut groups: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
            for row in &table.rows {
                let key = format!("{}", row[ci]);
                groups.entry(key).or_default().push(row.clone());
            }

            let mut map = HashMap::new();
            for (key, rows) in groups {
                map.insert(key, Value::Table(Table::new(table.columns.clone(), rows)));
            }
            Ok(Value::Map(map))
        }
        Value::List(items) => {
            // Group a List of Records/Variants/Maps by field name
            let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
            for item in items {
                let key = match item {
                    Value::Record(m) | Value::Map(m) => {
                        m.get(&col_name).map(|v| format!("{v}")).unwrap_or_default()
                    }
                    Value::Variant { ref chrom, pos, ref id, ref ref_allele, ref alt_allele, quality, ref filter, ref info } => {
                        match col_name.as_str() {
                            "chrom" => chrom.clone(),
                            "pos" => pos.to_string(),
                            "id" => id.clone(),
                            "ref_allele" | "ref" => ref_allele.clone(),
                            "alt_allele" | "alt" => alt_allele.clone(),
                            "quality" | "qual" => format!("{quality}"),
                            "filter" => filter.clone(),
                            _ => String::new(),
                        }
                    }
                    _ => format!("{item}"),
                };
                groups.entry(key).or_default().push(item.clone());
            }
            let mut map = HashMap::new();
            for (key, vals) in groups {
                map.insert(key, Value::List(vals));
            }
            Ok(Value::Map(map))
        }
        other => Err(BioLangError::type_error(
            format!("group_by() requires Table or List, got {}", other.type_of()),
            None,
        )),
    }
}

// ── count_by(collection, field) → List<Record{key, count}> ─────────

fn builtin_count_by(args: Vec<Value>) -> Result<Value> {
    let col_name = require_str(&args[1], "count_by")?;

    let mut counts: HashMap<String, i64> = HashMap::new();

    match &args[0] {
        Value::Table(table) => {
            let ci = table.col_index(&col_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("count_by(): column '{col_name}' not found"),
                    None,
                )
            })?;
            for row in &table.rows {
                let key = format!("{}", row[ci]);
                *counts.entry(key).or_default() += 1;
            }
        }
        Value::List(items) => {
            for item in items {
                let key = match item {
                    Value::Record(m) | Value::Map(m) => {
                        m.get(&col_name).map(|v| format!("{v}")).unwrap_or_default()
                    }
                    Value::Variant { ref chrom, ref filter, .. } => {
                        match col_name.as_str() {
                            "chrom" => chrom.clone(),
                            "filter" => filter.clone(),
                            _ => format!("{item}"),
                        }
                    }
                    _ => format!("{item}"),
                };
                *counts.entry(key).or_default() += 1;
            }
        }
        other => return Err(BioLangError::type_error(
            format!("count_by() requires Table or List, got {}", other.type_of()),
            None,
        )),
    }

    let mut result: Vec<Value> = counts.into_iter().map(|(key, count)| {
        let mut rec = HashMap::new();
        rec.insert("key".to_string(), Value::Str(key));
        rec.insert("count".to_string(), Value::Int(count));
        Value::Record(rec)
    }).collect();
    result.sort_by(|a, b| {
        let ca = if let Value::Record(m) = a { m.get("count").and_then(|v| v.as_int()).unwrap_or(0) } else { 0 };
        let cb = if let Value::Record(m) = b { m.get("count").and_then(|v| v.as_int()).unwrap_or(0) } else { 0 };
        cb.cmp(&ca) // descending
    });
    Ok(Value::List(result))
}

// ── filter_by(collection, field, op, value) → filtered collection ────
// Native filter that avoids interpreter dispatch per element.
// Op: "==", "!=", ">", ">=", "<", "<=", "contains", "starts_with", "ends_with"

fn builtin_filter_by(args: Vec<Value>) -> Result<Value> {
    let field = require_str(&args[1], "filter_by")?;
    let op = require_str(&args[2], "filter_by")?;
    let cmp_val = &args[3];

    // Extract field value from a Variant
    fn variant_field(v: &Value, field: &str) -> Option<Value> {
        if let Value::Variant { ref chrom, pos, ref id, ref ref_allele, ref alt_allele, quality, ref filter, ref info } = v {
            Some(match field {
                "chrom" => Value::Str(chrom.clone()),
                "pos" => Value::Int(*pos),
                "id" => Value::Str(id.clone()),
                "ref_allele" | "ref" => Value::Str(ref_allele.clone()),
                "alt_allele" | "alt" => Value::Str(alt_allele.clone()),
                "quality" | "qual" => Value::Float(*quality),
                "filter" => Value::Str(filter.clone()),
                "is_snp" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() == 1 && first_alt.len() == 1)
                }
                "is_indel" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() != first_alt.len())
                }
                "variant_type" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    let t = if ref_allele.len() == 1 && first_alt.len() == 1 { "SNP" }
                            else if ref_allele.len() != first_alt.len() { "INDEL" }
                            else { "MNP" };
                    Value::Str(t.to_string())
                }
                _ => {
                    // Check direct info keys first
                    if let Some(v) = info.get(field) {
                        if field != "_raw" { return Some(v.clone()); }
                    }
                    // Lazy INFO: parse _raw and look up field
                    if info.len() == 1 {
                        if let Some(Value::Str(raw)) = info.get("_raw") {
                            for part in raw.split(';') {
                                if part.is_empty() { continue; }
                                if let Some((key, val)) = part.split_once('=') {
                                    if key == field {
                                        if val == "." {
                                            return Some(Value::Nil);
                                        } else if let Ok(n) = val.parse::<i64>() {
                                            return Some(Value::Int(n));
                                        } else if let Ok(f) = val.parse::<f64>() {
                                            return Some(Value::Float(f));
                                        } else {
                                            return Some(Value::Str(val.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return None;
                }
            })
        } else {
            None
        }
    }

    // Extract field from Record/Map
    fn record_field(v: &Value, field: &str) -> Option<Value> {
        match v {
            Value::Record(m) | Value::Map(m) => m.get(field).cloned(),
            _ => None,
        }
    }

    // Get field value from any supported type
    fn get_field(v: &Value, field: &str) -> Option<Value> {
        variant_field(v, field).or_else(|| record_field(v, field))
    }

    // Compare two Values with given operator
    fn compare(field_val: &Value, op: &str, cmp_val: &Value) -> bool {
        match op {
            "==" => field_val == cmp_val,
            "!=" => field_val != cmp_val,
            ">=" | ">" | "<=" | "<" => {
                let a = match field_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                let b = match cmp_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                match op {
                    ">=" => a >= b,
                    ">" => a > b,
                    "<=" => a <= b,
                    "<" => a < b,
                    _ => false,
                }
            }
            "contains" => {
                if let (Value::Str(a), Value::Str(b)) = (field_val, cmp_val) {
                    a.contains(b.as_str())
                } else { false }
            }
            "starts_with" => {
                if let (Value::Str(a), Value::Str(b)) = (field_val, cmp_val) {
                    a.starts_with(b.as_str())
                } else { false }
            }
            "ends_with" => {
                if let (Value::Str(a), Value::Str(b)) = (field_val, cmp_val) {
                    a.ends_with(b.as_str())
                } else { false }
            }
            _ => false,
        }
    }

    match &args[0] {
        Value::Table(table) => {
            let ci = table.col_index(&field).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("filter_by(): column '{field}' not found"),
                    None,
                )
            })?;
            let columns = table.columns.clone();
            let kept_rows: Vec<Vec<Value>> = table.rows.iter()
                .filter(|row| compare(&row[ci], &op, cmp_val))
                .cloned()
                .collect();
            Ok(Value::Table(bl_core::value::Table::new(columns, kept_rows)))
        }
        Value::List(items) => {
            let result: Vec<Value> = items.iter()
                .filter(|item| {
                    get_field(item, &field)
                        .map(|fv| compare(&fv, &op, cmp_val))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            Ok(Value::List(result))
        }
        other => Err(BioLangError::type_error(
            format!("filter_by() requires Table or List, got {}", other.type_of()),
            None,
        )),
    }
}

/// multi_filter_by(collection, field1, op1, val1, field2, op2, val2, ...)
/// Applies all filter conditions in a single pass — avoids cloning intermediate results.
fn builtin_multi_filter_by(args: Vec<Value>) -> Result<Value> {
    if args.len() < 4 || (args.len() - 1) % 3 != 0 {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            "multi_filter_by() takes (collection, field, op, val, ...) in groups of 3",
            None,
        ));
    }

    // Parse conditions
    let mut conditions: Vec<(String, String, Value)> = Vec::new();
    let mut i = 1;
    while i + 2 < args.len() {
        let field = require_str(&args[i], "multi_filter_by")?;
        let op = require_str(&args[i + 1], "multi_filter_by")?;
        let cmp_val = args[i + 2].clone();
        conditions.push((field, op, cmp_val));
        i += 3;
    }

    // Reuse the same field/compare logic from filter_by
    fn variant_field(v: &Value, field: &str) -> Option<Value> {
        if let Value::Variant { ref chrom, pos, ref id, ref ref_allele, ref alt_allele, quality, ref filter, ref info } = v {
            Some(match field {
                "chrom" => Value::Str(chrom.clone()),
                "pos" => Value::Int(*pos),
                "id" => Value::Str(id.clone()),
                "ref_allele" | "ref" => Value::Str(ref_allele.clone()),
                "alt_allele" | "alt" => Value::Str(alt_allele.clone()),
                "quality" | "qual" => Value::Float(*quality),
                "filter" => Value::Str(filter.clone()),
                "is_snp" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() == 1 && first_alt.len() == 1)
                }
                "is_indel" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() != first_alt.len())
                }
                "variant_type" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    let t = if ref_allele.len() == 1 && first_alt.len() == 1 { "SNP" }
                            else if ref_allele.len() != first_alt.len() { "INDEL" }
                            else { "MNP" };
                    Value::Str(t.to_string())
                }
                _ => {
                    if let Some(v) = info.get(field) {
                        if field != "_raw" { return Some(v.clone()); }
                    }
                    if info.len() == 1 {
                        if let Some(Value::Str(raw)) = info.get("_raw") {
                            for part in raw.split(';') {
                                if part.is_empty() { continue; }
                                if let Some((key, val)) = part.split_once('=') {
                                    if key == field {
                                        if val == "." { return Some(Value::Nil); }
                                        else if let Ok(n) = val.parse::<i64>() { return Some(Value::Int(n)); }
                                        else if let Ok(f) = val.parse::<f64>() { return Some(Value::Float(f)); }
                                        else { return Some(Value::Str(val.to_string())); }
                                    }
                                }
                            }
                        }
                    }
                    return None;
                }
            })
        } else {
            None
        }
    }

    fn record_field(v: &Value, field: &str) -> Option<Value> {
        match v {
            Value::Record(m) | Value::Map(m) => m.get(field).cloned(),
            _ => None,
        }
    }

    fn get_field(v: &Value, field: &str) -> Option<Value> {
        variant_field(v, field).or_else(|| record_field(v, field))
    }

    fn compare(field_val: &Value, op: &str, cmp_val: &Value) -> bool {
        match op {
            "==" => field_val == cmp_val,
            "!=" => field_val != cmp_val,
            ">=" | ">" | "<=" | "<" => {
                let a = match field_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                let b = match cmp_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                match op { ">=" => a >= b, ">" => a > b, "<=" => a <= b, "<" => a < b, _ => false }
            }
            _ => false,
        }
    }

    match &args[0] {
        Value::List(items) => {
            let result: Vec<Value> = items.iter()
                .filter(|item| {
                    conditions.iter().all(|(field, op, cmp_val)| {
                        get_field(item, field)
                            .map(|fv| compare(&fv, op, cmp_val))
                            .unwrap_or(false)
                    })
                })
                .cloned()
                .collect();
            Ok(Value::List(result))
        }
        Value::Table(table) => {
            // Resolve column indices upfront
            let col_specs: Vec<(usize, String, Value)> = conditions.iter().map(|(field, op, cmp_val)| {
                let ci = table.col_index(field).ok_or_else(|| {
                    BioLangError::runtime(ErrorKind::NameError, format!("multi_filter_by(): column '{field}' not found"), None)
                })?;
                Ok((ci, op.clone(), cmp_val.clone()))
            }).collect::<Result<Vec<_>>>()?;

            let kept: Vec<Vec<Value>> = table.rows.iter()
                .filter(|row| {
                    col_specs.iter().all(|(ci, op, cmp_val)| compare(&row[*ci], op, cmp_val))
                })
                .cloned()
                .collect();
            Ok(Value::Table(Table::new(table.columns.clone(), kept)))
        }
        other => Err(BioLangError::type_error(
            format!("multi_filter_by() requires Table or List, got {}", other.type_of()), None,
        )),
    }
}

/// count_where(collection, field, op, value) → Int
/// Like filter_by but only counts matches — no cloning.
fn builtin_count_where(args: Vec<Value>) -> Result<Value> {
    let field = require_str(&args[1], "count_where")?;
    let op = require_str(&args[2], "count_where")?;
    let cmp_val = &args[3];

    // Reuse field extraction and comparison from filter_by
    fn variant_field_fast(v: &Value, field: &str) -> Option<Value> {
        if let Value::Variant { ref chrom, pos, ref ref_allele, ref alt_allele, quality, ref filter, ref info, .. } = v {
            Some(match field {
                "chrom" => Value::Str(chrom.clone()),
                "pos" => Value::Int(*pos),
                "quality" | "qual" => Value::Float(*quality),
                "filter" => Value::Str(filter.clone()),
                "is_snp" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() == 1 && first_alt.len() == 1)
                }
                "is_indel" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    Value::Bool(ref_allele.len() != first_alt.len())
                }
                "variant_type" => {
                    let first_alt = alt_allele.split(',').next().unwrap_or("");
                    let t = if ref_allele.len() == 1 && first_alt.len() == 1 { "SNP" }
                            else if ref_allele.len() != first_alt.len() { "INDEL" }
                            else { "MNP" };
                    Value::Str(t.to_string())
                }
                _ => {
                    if let Some(v) = info.get(field) {
                        if field != "_raw" { return Some(v.clone()); }
                    }
                    if info.len() == 1 {
                        if let Some(Value::Str(raw)) = info.get("_raw") {
                            for part in raw.split(';') {
                                if part.is_empty() { continue; }
                                if let Some((key, val)) = part.split_once('=') {
                                    if key == field {
                                        if val == "." { return Some(Value::Nil); }
                                        else if let Ok(n) = val.parse::<i64>() { return Some(Value::Int(n)); }
                                        else if let Ok(f) = val.parse::<f64>() { return Some(Value::Float(f)); }
                                        else { return Some(Value::Str(val.to_string())); }
                                    }
                                }
                            }
                        }
                    }
                    return None;
                }
            })
        } else {
            None
        }
    }

    fn get_field(v: &Value, field: &str) -> Option<Value> {
        variant_field_fast(v, field).or_else(|| match v {
            Value::Record(m) | Value::Map(m) => m.get(field).cloned(),
            _ => None,
        })
    }

    fn compare(field_val: &Value, op: &str, cmp_val: &Value) -> bool {
        match op {
            "==" => field_val == cmp_val,
            "!=" => field_val != cmp_val,
            ">=" | ">" | "<=" | "<" => {
                let a = match field_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                let b = match cmp_val {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return false,
                };
                match op { ">=" => a >= b, ">" => a > b, "<=" => a <= b, "<" => a < b, _ => false }
            }
            _ => false,
        }
    }

    let count = match &args[0] {
        Value::List(items) => {
            items.iter()
                .filter(|item| {
                    get_field(item, &field)
                        .map(|fv| compare(&fv, &op, cmp_val))
                        .unwrap_or(false)
                })
                .count()
        }
        Value::Table(table) => {
            let ci = table.col_index(&field).ok_or_else(|| {
                BioLangError::runtime(ErrorKind::NameError, format!("count_where(): column '{field}' not found"), None)
            })?;
            table.rows.iter()
                .filter(|row| compare(&row[ci], &op, cmp_val))
                .count()
        }
        other => return Err(BioLangError::type_error(
            format!("count_where() requires Table or List, got {}", other.type_of()), None,
        )),
    };

    Ok(Value::Int(count as i64))
}

/// variant_summary(variants) or variant_summary(variants, group_field)
/// Single-pass summary: groups variants by field (default "chrom") and counts SNPs/indels.
/// Returns Table with columns: group, total, snps, indels, mean_qual.
/// Entirely native — no interpreter dispatch, no cloning.
fn builtin_variant_summary(args: Vec<Value>) -> Result<Value> {
    let items = match &args[0] {
        Value::List(items) => items,
        other => return Err(BioLangError::type_error(
            format!("variant_summary() requires List of Variants, got {}", other.type_of()), None,
        )),
    };

    let group_field = if args.len() > 1 {
        require_str(&args[1], "variant_summary")?
    } else {
        "chrom".to_string()
    };

    // Single pass: accumulate stats per group
    struct GroupStats {
        total: i64,
        snps: i64,
        indels: i64,
        qual_sum: f64,
    }
    let mut groups: HashMap<String, GroupStats> = HashMap::new();

    for item in items {
        if let Value::Variant { ref chrom, ref ref_allele, ref alt_allele, quality, .. } = item {
            let key = match group_field.as_str() {
                "chrom" => chrom.clone(),
                _ => chrom.clone(), // Default to chrom for now
            };
            let stats = groups.entry(key).or_insert(GroupStats {
                total: 0, snps: 0, indels: 0, qual_sum: 0.0,
            });
            stats.total += 1;
            stats.qual_sum += quality;
            let first_alt = alt_allele.split(',').next().unwrap_or("");
            if ref_allele.len() == 1 && first_alt.len() == 1 {
                stats.snps += 1;
            } else {
                stats.indels += 1;
            }
        }
    }

    // Build result Table sorted by total descending
    let columns = vec![
        "group".to_string(), "total".to_string(), "snps".to_string(),
        "indels".to_string(), "mean_qual".to_string(),
    ];
    let mut rows: Vec<Vec<Value>> = groups.into_iter().map(|(key, stats)| {
        vec![
            Value::Str(key),
            Value::Int(stats.total),
            Value::Int(stats.snps),
            Value::Int(stats.indels),
            Value::Float(if stats.total > 0 { stats.qual_sum / stats.total as f64 } else { 0.0 }),
        ]
    }).collect();
    rows.sort_by(|a, b| {
        let ta = if let Value::Int(n) = &a[1] { *n } else { 0 };
        let tb = if let Value::Int(n) = &b[1] { *n } else { 0 };
        tb.cmp(&ta)
    });

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── classify_variants(variants) → List<Record{chrom, variant_type}> ──
// Native variant classification avoiding interpreter dispatch per element.

fn builtin_classify_variants(args: Vec<Value>) -> Result<Value> {
    let items = match &args[0] {
        Value::List(items) => items,
        other => return Err(BioLangError::type_error(
            format!("classify_variants() requires List of Variants, got {}", other.type_of()),
            None,
        )),
    };

    let mut result = Vec::with_capacity(items.len());
    for item in items {
        if let Value::Variant { ref chrom, ref ref_allele, ref alt_allele, .. } = item {
            let first_alt = alt_allele.split(',').next().unwrap_or("");
            let vtype = if ref_allele.len() == 1 && first_alt.len() == 1 {
                "SNP"
            } else {
                "INDEL"
            };
            let mut rec = HashMap::with_capacity(2);
            rec.insert("chrom".to_string(), Value::Str(chrom.clone()));
            rec.insert("variant_type".to_string(), Value::Str(vtype.to_string()));
            result.push(Value::Record(rec));
        }
    }
    Ok(Value::List(result))
}

// ── drop_cols(table, col1, col2, ...) ────────────────────────────────

fn builtin_drop_cols(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "drop_cols")?;
    let mut drop_names = Vec::new();
    for arg in &args[1..] {
        drop_names.push(require_str(arg, "drop_cols")?);
    }
    let keep_indices: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| !drop_names.contains(c))
        .map(|(i, _)| i)
        .collect();
    let new_cols: Vec<String> = keep_indices.iter().map(|&i| table.columns[i].clone()).collect();
    let new_rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .map(|row| keep_indices.iter().map(|&i| row[i].clone()).collect())
        .collect();
    Ok(Value::Table(Table::new(new_cols, new_rows)))
}

// ── from_records(list_of_records) → Table ───────────────────────────

fn builtin_from_records(args: Vec<Value>) -> Result<Value> {
    let list = match &args[0] {
        Value::List(items) => items,
        other => {
            return Err(BioLangError::type_error(
                format!("from_records() requires List, got {}", other.type_of()),
                None,
            ))
        }
    };
    if list.is_empty() {
        return Ok(Value::Table(Table::new(vec![], vec![])));
    }
    // Collect column names from first record
    let first = match &list[0] {
        Value::Record(m) | Value::Map(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("from_records() items must be Records, got {}", other.type_of()),
                None,
            ))
        }
    };
    let columns: Vec<String> = first.keys().cloned().collect();
    let mut rows = Vec::new();
    for item in list {
        let map = match item {
            Value::Record(m) | Value::Map(m) => m,
            _ => continue,
        };
        let row: Vec<Value> = columns.iter().map(|c| map.get(c).cloned().unwrap_or(Value::Nil)).collect();
        rows.push(row);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── table({col1: [...], col2: [...]}) → Table ──────────────────────

fn builtin_table(args: Vec<Value>) -> Result<Value> {
    let map = match &args[0] {
        Value::Record(m) | Value::Map(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("table() requires Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let columns: Vec<String> = map.keys().cloned().collect();
    if columns.is_empty() {
        return Ok(Value::Table(Table::new(vec![], vec![])));
    }
    // Get column lengths
    let first_col = match map.values().next() {
        Some(Value::List(items)) => items,
        _ => {
            return Err(BioLangError::type_error(
                "table() column values must be Lists",
                None,
            ))
        }
    };
    let n = first_col.len();
    let col_data: Vec<&Vec<Value>> = columns
        .iter()
        .map(|c| match map.get(c).unwrap() {
            Value::List(items) => Ok(items),
            _ => Err(BioLangError::type_error("table() column values must be Lists", None)),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let row: Vec<Value> = col_data.iter().map(|col| col.get(i).cloned().unwrap_or(Value::Nil)).collect();
        rows.push(row);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── head(table, n?) / tail(table, n?) ───────────────────────────────

fn builtin_head(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "head")?;
    let n = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n as usize,
            _ => 5,
        }
    } else {
        5
    };
    let rows = table.rows.iter().take(n).cloned().collect();
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

fn builtin_tail(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "tail")?;
    let n = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n as usize,
            _ => 5,
        }
    } else {
        5
    };
    let skip = table.num_rows().saturating_sub(n);
    let rows = table.rows.iter().skip(skip).cloned().collect();
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

// ── slice(table, start, end) ────────────────────────────────────────

fn builtin_slice(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "slice")?;
    let start = match &args[1] {
        Value::Int(n) => *n as usize,
        _ => 0,
    };
    let end = match &args[2] {
        Value::Int(n) => (*n as usize).min(table.num_rows()),
        _ => table.num_rows(),
    };
    let rows = table.rows[start..end].to_vec();
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

// ── sample(table, n?) ───────────────────────────────────────────────

fn builtin_sample(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "sample")?;
    let n = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => (*n as usize).min(table.num_rows()),
            _ => 5usize.min(table.num_rows()),
        }
    } else {
        5usize.min(table.num_rows())
    };
    // Simple deterministic sampling using stride
    let step = if n == 0 { 1 } else { (table.num_rows() as f64 / n as f64).ceil() as usize };
    let mut rows = Vec::new();
    let mut i = 0;
    while rows.len() < n && i < table.num_rows() {
        rows.push(table.rows[i].clone());
        i += step.max(1);
    }
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

// ── fill_null(table, value) ─────────────────────────────────────────

fn builtin_fill_null(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "fill_null")?;
    let fill = &args[1];
    let rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| if matches!(v, Value::Nil) { fill.clone() } else { v.clone() })
                .collect()
        })
        .collect();
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

// ── drop_null(table, col?) ──────────────────────────────────────────

fn builtin_drop_null(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "drop_null")?;
    let col_idx = if args.len() > 1 {
        let name = require_str(&args[1], "drop_null")?;
        Some(table.col_index(&name).ok_or_else(|| {
            BioLangError::runtime(ErrorKind::NameError, format!("drop_null(): column '{name}' not found"), None)
        })?)
    } else {
        None
    };
    let rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .filter(|row| {
            if let Some(ci) = col_idx {
                !matches!(row[ci], Value::Nil)
            } else {
                !row.iter().any(|v| matches!(v, Value::Nil))
            }
        })
        .cloned()
        .collect();
    Ok(Value::Table(Table::new(table.columns.clone(), rows)))
}

// ── value_counts(table, col) → Table {value, count} ────────────────

fn builtin_value_counts(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "value_counts")?;
    let col_name = require_str(&args[1], "value_counts")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("value_counts(): column '{col_name}' not found"), None)
    })?;
    let mut keys: Vec<String> = Vec::new();
    let mut counts: Vec<i64> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    for row in &table.rows {
        let key = format!("{}", row[ci]);
        if let Some(pos) = keys.iter().position(|k| k == &key) {
            counts[pos] += 1;
        } else {
            keys.push(key);
            values.push(row[ci].clone());
            counts.push(1);
        }
    }
    // Sort by count descending
    let mut pairs: Vec<(Value, i64)> = values.into_iter().zip(counts).collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let rows: Vec<Vec<Value>> = pairs
        .into_iter()
        .map(|(v, c)| vec![v, Value::Int(c)])
        .collect();
    Ok(Value::Table(Table::new(
        vec!["value".into(), "count".into()],
        rows,
    )))
}

// ── describe(table) → summary stats per column ──────────────────────

fn builtin_describe(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "describe")?;
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for (ci, col) in table.columns.iter().enumerate() {
        let vals: Vec<&Value> = table.rows.iter().map(|r| &r[ci]).collect();
        let n = vals.len() as i64;
        let null_count = vals.iter().filter(|v| matches!(v, Value::Nil)).count() as i64;
        // Collect numeric values
        let nums: Vec<f64> = vals
            .iter()
            .filter_map(|v| match v {
                Value::Int(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            })
            .collect();
        let (mean, min, max, std_val) = if nums.is_empty() {
            (Value::Nil, Value::Nil, Value::Nil, Value::Nil)
        } else {
            let sum: f64 = nums.iter().sum();
            let mean = sum / nums.len() as f64;
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let variance = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nums.len() as f64;
            (
                Value::Float(mean),
                Value::Float(min),
                Value::Float(max),
                Value::Float(variance.sqrt()),
            )
        };
        // Determine dtype
        let dtype = if nums.len() == (n - null_count) as usize && !nums.is_empty() {
            if vals.iter().all(|v| matches!(v, Value::Int(_) | Value::Nil)) {
                "int"
            } else {
                "float"
            }
        } else if vals.iter().all(|v| matches!(v, Value::Str(_) | Value::Nil)) {
            "str"
        } else if vals.iter().all(|v| matches!(v, Value::Bool(_) | Value::Nil)) {
            "bool"
        } else {
            "mixed"
        };
        out_rows.push(vec![
            Value::Str(col.clone()),
            Value::Str(dtype.into()),
            Value::Int(n),
            Value::Int(null_count),
            mean,
            std_val,
            min,
            max,
        ]);
    }
    Ok(Value::Table(Table::new(
        vec![
            "column".into(),
            "dtype".into(),
            "count".into(),
            "null_count".into(),
            "mean".into(),
            "std".into(),
            "min".into(),
            "max".into(),
        ],
        out_rows,
    )))
}

// ── concat(table1, table2, ...) — vertical stack ────────────────────

fn builtin_concat(args: Vec<Value>) -> Result<Value> {
    let first = require_table(&args[0], "concat")?;
    let mut all_rows = first.rows.clone();
    for arg in &args[1..] {
        let t = require_table(arg, "concat")?;
        if t.columns != first.columns {
            return Err(BioLangError::type_error(
                "concat(): all tables must have the same columns",
                None,
            ));
        }
        all_rows.extend(t.rows.clone());
    }
    Ok(Value::Table(Table::new(first.columns.clone(), all_rows)))
}

// ── bind_cols(table1, table2, ...) — horizontal stack ───────────────

fn builtin_bind_cols(args: Vec<Value>) -> Result<Value> {
    let first = require_table(&args[0], "bind_cols")?;
    let n = first.num_rows();
    let mut all_cols = first.columns.clone();
    let mut col_data: Vec<Vec<Vec<Value>>> = vec![first.rows.clone()];
    for arg in &args[1..] {
        let t = require_table(arg, "bind_cols")?;
        if t.num_rows() != n {
            return Err(BioLangError::type_error(
                format!("bind_cols(): row count mismatch ({} vs {})", n, t.num_rows()),
                None,
            ));
        }
        all_cols.extend(t.columns.clone());
        col_data.push(t.rows.clone());
    }
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::new();
        for data in &col_data {
            row.extend(data[i].clone());
        }
        rows.push(row);
    }
    Ok(Value::Table(Table::new(all_cols, rows)))
}

// ── Additional joins ────────────────────────────────────────────────

fn builtin_right_join(args: Vec<Value>) -> Result<Value> {
    // right_join(a, b, key) = left_join(b, a, key)
    let reordered = vec![args[1].clone(), args[0].clone(), args[2].clone()];
    builtin_left_join(reordered)
}

fn builtin_outer_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "outer_join")?;
    let right = require_table(&args[1], "outer_join")?;
    let key_col = require_str(&args[2], "outer_join")?;
    let left_ki = left.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("outer_join(): key '{key_col}' not in left"), None)
    })?;
    let right_ki = right.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("outer_join(): key '{key_col}' not in right"), None)
    })?;
    let right_extra: Vec<usize> = (0..right.num_cols()).filter(|&i| i != right_ki).collect();
    let mut out_cols = left.columns.clone();
    for &i in &right_extra {
        out_cols.push(right.columns[i].clone());
    }
    // Build hash index on right table for O(n+m) join
    let mut right_index: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::with_capacity(right.rows.len());
    for (i, rrow) in right.rows.iter().enumerate() {
        let rkey = format!("{}", rrow[right_ki]);
        right_index.entry(rkey).or_default().push(i);
    }
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    let mut right_matched = vec![false; right.num_rows()];
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        if let Some(indices) = right_index.get(&lkey) {
            for &ri in indices {
                let rrow = &right.rows[ri];
                let mut row = lrow.clone();
                for &i in &right_extra {
                    row.push(rrow[i].clone());
                }
                out_rows.push(row);
                right_matched[ri] = true;
            }
        } else {
            let mut row = lrow.clone();
            for _ in &right_extra {
                row.push(Value::Nil);
            }
            out_rows.push(row);
        }
    }
    // Unmatched right rows
    for (ri, rrow) in right.rows.iter().enumerate() {
        if !right_matched[ri] {
            let mut row: Vec<Value> = left.columns.iter().enumerate().map(|(i, _)| {
                if i == left_ki { rrow[right_ki].clone() } else { Value::Nil }
            }).collect();
            for &i in &right_extra {
                row.push(rrow[i].clone());
            }
            out_rows.push(row);
        }
    }
    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

fn builtin_cross_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "cross_join")?;
    let right = require_table(&args[1], "cross_join")?;
    let product = (left.num_rows() as u64).saturating_mul(right.num_rows() as u64);
    if product > 10_000_000 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "cross_join() would produce {} rows ({} × {}). Limit: 10,000,000. \
                 Filter tables before joining.",
                product, left.num_rows(), right.num_rows()
            ),
            None,
        ));
    }
    let mut out_cols = left.columns.clone();
    out_cols.extend(right.columns.clone());
    let mut out_rows = Vec::with_capacity(product as usize);
    for lrow in &left.rows {
        for rrow in &right.rows {
            let mut row = lrow.clone();
            row.extend(rrow.clone());
            out_rows.push(row);
        }
    }
    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

fn builtin_anti_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "anti_join")?;
    let right = require_table(&args[1], "anti_join")?;
    let key_col = require_str(&args[2], "anti_join")?;
    let left_ki = left.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("anti_join(): key '{key_col}' not in left"), None)
    })?;
    let right_ki = right.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("anti_join(): key '{key_col}' not in right"), None)
    })?;
    let right_keys: std::collections::HashSet<String> = right.rows.iter().map(|r| format!("{}", r[right_ki])).collect();
    let rows: Vec<Vec<Value>> = left
        .rows
        .iter()
        .filter(|r| !right_keys.contains(&format!("{}", r[left_ki])))
        .cloned()
        .collect();
    Ok(Value::Table(Table::new(left.columns.clone(), rows)))
}

fn builtin_semi_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "semi_join")?;
    let right = require_table(&args[1], "semi_join")?;
    let key_col = require_str(&args[2], "semi_join")?;
    let left_ki = left.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("semi_join(): key '{key_col}' not in left"), None)
    })?;
    let right_ki = right.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("semi_join(): key '{key_col}' not in right"), None)
    })?;
    let right_keys: std::collections::HashSet<String> = right.rows.iter().map(|r| format!("{}", r[right_ki])).collect();
    let rows: Vec<Vec<Value>> = left
        .rows
        .iter()
        .filter(|r| right_keys.contains(&format!("{}", r[left_ki])))
        .cloned()
        .collect();
    Ok(Value::Table(Table::new(left.columns.clone(), rows)))
}

// ── explode(table, col) — unnest list column ────────────────────────

fn builtin_explode(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "explode")?;
    let col_name = require_str(&args[1], "explode")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("explode(): column '{col_name}' not found"), None)
    })?;
    let mut out_rows = Vec::new();
    for row in &table.rows {
        match &row[ci] {
            Value::List(items) => {
                for item in items {
                    let mut new_row = row.clone();
                    new_row[ci] = item.clone();
                    out_rows.push(new_row);
                }
            }
            _ => out_rows.push(row.clone()),
        }
    }
    Ok(Value::Table(Table::new(table.columns.clone(), out_rows)))
}

// ── Window functions ────────────────────────────────────────────────

fn builtin_row_number(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "row_number")?;
    let col: Vec<Value> = (1..=table.num_rows() as i64).map(Value::Int).collect();
    let mut cols = table.columns.clone();
    cols.push("row_number".into());
    let mut rows: Vec<Vec<Value>> = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(col[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_rank(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "rank")?;
    let col_name = require_str(&args[1], "rank")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("rank(): column '{col_name}' not found"), None)
    })?;
    // Sort indices by column value
    let mut indices: Vec<usize> = (0..table.num_rows()).collect();
    indices.sort_by(|&a, &b| val_cmp(&table.rows[a][ci], &table.rows[b][ci]));
    let mut ranks = vec![0i64; table.num_rows()];
    let mut current_rank = 1i64;
    for (pos, &idx) in indices.iter().enumerate() {
        if pos > 0 && val_cmp(&table.rows[indices[pos - 1]][ci], &table.rows[idx][ci]) != std::cmp::Ordering::Equal {
            current_rank = pos as i64 + 1;
        }
        ranks[idx] = current_rank;
    }
    let mut cols = table.columns.clone();
    cols.push("rank".into());
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(Value::Int(ranks[i]));
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn col_to_floats(table: &Table, ci: usize) -> Vec<Option<f64>> {
    table
        .rows
        .iter()
        .map(|r| match &r[ci] {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        })
        .collect()
}

fn builtin_lag(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "lag")?;
    let col_name = require_str(&args[1], "lag")?;
    let offset = if args.len() > 2 {
        match &args[2] { Value::Int(n) => *n as usize, _ => 1 }
    } else { 1 };
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("lag(): column '{col_name}' not found"), None)
    })?;
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_lag{offset}"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(if i >= offset { table.rows[i - offset][ci].clone() } else { Value::Nil });
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_lead(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "lead")?;
    let col_name = require_str(&args[1], "lead")?;
    let offset = if args.len() > 2 {
        match &args[2] { Value::Int(n) => *n as usize, _ => 1 }
    } else { 1 };
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("lead(): column '{col_name}' not found"), None)
    })?;
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_lead{offset}"));
    let mut rows = table.rows.clone();
    let n = rows.len();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(if i + offset < n { table.rows[i + offset][ci].clone() } else { Value::Nil });
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_cumsum(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cumsum")?;
    let col_name = require_str(&args[1], "cumsum")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("cumsum(): column '{col_name}' not found"), None)
    })?;
    let vals = col_to_floats(table, ci);
    let mut cum = 0.0;
    let cum_vals: Vec<Value> = vals
        .iter()
        .map(|v| match v {
            Some(f) => { cum += f; Value::Float(cum) }
            None => Value::Nil,
        })
        .collect();
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_cumsum"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(cum_vals[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_cummax(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cummax")?;
    let col_name = require_str(&args[1], "cummax")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("cummax(): column '{col_name}' not found"), None)
    })?;
    let vals = col_to_floats(table, ci);
    let mut max = f64::NEG_INFINITY;
    let cum_vals: Vec<Value> = vals
        .iter()
        .map(|v| match v {
            Some(f) => { if *f > max { max = *f; } Value::Float(max) }
            None => Value::Nil,
        })
        .collect();
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_cummax"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(cum_vals[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_cummin(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cummin")?;
    let col_name = require_str(&args[1], "cummin")?;
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("cummin(): column '{col_name}' not found"), None)
    })?;
    let vals = col_to_floats(table, ci);
    let mut min = f64::INFINITY;
    let cum_vals: Vec<Value> = vals
        .iter()
        .map(|v| match v {
            Some(f) => { if *f < min { min = *f; } Value::Float(min) }
            None => Value::Nil,
        })
        .collect();
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_cummin"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(cum_vals[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_rolling_mean(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "rolling_mean")?;
    let col_name = require_str(&args[1], "rolling_mean")?;
    let window = match &args[2] {
        Value::Int(n) => *n as usize,
        _ => return Err(BioLangError::type_error("rolling_mean() window must be Int", None)),
    };
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("rolling_mean(): column '{col_name}' not found"), None)
    })?;
    let vals = col_to_floats(table, ci);
    let mut result = Vec::with_capacity(vals.len());
    for i in 0..vals.len() {
        if i + 1 < window {
            result.push(Value::Nil);
        } else {
            let start = i + 1 - window;
            let slice: Vec<f64> = vals[start..=i].iter().filter_map(|v| *v).collect();
            if slice.len() == window {
                result.push(Value::Float(slice.iter().sum::<f64>() / window as f64));
            } else {
                result.push(Value::Nil);
            }
        }
    }
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_rmean{window}"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(result[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

fn builtin_rolling_sum(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "rolling_sum")?;
    let col_name = require_str(&args[1], "rolling_sum")?;
    let window = match &args[2] {
        Value::Int(n) => *n as usize,
        _ => return Err(BioLangError::type_error("rolling_sum() window must be Int", None)),
    };
    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, format!("rolling_sum(): column '{col_name}' not found"), None)
    })?;
    let vals = col_to_floats(table, ci);
    let mut result = Vec::with_capacity(vals.len());
    for i in 0..vals.len() {
        if i + 1 < window {
            result.push(Value::Nil);
        } else {
            let start = i + 1 - window;
            let slice: Vec<f64> = vals[start..=i].iter().filter_map(|v| *v).collect();
            if slice.len() == window {
                result.push(Value::Float(slice.iter().sum::<f64>()));
            } else {
                result.push(Value::Nil);
            }
        }
    }
    let mut cols = table.columns.clone();
    cols.push(format!("{col_name}_rsum{window}"));
    let mut rows = table.rows.clone();
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(result[i].clone());
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

// ── Display ─────────────────────────────────────────────────────────

fn builtin_col_width(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "col_width")?;
    let width = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("col_width() requires Int for width, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut table = Table::new(t.columns.clone(), t.rows.clone());
    table.max_col_width = Some(width);
    Ok(Value::Table(table))
}

// ── Column extraction & aggregation (native, no interpreter dispatch) ────

/// Helper: extract numeric values from a column in a Table or List of Records.
fn extract_col_floats(val: &Value, col: &str, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::Table(t) => {
            let ci = t.col_index(col).ok_or_else(|| {
                BioLangError::runtime(ErrorKind::NameError, format!("{func}(): column '{col}' not found"), None)
            })?;
            Ok(t.rows.iter().filter_map(|row| {
                match &row[ci] {
                    Value::Float(f) => Some(*f),
                    Value::Int(i) => Some(*i as f64),
                    _ => None,
                }
            }).collect())
        }
        Value::List(items) => {
            Ok(items.iter().filter_map(|item| {
                match item {
                    Value::Record(m) | Value::Map(m) => m.get(col).and_then(|v| match v {
                        Value::Float(f) => Some(*f),
                        Value::Int(i) => Some(*i as f64),
                        _ => None,
                    }),
                    _ => None,
                }
            }).collect())
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table or List, got {}", other.type_of()), None,
        )),
    }
}

/// Helper: extract all values from a column.
fn extract_col_values(val: &Value, col: &str, func: &str) -> Result<Vec<Value>> {
    match val {
        Value::Table(t) => {
            let ci = t.col_index(col).ok_or_else(|| {
                BioLangError::runtime(ErrorKind::NameError, format!("{func}(): column '{col}' not found"), None)
            })?;
            Ok(t.rows.iter().map(|row| row[ci].clone()).collect())
        }
        Value::List(items) => {
            Ok(items.iter().filter_map(|item| {
                match item {
                    Value::Record(m) | Value::Map(m) => m.get(col).cloned(),
                    _ => None,
                }
            }).collect())
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table or List, got {}", other.type_of()), None,
        )),
    }
}

fn builtin_col_values(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_values")?;
    Ok(Value::List(extract_col_values(&args[0], &col, "col_values")?))
}

fn builtin_col_mean(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_mean")?;
    let vals = extract_col_floats(&args[0], &col, "col_mean")?;
    if vals.is_empty() {
        return Ok(Value::Float(f64::NAN));
    }
    let sum: f64 = vals.iter().sum();
    Ok(Value::Float(sum / vals.len() as f64))
}

fn builtin_col_sum(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_sum")?;
    let vals = extract_col_floats(&args[0], &col, "col_sum")?;
    let sum: f64 = vals.iter().sum();
    // Return Int if all values were integers
    if sum == sum.floor() && sum.abs() < i64::MAX as f64 {
        Ok(Value::Int(sum as i64))
    } else {
        Ok(Value::Float(sum))
    }
}

fn builtin_col_stdev(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_stdev")?;
    let vals = extract_col_floats(&args[0], &col, "col_stdev")?;
    if vals.len() < 2 {
        return Ok(Value::Float(f64::NAN));
    }
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Ok(Value::Float(variance.sqrt()))
}

fn builtin_col_min(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_min")?;
    let vals = extract_col_floats(&args[0], &col, "col_min")?;
    Ok(Value::Float(vals.iter().copied().fold(f64::INFINITY, f64::min)))
}

fn builtin_col_max(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "col_max")?;
    let vals = extract_col_floats(&args[0], &col, "col_max")?;
    Ok(Value::Float(vals.iter().copied().fold(f64::NEG_INFINITY, f64::max)))
}

/// group_stats(grouped_map, col, "mean"|"sum"|"count"|"stdev"|"min"|"max")
/// Returns List<Record{key, value}> — native aggregation per group, no interpreter dispatch.
fn builtin_group_stats(args: Vec<Value>) -> Result<Value> {
    let col = require_str(&args[1], "group_stats")?;
    let agg = require_str(&args[2], "group_stats")?;

    let groups = match &args[0] {
        Value::Map(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("group_stats() requires Map (from group_by), got {}", other.type_of()), None,
            ));
        }
    };

    let mut results = Vec::with_capacity(groups.len());
    for (key, subtable) in groups {
        let vals = extract_col_floats(subtable, &col, "group_stats")?;
        let n = vals.len();
        let value = match agg.as_str() {
            "count" => Value::Int(n as i64),
            "sum" => {
                let s: f64 = vals.iter().sum();
                if s == s.floor() && s.abs() < i64::MAX as f64 { Value::Int(s as i64) } else { Value::Float(s) }
            }
            "mean" => {
                if n == 0 { Value::Float(f64::NAN) }
                else { Value::Float(vals.iter().sum::<f64>() / n as f64) }
            }
            "stdev" => {
                if n < 2 { Value::Float(f64::NAN) }
                else {
                    let mean = vals.iter().sum::<f64>() / n as f64;
                    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
                    Value::Float(var.sqrt())
                }
            }
            "min" => Value::Float(vals.iter().copied().fold(f64::INFINITY, f64::min)),
            "max" => Value::Float(vals.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("group_stats(): unknown aggregation '{other}', expected mean/sum/count/stdev/min/max"),
                    None,
                ));
            }
        };
        let mut rec = HashMap::new();
        rec.insert("key".to_string(), Value::Str(key.clone()));
        rec.insert("value".to_string(), value);
        results.push(Value::Record(rec));
    }
    Ok(Value::List(results))
}

// ── CSV/TSV I/O ─────────────────────────────────────────────────────

fn builtin_read_delimited(args: Vec<Value>, delim: char) -> Result<Value> {
    let path = require_str(&args[0], if delim == ',' { "csv" } else { "tsv" })?;
    let content = std::fs::read_to_string(&path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("cannot read file '{path}': {e}"),
            None,
        )
    })?;

    let mut lines = content.lines();

    // Parse header
    let header_line = lines.next().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "empty file", None)
    })?;
    let columns: Vec<String> = parse_delimited_line(header_line, delim)
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // Parse rows with type inference
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_delimited_line(line, delim);
        let row: Vec<Value> = fields
            .into_iter()
            .map(infer_value)
            .collect();
        rows.push(row);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Parse a delimited line, handling quoted fields.
fn parse_delimited_line(line: &str, delim: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped quote
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == delim {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

/// Infer a Value type from a string field: Int > Float > Bool > Str.
fn infer_value(s: String) -> Value {
    if s.is_empty() || s == "NA" || s == "na" || s == "." {
        return Value::Nil;
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Int(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }
    match s.as_str() {
        "true" | "TRUE" | "True" => Value::Bool(true),
        "false" | "FALSE" | "False" => Value::Bool(false),
        _ => Value::Str(s),
    }
}

fn builtin_write_delimited(args: Vec<Value>, delim: char) -> Result<Value> {
    let table = require_table(&args[0], if delim == ',' { "write_csv" } else { "write_tsv" })?;
    let path = require_str(&args[1], if delim == ',' { "write_csv" } else { "write_tsv" })?;
    let d = if delim == ',' { "," } else { "\t" };

    let mut output = String::new();
    // Header
    output.push_str(&table.columns.join(d));
    output.push('\n');
    // Rows
    for row in &table.rows {
        let fields: Vec<String> = row.iter().map(|v| format_csv_value(v, delim)).collect();
        output.push_str(&fields.join(d));
        output.push('\n');
    }

    std::fs::write(&path, &output).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("cannot write file '{path}': {e}"),
            None,
        )
    })?;

    Ok(Value::Nil)
}

fn format_csv_value(val: &Value, delim: char) -> String {
    match val {
        Value::Nil => String::new(),
        Value::Str(s) => {
            if s.contains(delim) || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        other => format!("{other}"),
    }
}

// ── Join operations ─────────────────────────────────────────────────

fn builtin_inner_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "inner_join")?;
    let right = require_table(&args[1], "inner_join")?;
    let key_col = require_str(&args[2], "inner_join")?;

    let left_ki = left.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("inner_join(): key column '{key_col}' not found in left table"),
            None,
        )
    })?;
    let right_ki = right.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("inner_join(): key column '{key_col}' not found in right table"),
            None,
        )
    })?;

    // Build output columns: left columns + right columns (excluding key)
    let mut out_cols = left.columns.clone();
    let right_col_indices: Vec<usize> = (0..right.num_cols())
        .filter(|&i| i != right_ki)
        .collect();
    for &i in &right_col_indices {
        out_cols.push(right.columns[i].clone());
    }

    // Build hash index on right table for O(n+m) join instead of O(n*m)
    let mut right_index: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::with_capacity(right.rows.len());
    for (i, rrow) in right.rows.iter().enumerate() {
        let rkey = format!("{}", rrow[right_ki]);
        right_index.entry(rkey).or_default().push(i);
    }

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        if let Some(indices) = right_index.get(&lkey) {
            for &ri in indices {
                let rrow = &right.rows[ri];
                let mut new_row = lrow.clone();
                for &i in &right_col_indices {
                    new_row.push(rrow[i].clone());
                }
                out_rows.push(new_row);
            }
        }
    }

    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

fn builtin_left_join(args: Vec<Value>) -> Result<Value> {
    let left = require_table(&args[0], "left_join")?;
    let right = require_table(&args[1], "left_join")?;
    let key_col = require_str(&args[2], "left_join")?;

    let left_ki = left.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("left_join(): key column '{key_col}' not found in left table"),
            None,
        )
    })?;
    let right_ki = right.col_index(&key_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("left_join(): key column '{key_col}' not found in right table"),
            None,
        )
    })?;

    let mut out_cols = left.columns.clone();
    let right_col_indices: Vec<usize> = (0..right.num_cols())
        .filter(|&i| i != right_ki)
        .collect();
    for &i in &right_col_indices {
        out_cols.push(right.columns[i].clone());
    }

    // Build hash index on right table for O(n+m) join
    let mut right_index: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::with_capacity(right.rows.len());
    for (i, rrow) in right.rows.iter().enumerate() {
        let rkey = format!("{}", rrow[right_ki]);
        right_index.entry(rkey).or_default().push(i);
    }

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        if let Some(indices) = right_index.get(&lkey) {
            for &ri in indices {
                let rrow = &right.rows[ri];
                let mut new_row = lrow.clone();
                for &i in &right_col_indices {
                    new_row.push(rrow[i].clone());
                }
                out_rows.push(new_row);
            }
        } else {
            let mut new_row = lrow.clone();
            for _ in &right_col_indices {
                new_row.push(Value::Nil);
            }
            out_rows.push(new_row);
        }
    }

    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

// ── Pivot operations ────────────────────────────────────────────────

fn builtin_pivot_longer(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "pivot_longer")?;
    let cols_to_pivot = match &args[1] {
        Value::List(items) => {
            let mut names = Vec::new();
            for item in items {
                names.push(require_str(item, "pivot_longer")?);
            }
            names
        }
        other => {
            return Err(BioLangError::type_error(
                format!("pivot_longer() cols must be List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let name_col = require_str(&args[2], "pivot_longer")?;
    let value_col = require_str(&args[3], "pivot_longer")?;

    // ID columns = columns NOT in cols_to_pivot
    let id_indices: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| !cols_to_pivot.contains(c))
        .map(|(i, _)| i)
        .collect();
    let pivot_indices: Vec<(usize, String)> = cols_to_pivot
        .iter()
        .filter_map(|c| table.col_index(c).map(|i| (i, c.clone())))
        .collect();

    let mut out_cols: Vec<String> = id_indices.iter().map(|&i| table.columns[i].clone()).collect();
    out_cols.push(name_col);
    out_cols.push(value_col);

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &table.rows {
        for (pi, pname) in &pivot_indices {
            let mut new_row: Vec<Value> = id_indices.iter().map(|&i| row[i].clone()).collect();
            new_row.push(Value::Str(pname.clone()));
            new_row.push(row[*pi].clone());
            out_rows.push(new_row);
        }
    }

    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

fn builtin_pivot_wider(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "pivot_wider")?;
    let name_col = require_str(&args[1], "pivot_wider")?;
    let value_col = require_str(&args[2], "pivot_wider")?;

    let name_ci = table.col_index(&name_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("pivot_wider(): column '{name_col}' not found"),
            None,
        )
    })?;
    let value_ci = table.col_index(&value_col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("pivot_wider(): column '{value_col}' not found"),
            None,
        )
    })?;

    // ID columns: everything except name_col and value_col
    let id_indices: Vec<usize> = (0..table.num_cols())
        .filter(|&i| i != name_ci && i != value_ci)
        .collect();

    // Collect unique values from name column (these become new columns)
    let mut new_col_names: Vec<String> = Vec::new();
    for row in &table.rows {
        let name = format!("{}", row[name_ci]);
        if !new_col_names.contains(&name) {
            new_col_names.push(name);
        }
    }

    let mut out_cols: Vec<String> = id_indices.iter().map(|&i| table.columns[i].clone()).collect();
    out_cols.extend(new_col_names.clone());

    // Group rows by ID columns
    let mut groups: Vec<(Vec<Value>, HashMap<String, Value>)> = Vec::new();
    for row in &table.rows {
        let id_vals: Vec<Value> = id_indices.iter().map(|&i| row[i].clone()).collect();
        let name = format!("{}", row[name_ci]);
        let value = row[value_ci].clone();

        if let Some(entry) = groups.iter_mut().find(|(ids, _)| *ids == id_vals) {
            entry.1.insert(name, value);
        } else {
            let mut map = HashMap::new();
            map.insert(name, value);
            groups.push((id_vals, map));
        }
    }

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for (id_vals, name_vals) in &groups {
        let mut row = id_vals.clone();
        for col_name in &new_col_names {
            row.push(name_vals.get(col_name).cloned().unwrap_or(Value::Nil));
        }
        out_rows.push(row);
    }

    Ok(Value::Table(Table::new(out_cols, out_rows)))
}

// ── bio_join(left, right, [on], [join_type]) ─────────────────────────
// Biologically-aware join for multi-omics data. Auto-detects join key
// from common bio columns, supports case-insensitive gene symbol matching
// and Ensembl ID version stripping.

/// Priority list of biologically significant column names for auto-detection.
const BIO_JOIN_KEYS: &[&str] = &[
    "gene", "gene_id", "gene_name", "symbol", "ensembl_id",
    "uniprot_id", "accession", "id", "name",
];

/// Normalize a biological identifier for matching:
/// - lowercase for case-insensitive gene symbol matching
/// - strip Ensembl version suffixes (ENSG00000012048.16 -> ensg00000012048)
fn bio_normalize_key(val: &Value) -> String {
    let s = format!("{}", val);
    let lower = s.to_lowercase();
    // Strip version suffix for Ensembl-style IDs (ENS + species code + digits + .version)
    if lower.starts_with("ens") {
        if let Some(dot_pos) = lower.rfind('.') {
            let after_dot = &lower[dot_pos + 1..];
            if !after_dot.is_empty() && after_dot.chars().all(|c| c.is_ascii_digit()) {
                return lower[..dot_pos].to_string();
            }
        }
    }
    lower
}

/// Convert a Value (Table or List of Records) into a Table.
fn coerce_to_table(val: &Value, func: &str) -> Result<Table> {
    match val {
        Value::Table(t) => Ok(t.clone()),
        Value::List(items) => {
            if items.is_empty() {
                return Ok(Table::new(vec![], vec![]));
            }
            let first = match &items[0] {
                Value::Record(m) | Value::Map(m) => m,
                other => {
                    return Err(BioLangError::type_error(
                        format!("{func}() requires Table or List of Records, got List of {}", other.type_of()),
                        None,
                    ))
                }
            };
            let columns: Vec<String> = first.keys().cloned().collect();
            let mut rows = Vec::new();
            for item in items {
                let map = match item {
                    Value::Record(m) | Value::Map(m) => m,
                    _ => continue,
                };
                let row: Vec<Value> = columns
                    .iter()
                    .map(|c| map.get(c).cloned().unwrap_or(Value::Nil))
                    .collect();
                rows.push(row);
            }
            Ok(Table::new(columns, rows))
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table or List of Records, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_bio_join(args: Vec<Value>) -> Result<Value> {
    let left = coerce_to_table(&args[0], "bio_join")?;
    let right = coerce_to_table(&args[1], "bio_join")?;

    // Determine join key
    let key_col = if args.len() >= 3 {
        require_str(&args[2], "bio_join")?
    } else {
        // Auto-detect: find first column name from priority list present in both tables
        let left_cols: std::collections::HashSet<String> =
            left.columns.iter().map(|c| c.to_lowercase()).collect();
        let right_cols: std::collections::HashSet<String> =
            right.columns.iter().map(|c| c.to_lowercase()).collect();

        let mut detected: Option<String> = None;
        for &candidate in BIO_JOIN_KEYS {
            if left_cols.contains(candidate) && right_cols.contains(candidate) {
                // Return the original-case column name from the left table
                detected = left
                    .columns
                    .iter()
                    .find(|c| c.to_lowercase() == candidate)
                    .cloned();
                break;
            }
        }
        detected.ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "bio_join(): no common biological key column found; specify one explicitly".to_string(),
                None,
            )
        })?
    };

    // Determine join type
    let join_type = if args.len() >= 4 {
        let jt = require_str(&args[3], "bio_join")?;
        match jt.to_lowercase().as_str() {
            "inner" | "left" | "right" | "outer" => jt.to_lowercase(),
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("bio_join(): unknown join type '{other}'; expected inner, left, right, or outer"),
                    None,
                ))
            }
        }
    } else {
        "inner".to_string()
    };

    // Find key column indices (case-insensitive search)
    let key_lower = key_col.to_lowercase();
    let left_ki = left
        .columns
        .iter()
        .position(|c| c.to_lowercase() == key_lower)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                format!("bio_join(): key column '{key_col}' not found in left input"),
                None,
            )
        })?;
    let right_ki = right
        .columns
        .iter()
        .position(|c| c.to_lowercase() == key_lower)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                format!("bio_join(): key column '{key_col}' not found in right input"),
                None,
            )
        })?;

    // Build output columns: left columns + right columns (excluding key)
    let mut out_cols = left.columns.clone();
    let right_col_indices: Vec<usize> = (0..right.num_cols())
        .filter(|&i| i != right_ki)
        .collect();
    for &i in &right_col_indices {
        let col = &right.columns[i];
        // Avoid duplicate column names by suffixing with _right
        if out_cols.iter().any(|c| c == col) {
            out_cols.push(format!("{col}_right"));
        } else {
            out_cols.push(col.clone());
        }
    }

    // Build hash index on right table using bio-normalized keys
    let mut right_index: HashMap<String, Vec<usize>> =
        HashMap::with_capacity(right.rows.len());
    for (i, rrow) in right.rows.iter().enumerate() {
        let rkey = bio_normalize_key(&rrow[right_ki]);
        right_index.entry(rkey).or_default().push(i);
    }

    let right_fill_count = right_col_indices.len();
    let left_fill_count = left.num_cols();
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    let mut right_matched: Vec<bool> = if join_type == "outer" || join_type == "right" {
        vec![false; right.rows.len()]
    } else {
        vec![]
    };

    for lrow in &left.rows {
        let lkey = bio_normalize_key(&lrow[left_ki]);
        if let Some(indices) = right_index.get(&lkey) {
            for &ri in indices {
                if !right_matched.is_empty() {
                    right_matched[ri] = true;
                }
                let rrow = &right.rows[ri];
                let mut new_row = lrow.clone();
                for &i in &right_col_indices {
                    new_row.push(rrow[i].clone());
                }
                out_rows.push(new_row);
            }
        } else if join_type == "left" || join_type == "outer" {
            let mut new_row = lrow.clone();
            for _ in 0..right_fill_count {
                new_row.push(Value::Nil);
            }
            out_rows.push(new_row);
        }
        // inner/right: skip unmatched left rows
    }

    // For right/outer joins, add unmatched right rows
    if join_type == "right" || join_type == "outer" {
        for (ri, matched) in right_matched.iter().enumerate() {
            if !matched {
                let rrow = &right.rows[ri];
                let mut new_row: Vec<Value> = (0..left_fill_count)
                    .map(|i| {
                        if i == left_ki {
                            rrow[right_ki].clone()
                        } else {
                            Value::Nil
                        }
                    })
                    .collect();
                for &i in &right_col_indices {
                    new_row.push(rrow[i].clone());
                }
                out_rows.push(new_row);
            }
        }
    }

    // Return as List of Records for easy field access in pipes
    let records: Vec<Value> = out_rows
        .iter()
        .map(|row| {
            let mut map = HashMap::new();
            for (i, col) in out_cols.iter().enumerate() {
                map.insert(col.clone(), row.get(i).cloned().unwrap_or(Value::Nil));
            }
            Value::Record(map)
        })
        .collect();

    Ok(Value::List(records))
}
