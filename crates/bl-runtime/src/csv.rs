use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn csv_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("csv", Arity::Range(1, 2)),
        ("tsv", Arity::Exact(1)),
        ("write_csv", Arity::Range(2, 3)),
        ("write_tsv", Arity::Exact(2)),
    ]
}

pub fn is_csv_builtin(name: &str) -> bool {
    matches!(name, "csv" | "tsv" | "write_csv" | "write_tsv")
}

pub fn call_csv_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "csv" => builtin_csv(args),
        "tsv" => builtin_tsv(args),
        "write_csv" => builtin_write_csv(args),
        "write_tsv" => builtin_write_tsv(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown csv builtin '{name}'"),
            None,
        )),
    }
}

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

/// Read a delimited file into a Table.
fn read_delimited(path: &str, sep: &str, has_header: bool) -> Result<Value> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("csv: cannot read '{path}': {e}"), None)
    })?;

    let mut lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(Value::Table(Table::empty()));
    }

    // Remove BOM if present
    if let Some(first) = lines.first_mut() {
        if first.starts_with('\u{feff}') {
            *first = &first[3..];
        }
    }

    let col_names: Vec<String>;
    let data_start;

    if has_header {
        col_names = parse_csv_line(lines[0], sep);
        data_start = 1;
    } else {
        // Auto-generate column names
        let ncols = parse_csv_line(lines[0], sep).len();
        col_names = (0..ncols).map(|i| format!("col{}", i + 1)).collect();
        data_start = 0;
    }

    let ncols = col_names.len();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for line in &lines[data_start..] {
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_line(line, sep);
        let mut row = Vec::with_capacity(ncols);
        for i in 0..ncols {
            let val = fields.get(i).map(|s| s.as_str()).unwrap_or("");
            row.push(parse_field(val));
        }
        rows.push(row);
    }

    let table = Table::new(col_names, rows);
    Ok(Value::Table(table))
}

/// Parse a single CSV line, handling quoted fields.
fn parse_csv_line(line: &str, sep: &str) -> Vec<String> {
    if sep.len() == 1 {
        let sep_char = sep.chars().next().unwrap();
        parse_csv_line_char(line, sep_char)
    } else {
        // Simple split for multi-char separators
        line.split(sep).map(|s| s.trim().to_string()).collect()
    }
}

fn parse_csv_line_char(line: &str, sep: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped quote
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == sep {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

/// Parse a field value, auto-detecting type.
fn parse_field(s: &str) -> Value {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "NA" || trimmed == "na" || trimmed == "null" || trimmed == "." {
        return Value::Nil;
    }
    if trimmed == "true" || trimmed == "TRUE" {
        return Value::Bool(true);
    }
    if trimmed == "false" || trimmed == "FALSE" {
        return Value::Bool(false);
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Int(i);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }
    Value::Str(trimmed.to_string())
}

fn builtin_csv(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "csv")?;
    let (sep, has_header) = if args.len() > 1 {
        extract_csv_options(&args[1], path)?
    } else {
        let sep = if path.ends_with(".tsv") || path.ends_with(".tab") {
            "\t".to_string()
        } else {
            ",".to_string()
        };
        (sep, true)
    };
    read_delimited(path, &sep, has_header)
}

fn builtin_tsv(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "tsv")?;
    read_delimited(path, "\t", true)
}

fn extract_csv_options(opts: &Value, path: &str) -> Result<(String, bool)> {
    match opts {
        Value::Record(m) | Value::Map(m) => {
            let sep = m
                .get("sep")
                .and_then(|v| match v {
                    Value::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    if path.ends_with(".tsv") || path.ends_with(".tab") {
                        "\t".to_string()
                    } else {
                        ",".to_string()
                    }
                });
            let has_header = m
                .get("header")
                .and_then(|v| match v {
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(true);
            Ok((sep, has_header))
        }
        _ => Err(BioLangError::type_error(
            "csv() options must be a record like {sep: \",\", header: true}",
            None,
        )),
    }
}

/// Write a Table to a delimited file.
fn write_delimited(table: &Table, path: &str, sep: &str, write_header: bool) -> Result<Value> {
    use std::io::Write;
    let mut file = std::fs::File::create(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("write_csv: cannot create '{path}': {e}"), None)
    })?;

    let nrows = table.num_rows();

    if write_header {
        let header_line: Vec<&str> = table.columns.iter().map(|s| s.as_str()).collect();
        writeln!(file, "{}", header_line.join(sep)).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("write_csv: {e}"), None)
        })?;
    }

    for row in &table.rows {
        let fields: Vec<String> = row.iter().map(|v| format_csv_field(v, sep)).collect();
        writeln!(file, "{}", fields.join(sep)).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("write_csv: {e}"), None)
        })?;
    }

    let mut result = HashMap::new();
    result.insert("rows".to_string(), Value::Int(nrows as i64));
    result.insert("cols".to_string(), Value::Int(table.columns.len() as i64));
    result.insert("output".to_string(), Value::Str(path.to_string()));
    Ok(Value::Record(result))
}

fn format_csv_field(val: &Value, sep: &str) -> String {
    match val {
        Value::Nil => String::new(),
        Value::Str(s) => {
            if s.contains(sep) || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        other => format!("{other}"),
    }
}

fn builtin_write_csv(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("write_csv() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let path = require_str(&args[1], "write_csv")?;
    let (sep, write_header) = if args.len() > 2 {
        extract_write_options(&args[2])?
    } else {
        (",".to_string(), true)
    };
    write_delimited(table, path, &sep, write_header)
}

fn builtin_write_tsv(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("write_tsv() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let path = require_str(&args[1], "write_tsv")?;
    write_delimited(table, path, "\t", true)
}

fn extract_write_options(opts: &Value) -> Result<(String, bool)> {
    match opts {
        Value::Record(m) | Value::Map(m) => {
            let sep = m
                .get("sep")
                .and_then(|v| match v {
                    Value::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| ",".to_string());
            let header = m
                .get("header")
                .and_then(|v| match v {
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(true);
            Ok((sep, header))
        }
        _ => Err(BioLangError::type_error(
            "write_csv() options must be a record",
            None,
        )),
    }
}
