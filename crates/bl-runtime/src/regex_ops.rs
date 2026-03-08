use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

/// Returns the list of (name, arity) for all regex builtins.
pub fn regex_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("regex_match", Arity::Exact(2)),
        ("regex_find", Arity::Exact(2)),
        ("regex_replace", Arity::Exact(3)),
        ("regex_split", Arity::Exact(2)),
        ("regex_captures", Arity::Exact(2)),
        ("regex_find_all", Arity::Exact(2)),
        ("regex_replace_all", Arity::Exact(3)),
    ]
}

/// Check if a name is a known regex builtin.
pub fn is_regex_builtin(name: &str) -> bool {
    matches!(name, "regex_match" | "regex_find" | "regex_replace" | "regex_split"
        | "regex_captures" | "regex_find_all" | "regex_replace_all")
}

/// Execute a regex builtin by name.
pub fn call_regex_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "regex_match" => builtin_regex_match(args),
        "regex_find" => builtin_regex_find(args),
        "regex_replace" => builtin_regex_replace(args),
        "regex_split" => builtin_regex_split(args),
        "regex_captures" => builtin_regex_captures(args),
        "regex_find_all" => builtin_regex_find(args), // same as regex_find (already returns all)
        "regex_replace_all" => builtin_regex_replace(args), // same as regex_replace (already does replace_all)
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown regex builtin '{name}'"),
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

/// Extract pattern string from either a Str or a Regex value.
fn extract_pattern(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::Str(s) => Ok(s.clone()),
        Value::Regex { pattern, flags } => {
            // Build regex pattern string with flags as inline flags
            let mut p = String::new();
            if !flags.is_empty() {
                p.push_str("(?");
                p.push_str(flags);
                p.push(')');
            }
            p.push_str(pattern);
            Ok(p)
        }
        other => Err(BioLangError::type_error(
            format!("{func}() pattern requires Str or Regex, got {}", other.type_of()),
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

fn builtin_regex_match(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "regex_match")?;
    let pattern = extract_pattern(&args[1], "regex_match")?;
    let re = compile_regex(&pattern, "regex_match")?;
    Ok(Value::Bool(re.is_match(s)))
}

fn builtin_regex_find(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "regex_find")?;
    let pattern = extract_pattern(&args[1], "regex_find")?;
    let re = compile_regex(&pattern, "regex_find")?;
    let matches: Vec<Value> = re
        .find_iter(s)
        .map(|m| Value::Str(m.as_str().to_string()))
        .collect();
    Ok(Value::List(matches))
}

fn builtin_regex_replace(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "regex_replace")?;
    let pattern = extract_pattern(&args[1], "regex_replace")?;
    let replacement = require_str(&args[2], "regex_replace")?;
    let re = compile_regex(&pattern, "regex_replace")?;
    Ok(Value::Str(re.replace_all(s, replacement).into_owned()))
}

fn builtin_regex_captures(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "regex_captures")?;
    let pattern = extract_pattern(&args[1], "regex_captures")?;
    let re = compile_regex(&pattern, "regex_captures")?;
    match re.captures(s) {
        Some(caps) => {
            let groups: Vec<Value> = caps
                .iter()
                .map(|m| match m {
                    Some(m) => Value::Str(m.as_str().to_string()),
                    None => Value::Nil,
                })
                .collect();
            Ok(Value::List(groups))
        }
        None => Ok(Value::Nil),
    }
}

fn builtin_regex_split(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "regex_split")?;
    let pattern = extract_pattern(&args[1], "regex_split")?;
    let re = compile_regex(&pattern, "regex_split")?;
    let parts: Vec<Value> = re.split(s).map(|p| Value::Str(p.to_string())).collect();
    Ok(Value::List(parts))
}

