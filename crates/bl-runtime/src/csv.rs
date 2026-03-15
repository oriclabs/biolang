use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, StreamValue, Table, Value};
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

/// Hook for fetching URL content (used by WASM to load CSV from same-origin HTTP).
/// Set via `set_fetch_hook()` from the WASM bridge.
thread_local! {
    static FETCH_HOOK: std::cell::RefCell<Option<Arc<dyn Fn(&str) -> std::result::Result<String, String>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Set the fetch hook (called from br-wasm to enable HTTP CSV loading).
pub fn set_fetch_hook(hook: Option<Arc<dyn Fn(&str) -> std::result::Result<String, String>>>) {
    FETCH_HOOK.with(|h| *h.borrow_mut() = hook);
}

pub fn try_fetch_url(url: &str) -> Option<std::result::Result<String, String>> {
    FETCH_HOOK.with(|h| {
        let hook = h.borrow();
        hook.as_ref().map(|f| f(url))
    })
}

pub fn csv_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("csv", Arity::Range(1, 2)),
        ("read_csv", Arity::Range(1, 2)),
        ("tsv", Arity::Exact(1)),
        ("read_tsv", Arity::Exact(1)),
        ("write_csv", Arity::Range(2, 3)),
        ("write_tsv", Arity::Exact(2)),
    ]
}

pub fn is_csv_builtin(name: &str) -> bool {
    matches!(name, "csv" | "read_csv" | "tsv" | "read_tsv" | "write_csv" | "write_tsv")
}

pub fn call_csv_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "csv" => builtin_csv(args),
        "read_csv" => builtin_read_csv_eager(args),
        "tsv" => builtin_tsv(args),
        "read_tsv" => builtin_read_tsv_eager(args),
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

/// Read a delimited file into a Table (legacy: reads entire file into string).
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

/// Optimized buffered CSV reader: single-pass line-by-line, no full-file string allocation.
fn read_delimited_buffered(path: &str, sep: &str, has_header: bool) -> Result<Value> {
    use std::io::BufRead;

    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("csv: cannot open '{path}': {e}"), None)
    })?;
    let mut reader: Box<dyn BufRead> = if path.ends_with(".gz") || path.ends_with(".csv.gz") || path.ends_with(".tsv.gz") {
        #[cfg(feature = "native")]
        {
            Box::new(std::io::BufReader::with_capacity(128 * 1024, flate2::read::GzDecoder::new(file)))
        }
        #[cfg(not(feature = "native"))]
        {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("csv: gzip support requires native feature: '{path}'"),
                None,
            ));
        }
    } else {
        Box::new(std::io::BufReader::with_capacity(128 * 1024, file))
    };
    let sep_char = sep.chars().next().unwrap_or(',');

    let mut line_buf = String::new();

    // Read first line (header or first data row)
    line_buf.clear();
    let bytes = reader.read_line(&mut line_buf).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("csv: read error: {e}"), None)
    })?;
    if bytes == 0 {
        return Ok(Value::Table(Table::empty()));
    }

    let first_clean = {
        let trimmed = line_buf.trim_end();
        if trimmed.starts_with('\u{feff}') { &trimmed[3..] } else { trimmed }
    };

    let col_names: Vec<String>;
    let mut rows: Vec<Vec<Value>> = Vec::new();

    if has_header {
        col_names = parse_csv_line_char(first_clean, sep_char);
    } else {
        let fields = parse_csv_line_char(first_clean, sep_char);
        let ncols = fields.len();
        col_names = (0..ncols).map(|i| format!("col{}", i + 1)).collect();
        let mut row = Vec::with_capacity(ncols);
        for f in &fields {
            row.push(parse_field(f));
        }
        rows.push(row);
    }

    let ncols = col_names.len();

    // Fast path: check if file likely has no quoted fields
    // For simple CSVs, we can use split directly instead of char-by-char parsing
    loop {
        line_buf.clear();
        let bytes = reader.read_line(&mut line_buf).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("csv: read error: {e}"), None)
        })?;
        if bytes == 0 { break; }
        let line = line_buf.trim_end();
        if line.is_empty() { continue; }

        // Fast path: no quotes in line — use split directly
        if !line.contains('"') {
            let mut row = Vec::with_capacity(ncols);
            let mut count = 0;
            for field in line.split(sep_char) {
                if count < ncols {
                    row.push(parse_field(field));
                    count += 1;
                }
            }
            // Pad with Nil if fewer fields
            while count < ncols {
                row.push(Value::Nil);
                count += 1;
            }
            rows.push(row);
        } else {
            // Slow path: handle quoted fields
            let fields = parse_csv_line_char(line, sep_char);
            let mut row = Vec::with_capacity(ncols);
            for i in 0..ncols {
                let val = fields.get(i).map(|s| s.as_str()).unwrap_or("");
                row.push(parse_field(val));
            }
            rows.push(row);
        }
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
        let opts = extract_csv_options(&args[1], path)?;
        // Check for explicit {stream: true} to get streaming mode
        if wants_stream_csv(&args[1]) {
            return read_csv_stream(path, &opts.0, opts.1);
        }
        opts
    } else {
        let sep = if path.ends_with(".tsv") || path.ends_with(".tab") || path.ends_with(".tsv.gz") {
            "\t".to_string()
        } else {
            ",".to_string()
        };
        (sep, true)
    };
    read_delimited_buffered(path, &sep, has_header)
}

fn builtin_read_csv_eager(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "read_csv")?;
    let (sep, has_header) = if args.len() > 1 {
        extract_csv_options(&args[1], path)?
    } else {
        let sep = if path.ends_with(".tsv") || path.ends_with(".tab") || path.ends_with(".tsv.gz") {
            "\t".to_string()
        } else {
            ",".to_string()
        };
        (sep, true)
    };

    // HTTP/HTTPS URL: use fetch hook (WASM) or ureq (native)
    if path.starts_with("http://") || path.starts_with("https://") {
        return read_csv_from_url(path, &sep, has_header);
    }

    // For local paths, try the fetch hook first (enables WASM local file access
    // via the __blFetch.sync JS hook that checks in-memory file registries).
    if let Some(result) = try_fetch_url(path) {
        match result {
            Ok(text) if !text.starts_with("ERROR:") => {
                return parse_csv_text(&text, &sep, has_header);
            }
            _ => {} // Fall through to filesystem
        }
    }

    read_delimited_buffered(path, &sep, has_header)
}

fn read_csv_from_url(url: &str, sep: &str, has_header: bool) -> Result<Value> {
    // Try the WASM fetch hook first
    if let Some(result) = try_fetch_url(url) {
        let text = result.map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("read_csv: fetch error for '{url}': {e}"), None)
        })?;
        return parse_csv_text(&text, sep, has_header);
    }

    // Native fallback: try ureq if available
    #[cfg(feature = "native")]
    {
        let resp = ureq::get(url).call().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("read_csv: HTTP error for '{url}': {e}"), None)
        })?;
        let text = resp.into_string().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("read_csv: read error for '{url}': {e}"), None)
        })?;
        return parse_csv_text(&text, sep, has_header);
    }

    #[cfg(not(feature = "native"))]
    {
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_csv: no fetch hook available for URL '{url}'"),
            None,
        ))
    }
}

/// Parse CSV from an in-memory string.
fn parse_csv_text(text: &str, sep: &str, has_header: bool) -> Result<Value> {
    let sep_char = sep.chars().next().unwrap_or(',');
    let mut lines = text.lines();

    let first_line = match lines.next() {
        Some(l) => l,
        None => return Ok(Value::Table(Table::empty())),
    };
    let first_clean = if first_line.starts_with('\u{feff}') { &first_line[3..] } else { first_line };

    let col_names: Vec<String>;
    let mut rows: Vec<Vec<Value>> = Vec::new();

    if has_header {
        col_names = split_csv_line(first_clean, sep_char).into_iter().map(|s| s.to_string()).collect();
    } else {
        let fields = split_csv_line(first_clean, sep_char);
        col_names = (0..fields.len()).map(|i| format!("col_{}", i + 1)).collect();
        rows.push(fields.into_iter().map(|f| infer_value(f)).collect());
    }

    for line in lines {
        let trimmed = line.trim_end();
        if trimmed.is_empty() { continue; }
        let fields = split_csv_line(trimmed, sep_char);
        rows.push(fields.into_iter().map(|f| infer_value(f)).collect());
    }

    let ncols = col_names.len();
    let mut columns: Vec<Vec<Value>> = vec![Vec::with_capacity(rows.len()); ncols];
    for row in &rows {
        for (c, val) in row.iter().enumerate() {
            if c < ncols {
                columns[c].push(val.clone());
            }
        }
    }

    Ok(Value::Table(Table {
        columns: col_names,
        rows: columns,
        max_col_width: None,
    }))
}

fn split_csv_line(line: &str, sep: char) -> Vec<&str> {
    // Simple split (doesn't handle quoted fields with embedded separators)
    line.split(sep).collect()
}

fn infer_value(s: &str) -> Value {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "NA" || trimmed == "na" || trimmed == "N/A" || trimmed == "." {
        return Value::Nil;
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Int(i);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }
    if trimmed == "true" || trimmed == "TRUE" {
        return Value::Bool(true);
    }
    if trimmed == "false" || trimmed == "FALSE" {
        return Value::Bool(false);
    }
    Value::Str(trimmed.to_string())
}

fn builtin_tsv(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "tsv")?;
    read_delimited_buffered(path, "\t", true)
}

fn builtin_read_tsv_eager(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "read_tsv")?;
    read_delimited_buffered(path, "\t", true)
}

fn wants_stream_csv(opts: &Value) -> bool {
    match opts {
        Value::Record(m) | Value::Map(m) => {
            m.get("stream").map(|v| v.is_truthy()).unwrap_or(false)
        }
        _ => false,
    }
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
                    if path.ends_with(".tsv") || path.ends_with(".tab") || path.ends_with(".tsv.gz") {
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

// ── Streaming CSV/TSV iterator ──────────────────────────────────

struct CsvIter {
    lines: std::io::Lines<Box<dyn BufRead + Send>>,
    col_names: Vec<String>,
    sep: char,
}

impl Iterator for CsvIter {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        loop {
            let line = self.lines.next()?.ok()?;
            if line.is_empty() {
                continue;
            }
            let fields = parse_csv_line_char(&line, self.sep);
            let mut map = HashMap::new();
            for (i, col) in self.col_names.iter().enumerate() {
                let val = fields.get(i).map(|s| s.as_str()).unwrap_or("");
                map.insert(col.clone(), parse_field(val));
            }
            return Some(Value::Record(map));
        }
    }
}

fn read_csv_stream(path: &str, sep: &str, has_header: bool) -> Result<Value> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("csv: cannot open '{path}': {e}"), None)
    })?;
    let mut reader: Box<dyn BufRead + Send> = if path.ends_with(".gz") {
        #[cfg(feature = "native")]
        {
            Box::new(std::io::BufReader::new(flate2::read::GzDecoder::new(file)))
        }
        #[cfg(not(feature = "native"))]
        {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("csv: gzip support requires native feature: '{path}'"),
                None,
            ));
        }
    } else {
        Box::new(std::io::BufReader::new(file))
    };

    let sep_char = sep.chars().next().unwrap_or(',');

    let col_names = if has_header {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("csv: cannot read header: {e}"), None)
        })?;
        // Remove BOM if present
        let trimmed = if header_line.starts_with('\u{feff}') {
            &header_line[3..]
        } else {
            &header_line
        };
        parse_csv_line_char(trimmed.trim_end_matches('\n').trim_end_matches('\r'), sep_char)
    } else {
        // Read first line to determine column count, then chain it back
        let mut first_line = String::new();
        reader.read_line(&mut first_line).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("csv: cannot read first line: {e}"), None)
        })?;
        let first_trimmed = first_line.trim_end_matches('\n').trim_end_matches('\r');
        let ncols = parse_csv_line_char(first_trimmed, sep_char).len();
        let cols: Vec<String> = (0..ncols).map(|i| format!("col{}", i + 1)).collect();
        // Chain the first line back with the rest via Read::chain
        use std::io::Read;
        let first_bytes = first_line.into_bytes();
        let chain = std::io::Cursor::new(first_bytes).chain(reader);
        let chained_reader: Box<dyn BufRead + Send> = Box::new(std::io::BufReader::new(chain));
        let label = format!("csv:{path}");
        return Ok(Value::Stream(StreamValue::new(
            label,
            Box::new(CsvIter {
                lines: chained_reader.lines(),
                col_names: cols,
                sep: sep_char,
            }),
        )));
    };

    let label = format!("csv:{path}");
    Ok(Value::Stream(StreamValue::new(
        label,
        Box::new(CsvIter {
            lines: reader.lines(),
            col_names,
            sep: sep_char,
        }),
    )))
}
