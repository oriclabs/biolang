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
        ("value_counts", Arity::Exact(2)),
        ("describe", Arity::Exact(1)),
        // Combine
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
            | "head"
            | "tail"
            | "slice"
            | "sample"
            | "arrange"
            | "distinct"
            | "fill_null"
            | "drop_null"
            | "group_by"
            | "value_counts"
            | "describe"
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
        "head" => builtin_head(args),
        "tail" => builtin_tail(args),
        "slice" => builtin_slice(args),
        "sample" => builtin_sample(args),
        "arrange" => builtin_arrange(args),
        "distinct" => builtin_distinct(args),
        "fill_null" => builtin_fill_null(args),
        "drop_null" => builtin_drop_null(args),
        "group_by" => builtin_group_by(args),
        "value_counts" => builtin_value_counts(args),
        "describe" => builtin_describe(args),
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

// ── arrange(table, col, ...) — prefix "-" for desc ──────────────────

fn builtin_arrange(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "arrange")?;

    // Parse sort specs: each is a column name, optionally prefixed with "-"
    let mut specs: Vec<(usize, bool)> = Vec::new(); // (col_index, ascending)
    for arg in &args[1..] {
        let s = require_str(arg, "arrange")?;
        let (name, asc) = if let Some(stripped) = s.strip_prefix('-') {
            (stripped, false)
        } else {
            (s.as_str(), true)
        };
        match table.col_index(name) {
            Some(i) => specs.push((i, asc)),
            None => {
                return Err(BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("arrange(): column '{name}' not found"),
                    None,
                ))
            }
        }
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

// ── group_by(table, col) → Map<String, Table> ──────────────────────

fn builtin_group_by(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "group_by")?;
    let col_name = require_str(&args[1], "group_by")?;

    let ci = table.col_index(&col_name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("group_by(): column '{col_name}' not found"),
            None,
        )
    })?;

    let mut group_keys: Vec<String> = Vec::new();
    let mut group_rows: Vec<Vec<Vec<Value>>> = Vec::new();
    for row in &table.rows {
        let key = format!("{}", row[ci]);
        if let Some(pos) = group_keys.iter().position(|k| k == &key) {
            group_rows[pos].push(row.clone());
        } else {
            group_keys.push(key);
            group_rows.push(vec![row.clone()]);
        }
    }

    let mut map = HashMap::new();
    for (key, rows) in group_keys.into_iter().zip(group_rows) {
        map.insert(key, Value::Table(Table::new(table.columns.clone(), rows)));
    }
    Ok(Value::Map(map))
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
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    let mut right_matched = vec![false; right.num_rows()];
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        let mut matched = false;
        for (ri, rrow) in right.rows.iter().enumerate() {
            if format!("{}", rrow[right_ki]) == lkey {
                let mut row = lrow.clone();
                for &i in &right_extra {
                    row.push(rrow[i].clone());
                }
                out_rows.push(row);
                right_matched[ri] = true;
                matched = true;
            }
        }
        if !matched {
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
    let mut out_cols = left.columns.clone();
    out_cols.extend(right.columns.clone());
    let mut out_rows = Vec::new();
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
    let right_keys: Vec<String> = right.rows.iter().map(|r| format!("{}", r[right_ki])).collect();
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
    let right_keys: Vec<String> = right.rows.iter().map(|r| format!("{}", r[right_ki])).collect();
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

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        for rrow in &right.rows {
            let rkey = format!("{}", rrow[right_ki]);
            if lkey == rkey {
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

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for lrow in &left.rows {
        let lkey = format!("{}", lrow[left_ki]);
        let mut matched = false;
        for rrow in &right.rows {
            let rkey = format!("{}", rrow[right_ki]);
            if lkey == rkey {
                let mut new_row = lrow.clone();
                for &i in &right_col_indices {
                    new_row.push(rrow[i].clone());
                }
                out_rows.push(new_row);
                matched = true;
            }
        }
        if !matched {
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
