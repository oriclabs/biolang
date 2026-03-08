use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, StreamValue, Value};

use std::collections::HashMap;

/// Returns the list of (name, arity) for all text processing builtins.
pub fn text_builtin_list() -> Vec<(&'static str, Arity)> {
    let list = vec![
        ("grep", Arity::Range(2, 3)),
        ("grep_count", Arity::Exact(2)),
        ("lines", Arity::Exact(1)),
        ("cut", Arity::Exact(3)),
        ("paste", Arity::Range(2, 3)),
        ("uniq_count", Arity::Exact(1)),
        ("wc", Arity::Exact(1)),
        #[cfg(feature = "native")]
        ("tee", Arity::Exact(2)),
        #[cfg(feature = "native")]
        ("shell", Arity::Range(1, 2)),
        #[cfg(feature = "native")]
        ("count_lines", Arity::Exact(1)),
        #[cfg(feature = "native")]
        ("stream_lines", Arity::Exact(1)),
        ("stream_concat", Arity::Exact(2)),
    ];
    list
}

/// Check if a name is a known text processing builtin.
pub fn is_text_builtin(name: &str) -> bool {
    matches!(
        name,
        "grep" | "grep_count" | "lines" | "cut" | "paste" | "uniq_count" | "wc" | "stream_concat"
    ) || is_native_text_builtin(name)
}

#[cfg(feature = "native")]
fn is_native_text_builtin(name: &str) -> bool {
    matches!(name, "tee" | "shell" | "count_lines" | "stream_lines")
}

#[cfg(not(feature = "native"))]
fn is_native_text_builtin(_name: &str) -> bool {
    false
}

/// Execute a text processing builtin by name.
pub fn call_text_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "grep" => builtin_grep(args),
        "grep_count" => builtin_grep_count(args),
        "lines" => builtin_lines(args),
        "cut" => builtin_cut(args),
        "paste" => builtin_paste(args),
        "uniq_count" => builtin_uniq_count(args),
        "wc" => builtin_wc(args),
        #[cfg(feature = "native")]
        "tee" => builtin_tee(args),
        #[cfg(feature = "native")]
        "shell" => builtin_shell(args),
        #[cfg(feature = "native")]
        "count_lines" => builtin_count_lines(args),
        #[cfg(feature = "native")]
        "stream_lines" => builtin_stream_lines(args),
        "stream_concat" => builtin_stream_concat(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown text builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn compile_regex(pattern: &str, func: &str) -> Result<regex::Regex> {
    regex::Regex::new(pattern).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{func}() invalid regex: {e}"),
            None,
        )
    })
}

/// Resolve input to lines: auto-detects file path (native), List, or Str.
fn resolve_input_lines(input: &Value, func: &str) -> Result<Vec<String>> {
    match input {
        Value::List(items) => {
            let mut lines = Vec::with_capacity(items.len());
            for item in items {
                lines.push(format!("{item}"));
            }
            Ok(lines)
        }
        Value::Str(s) => {
            #[cfg(feature = "native")]
            {
                let path = std::path::Path::new(s.as_str());
                if path.is_file() {
                    let content = std::fs::read_to_string(path).map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("{func}() cannot read file '{s}': {e}"),
                            None,
                        )
                    })?;
                    return Ok(split_lines(&content));
                }
            }
            Ok(split_lines(s))
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str or List, got {}", other.type_of()),
            None,
        )),
    }
}

/// Split text into lines, handling \r\n and \n, stripping trailing empty line.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }
    text.lines().map(|l| l.to_string()).collect()
}

// ── grep ─────────────────────────────────────────────────────────

fn builtin_grep(args: Vec<Value>) -> Result<Value> {
    let pattern_str = require_str(&args[1], "grep")?;

    // Parse optional flags (3rd arg)
    let mut invert = false;
    let mut count = false;
    let mut ignore_case = false;
    let mut line_numbers = false;

    if args.len() > 2 {
        match &args[2] {
            Value::Record(m) | Value::Map(m) => {
                if let Some(Value::Bool(true)) = m.get("invert") {
                    invert = true;
                }
                if let Some(Value::Bool(true)) = m.get("count") {
                    count = true;
                }
                if let Some(Value::Bool(true)) = m.get("ignore_case") {
                    ignore_case = true;
                }
                if let Some(Value::Bool(true)) = m.get("line_numbers") {
                    line_numbers = true;
                }
            }
            other => {
                return Err(BioLangError::type_error(
                    format!("grep() flags must be a Record, got {}", other.type_of()),
                    None,
                ));
            }
        }
    }

    let actual_pattern = if ignore_case {
        format!("(?i){pattern_str}")
    } else {
        pattern_str.to_string()
    };
    let re = compile_regex(&actual_pattern, "grep")?;

    let input_lines = resolve_input_lines(&args[0], "grep")?;

    if count {
        let n = input_lines
            .iter()
            .filter(|line| re.is_match(line) != invert)
            .count();
        return Ok(Value::Int(n as i64));
    }

    if line_numbers {
        let matches: Vec<Value> = input_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| re.is_match(line) != invert)
            .map(|(i, line)| {
                Value::List(vec![Value::Int((i + 1) as i64), Value::Str(line.clone())])
            })
            .collect();
        return Ok(Value::List(matches));
    }

    let matches: Vec<Value> = input_lines
        .into_iter()
        .filter(|line| re.is_match(line) != invert)
        .map(Value::Str)
        .collect();
    Ok(Value::List(matches))
}

// ── grep_count ───────────────────────────────────────────────────

fn builtin_grep_count(args: Vec<Value>) -> Result<Value> {
    let pattern_str = require_str(&args[1], "grep_count")?;
    let re = compile_regex(pattern_str, "grep_count")?;
    let input_lines = resolve_input_lines(&args[0], "grep_count")?;
    let n = input_lines.iter().filter(|line| re.is_match(line)).count();
    Ok(Value::Int(n as i64))
}

// ── lines ────────────────────────────────────────────────────────

fn builtin_lines(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "lines")?;
    let result: Vec<Value> = split_lines(text).into_iter().map(Value::Str).collect();
    Ok(Value::List(result))
}

// ── cut ──────────────────────────────────────────────────────────

fn builtin_cut(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "cut")?;
    let delimiter = require_str(&args[1], "cut")?;

    // Parse fields: single Int or List of Ints
    let fields: Vec<usize> = match &args[2] {
        Value::Int(n) => vec![*n as usize],
        Value::List(items) => {
            let mut fs = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Int(n) => fs.push(*n as usize),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("cut() fields must be Int, got {}", other.type_of()),
                            None,
                        ));
                    }
                }
            }
            fs
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "cut() fields must be Int or List[Int], got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };

    let single_field = fields.len() == 1;
    let lines = split_lines(text);

    let result: Vec<Value> = lines
        .iter()
        .map(|line| {
            let parts: Vec<&str> = line.split(delimiter).collect();
            if single_field {
                let idx = fields[0];
                if idx < parts.len() {
                    Value::Str(parts[idx].to_string())
                } else {
                    Value::Str(String::new())
                }
            } else {
                let extracted: Vec<Value> = fields
                    .iter()
                    .map(|&idx| {
                        if idx < parts.len() {
                            Value::Str(parts[idx].to_string())
                        } else {
                            Value::Str(String::new())
                        }
                    })
                    .collect();
                Value::List(extracted)
            }
        })
        .collect();

    Ok(Value::List(result))
}

// ── paste ────────────────────────────────────────────────────────

fn builtin_paste(args: Vec<Value>) -> Result<Value> {
    let list_a = match &args[0] {
        Value::List(l) => l.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("paste() requires List, got {}", other.type_of()),
                None,
            ));
        }
    };
    let list_b = match &args[1] {
        Value::List(l) => l.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("paste() requires List, got {}", other.type_of()),
                None,
            ));
        }
    };
    let sep = if args.len() > 2 {
        require_str(&args[2], "paste")?.to_string()
    } else {
        "\t".to_string()
    };

    let max_len = list_a.len().max(list_b.len());
    let mut result = Vec::with_capacity(max_len);
    for i in 0..max_len {
        let a = list_a.get(i).map(|v| format!("{v}")).unwrap_or_default();
        let b = list_b.get(i).map(|v| format!("{v}")).unwrap_or_default();
        result.push(Value::Str(format!("{a}{sep}{b}")));
    }
    Ok(Value::List(result))
}

// ── uniq_count ───────────────────────────────────────────────────

fn builtin_uniq_count(args: Vec<Value>) -> Result<Value> {
    let items = match &args[0] {
        Value::List(l) => l,
        other => {
            return Err(BioLangError::type_error(
                format!("uniq_count() requires List, got {}", other.type_of()),
                None,
            ));
        }
    };

    // Count occurrences preserving insertion order
    let mut counts: HashMap<String, (Value, i64)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for item in items {
        let key = format!("{item}");
        if let Some(entry) = counts.get_mut(&key) {
            entry.1 += 1;
        } else {
            order.push(key.clone());
            counts.insert(key, (item.clone(), 1));
        }
    }

    // Sort by count descending, then by insertion order
    let mut entries: Vec<(String, Value, i64)> = order
        .into_iter()
        .map(|key| {
            let (val, count) = counts.remove(&key).unwrap();
            (key, val, count)
        })
        .collect();
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let result: Vec<Value> = entries
        .into_iter()
        .map(|(_, val, count)| {
            let mut rec = HashMap::new();
            rec.insert("value".to_string(), val);
            rec.insert("count".to_string(), Value::Int(count));
            Value::Record(rec)
        })
        .collect();

    Ok(Value::List(result))
}

// ── wc ───────────────────────────────────────────────────────────

fn builtin_wc(args: Vec<Value>) -> Result<Value> {
    let text = resolve_input_text(&args[0], "wc")?;

    let line_count = if text.is_empty() {
        0
    } else {
        text.lines().count() as i64
    };
    let word_count = text.split_whitespace().count() as i64;
    let char_count = text.chars().count() as i64;
    let byte_count = text.len() as i64;

    let mut rec = HashMap::new();
    rec.insert("lines".to_string(), Value::Int(line_count));
    rec.insert("words".to_string(), Value::Int(word_count));
    rec.insert("chars".to_string(), Value::Int(char_count));
    rec.insert("bytes".to_string(), Value::Int(byte_count));
    Ok(Value::Record(rec))
}

/// Resolve input to full text content (for wc).
fn resolve_input_text(input: &Value, func: &str) -> Result<String> {
    match input {
        Value::Str(s) => {
            #[cfg(feature = "native")]
            {
                let path = std::path::Path::new(s.as_str());
                if path.is_file() {
                    return std::fs::read_to_string(path).map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("{func}() cannot read file '{s}': {e}"),
                            None,
                        )
                    });
                }
            }
            Ok(s.clone())
        }
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

// ── Native-only builtins ─────────────────────────────────────────

#[cfg(feature = "native")]
fn builtin_tee(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[1], "tee")?;
    let display = format!("{}", args[0]);
    std::fs::write(path, &display).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("tee() cannot write to '{path}': {e}"),
            None,
        )
    })?;
    Ok(args.into_iter().next().unwrap())
}

#[cfg(feature = "native")]
fn builtin_shell(args: Vec<Value>) -> Result<Value> {
    let cmd = require_str(&args[0], "shell")?;
    let stdin_data = if args.len() > 1 {
        Some(format!("{}", args[1]))
    } else {
        None
    };

    let shell_cmd = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut child = std::process::Command::new(shell_cmd.0)
        .arg(shell_cmd.1)
        .arg(cmd)
        .stdin(if stdin_data.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("shell() failed to spawn: {e}"),
                None,
            )
        })?;

    if let Some(data) = stdin_data {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(data.as_bytes());
        }
    }

    let output = child.wait_with_output().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("shell() wait failed: {e}"),
            None,
        )
    })?;

    let mut rec = HashMap::new();
    rec.insert(
        "stdout".to_string(),
        Value::Str(String::from_utf8_lossy(&output.stdout).into_owned()),
    );
    rec.insert(
        "stderr".to_string(),
        Value::Str(String::from_utf8_lossy(&output.stderr).into_owned()),
    );
    rec.insert(
        "exit_code".to_string(),
        Value::Int(output.status.code().unwrap_or(-1) as i64),
    );
    Ok(Value::Record(rec))
}

#[cfg(feature = "native")]
fn builtin_count_lines(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "count_lines")?;
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("count_lines() cannot open '{path}': {e}"),
            None,
        )
    })?;
    let reader = BufReader::new(file);
    let count = reader.lines().count();
    Ok(Value::Int(count as i64))
}

// ── stream_lines (native only) ──────────────────────────────────

#[cfg(feature = "native")]
fn builtin_stream_lines(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "stream_lines")?;
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("stream_lines() cannot open '{path}': {e}"),
            None,
        )
    })?;
    let reader = BufReader::new(file);
    let iter = reader.lines().filter_map(|l| l.ok()).map(Value::Str);
    Ok(Value::Stream(StreamValue::new("lines", Box::new(iter))))
}

// ── stream_concat ───────────────────────────────────────────────

fn builtin_stream_concat(args: Vec<Value>) -> Result<Value> {
    let iter_a = match args[0].clone() {
        Value::Stream(s) => s.collect_all().into_iter(),
        Value::List(l) => l.into_iter(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "stream_concat() requires Stream or List, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let iter_b = match args[1].clone() {
        Value::Stream(s) => s.collect_all().into_iter(),
        Value::List(l) => l.into_iter(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "stream_concat() requires Stream or List, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let combined = iter_a.chain(iter_b);
    Ok(Value::Stream(StreamValue::new(
        "concat",
        Box::new(combined),
    )))
}

