use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

/// Convert a serde_json::Value to a BioLang Value.
pub fn json_to_value(j: serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Nil
            }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(arr) => {
            Value::List(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let record: HashMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect();
            Value::Record(record)
        }
    }
}

/// Convert a BioLang Value to a serde_json::Value.
pub fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::json!(*n),
        Value::Float(f) => serde_json::json!(*f),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json).collect())
        }
        Value::Map(map) | Value::Record(map) => {
            let obj: serde_json::Map<String, serde_json::Value> =
                map.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect();
            serde_json::Value::Object(obj)
        }
        Value::DNA(seq) => serde_json::Value::String(seq.data.clone()),
        Value::RNA(seq) => serde_json::Value::String(seq.data.clone()),
        Value::Protein(seq) => serde_json::Value::String(seq.data.clone()),
        Value::Table(t) => {
            let rows: Vec<serde_json::Value> = t
                .rows
                .iter()
                .map(|row| {
                    let obj: serde_json::Map<String, serde_json::Value> = t
                        .columns
                        .iter()
                        .zip(row.iter())
                        .map(|(col, val)| (col.clone(), value_to_json(val)))
                        .collect();
                    serde_json::Value::Object(obj)
                })
                .collect();
            serde_json::Value::Array(rows)
        }
        _ => serde_json::Value::String(format!("{v}")),
    }
}

/// Returns the list of (name, arity) for all JSON builtins.
pub fn json_builtin_list() -> Vec<(&'static str, Arity)> {
    let mut builtins = vec![
        ("json_parse", Arity::Exact(1)),
        ("json_stringify", Arity::Exact(1)),
        ("json_pretty", Arity::Exact(1)),
        ("json_keys", Arity::Exact(1)),
    ];
    #[cfg(feature = "native")]
    {
        builtins.push(("read_json", Arity::Exact(1)));
        builtins.push(("write_json", Arity::Exact(2)));
    }
    builtins
}

/// Check if a name is a known JSON builtin.
pub fn is_json_builtin(name: &str) -> bool {
    match name {
        "json_parse" | "json_stringify" | "json_pretty" | "json_keys" => true,
        #[cfg(feature = "native")]
        "read_json" | "write_json" => true,
        _ => false,
    }
}

/// Execute a JSON builtin by name.
pub fn call_json_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "json_parse" => builtin_json_parse(args),
        "json_stringify" => builtin_json_stringify(args),
        "json_pretty" => builtin_json_pretty(args),
        "json_keys" => builtin_json_keys(args),
        #[cfg(feature = "native")]
        "read_json" => builtin_read_json(args),
        #[cfg(feature = "native")]
        "write_json" => builtin_write_json(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown json builtin '{name}'"),
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

fn builtin_json_parse(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "json_parse")?;
    let json: serde_json::Value = serde_json::from_str(s).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("json_parse() invalid JSON: {e}"), None)
    })?;
    Ok(json_to_value(json))
}

fn builtin_json_stringify(args: Vec<Value>) -> Result<Value> {
    let json = value_to_json(&args[0]);
    let s = serde_json::to_string_pretty(&json).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("json_stringify() serialization error: {e}"),
            None,
        )
    })?;
    Ok(Value::Str(s))
}

#[cfg(feature = "native")]
fn builtin_read_json(args: Vec<Value>) -> Result<Value> {
    let path = require_str(&args[0], "read_json")?;
    let content = std::fs::read_to_string(path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("read_json() failed: {e}"), None)
    })?;
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("read_json() invalid JSON: {e}"), None)
    })?;
    Ok(json_to_value(json))
}

#[cfg(feature = "native")]
fn builtin_write_json(args: Vec<Value>) -> Result<Value> {
    let json = value_to_json(&args[0]);
    let path = require_str(&args[1], "write_json")?;
    let content = serde_json::to_string_pretty(&json).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("write_json() serialization error: {e}"),
            None,
        )
    })?;
    std::fs::write(path, content).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("write_json() failed: {e}"), None)
    })?;
    Ok(Value::Nil)
}

fn builtin_json_pretty(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "json_pretty")?;
    let json: serde_json::Value = serde_json::from_str(s).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("json_pretty() invalid JSON: {e}"), None)
    })?;
    let pretty = serde_json::to_string_pretty(&json).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("json_pretty() error: {e}"), None)
    })?;
    Ok(Value::Str(pretty))
}

fn builtin_json_keys(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Record(map) | Value::Map(map) => {
            let keys: Vec<Value> = map.keys().map(|k| Value::Str(k.clone())).collect();
            Ok(Value::List(keys))
        }
        Value::Str(s) => {
            let json: serde_json::Value = serde_json::from_str(s).map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("json_keys() invalid JSON: {e}"),
                    None,
                )
            })?;
            match json {
                serde_json::Value::Object(map) => {
                    let keys: Vec<Value> = map.keys().map(|k| Value::Str(k.clone())).collect();
                    Ok(Value::List(keys))
                }
                _ => Err(BioLangError::type_error(
                    "json_keys() requires a JSON object string",
                    None,
                )),
            }
        }
        other => Err(BioLangError::type_error(
            format!("json_keys() requires Record/Map/Str, got {}", other.type_of()),
            None,
        )),
    }
}

