//! Round-trip serialization of BioLang values, for REPL workspace save/restore.
//!
//! This deliberately does not reuse [`crate::json::value_to_json`], which is a
//! lossy *export* format: it flattens `Map` and `Record` to the same object,
//! turns a `Table` into a list of records, and renders anything it does not
//! recognise as its display string. Restoring a workspace through that would
//! silently change types — a `Table` would come back as a `List<Record>` — so
//! this module uses a tagged encoding that round-trips exactly, and refuses
//! values it cannot represent instead of guessing.
//!
//! Encoding: scalars and lists are written bare, so a large numeric matrix
//! stays compact; everything else is an object carrying a `__t` tag. Because
//! tagged payloads live under `v`/`rows`, a user record with its own `__t` key
//! cannot be confused for a tag.
//!
//! Functions and closures are **not** saved. They capture environment scopes
//! and AST bodies, so restoring them would need the whole interpreter state;
//! callers get the list of skipped names and can re-run the defining script.

use std::collections::HashMap;
use std::io::{Read, Write};

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::matrix::Matrix;
use bl_core::value::{BioSequence, Table, Value};
use serde_json::{json, Map as JMap, Value as J};

const TAG: &str = "__t";

fn err(msg: impl Into<String>) -> BioLangError {
    BioLangError::runtime(ErrorKind::IOError, format!("workspace: {}", msg.into()), None)
}

fn tagged(t: &str, pairs: Vec<(&str, J)>) -> J {
    let mut m = JMap::new();
    m.insert(TAG.to_string(), J::String(t.to_string()));
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    J::Object(m)
}

/// Encode a value, or return `None` if it is not representable.
pub fn encode(v: &Value) -> Option<J> {
    Some(match v {
        Value::Nil => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(n) => json!(*n),
        Value::Float(f) => {
            // JSON has no NaN/Infinity, and an integral float must not decode
            // back as an Int, so both go through a tag.
            if f.is_finite() {
                json!(*f)
            } else if f.is_nan() {
                tagged("f", vec![("v", J::String("nan".into()))])
            } else if *f > 0.0 {
                tagged("f", vec![("v", J::String("inf".into()))])
            } else {
                tagged("f", vec![("v", J::String("-inf".into()))])
            }
        }
        Value::Str(s) => J::String(s.to_string()),
        Value::List(items) => J::Array(items.iter().map(encode).collect::<Option<Vec<_>>>()?),
        Value::Tuple(items) => tagged(
            "tuple",
            vec![("v", J::Array(items.iter().map(encode).collect::<Option<Vec<_>>>()?))],
        ),
        Value::Set(items) => tagged(
            "set",
            vec![("v", J::Array(items.iter().map(encode).collect::<Option<Vec<_>>>()?))],
        ),
        Value::Map(m) => tagged("map", vec![("v", encode_obj(m)?)]),
        Value::Record(m) => tagged("rec", vec![("v", encode_obj(m)?)]),
        Value::Table(t) => tagged(
            "table",
            vec![
                ("cols", json!(t.columns)),
                (
                    "rows",
                    J::Array(
                        t.rows
                            .iter()
                            .map(|r| r.iter().map(encode).collect::<Option<Vec<_>>>().map(J::Array))
                            .collect::<Option<Vec<_>>>()?,
                    ),
                ),
            ],
        ),
        Value::Matrix(m) => tagged(
            "matrix",
            vec![
                ("nrow", json!(m.nrow)),
                ("ncol", json!(m.ncol)),
                ("data", json!(m.data)),
                ("row_names", json!(m.row_names)),
                ("col_names", json!(m.col_names)),
            ],
        ),
        Value::DNA(s) => tagged("dna", vec![("v", J::String(s.data.clone()))]),
        Value::RNA(s) => tagged("rna", vec![("v", J::String(s.data.clone()))]),
        Value::Protein(s) => tagged("prot", vec![("v", J::String(s.data.clone()))]),
        // Functions, streams, futures, plugin handles, compiled closures and
        // the remaining domain types are not representable on their own.
        _ => return None,
    })
}

fn encode_obj(m: &HashMap<String, Value>) -> Option<J> {
    let mut out = JMap::new();
    for (k, v) in m {
        out.insert(k.clone(), encode(v)?);
    }
    Some(J::Object(out))
}

/// Decode a value produced by [`encode`].
pub fn decode(j: &J) -> Result<Value> {
    Ok(match j {
        J::Null => Value::Nil,
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().ok_or_else(|| err("unrepresentable number"))?)
            }
        }
        J::String(s) => Value::Str(s.clone()),
        J::Array(a) => Value::List(a.iter().map(decode).collect::<Result<Vec<_>>>()?.into()),
        J::Object(o) => {
            let tag = o
                .get(TAG)
                .and_then(|t| t.as_str())
                .ok_or_else(|| err("object without a type tag"))?;
            match tag {
                "f" => {
                    let s = o.get("v").and_then(|v| v.as_str()).unwrap_or("nan");
                    Value::Float(match s {
                        "inf" => f64::INFINITY,
                        "-inf" => f64::NEG_INFINITY,
                        _ => f64::NAN,
                    })
                }
                "tuple" => Value::Tuple(decode_arr(o.get("v"))?),
                "set" => Value::Set(decode_arr(o.get("v"))?),
                "map" => Value::Map(decode_obj(o.get("v"))?.into()),
                "rec" => Value::Record(decode_obj(o.get("v"))?.into()),
                "table" => {
                    let cols: Vec<String> = o
                        .get("cols")
                        .and_then(|c| c.as_array())
                        .ok_or_else(|| err("table missing cols"))?
                        .iter()
                        .map(|c| c.as_str().unwrap_or("").to_string())
                        .collect();
                    let rows = o
                        .get("rows")
                        .and_then(|r| r.as_array())
                        .ok_or_else(|| err("table missing rows"))?
                        .iter()
                        .map(|r| decode_arr(Some(r)))
                        .collect::<Result<Vec<_>>>()?;
                    Value::Table(Table::new(cols, rows))
                }
                "matrix" => {
                    let nrow = o.get("nrow").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                    let ncol = o.get("ncol").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                    let data: Vec<f64> = o
                        .get("data")
                        .and_then(|d| d.as_array())
                        .ok_or_else(|| err("matrix missing data"))?
                        .iter()
                        .map(|x| x.as_f64().unwrap_or(f64::NAN))
                        .collect();
                    let mut m = Matrix::new(data, nrow, ncol).map_err(err)?;
                    m.row_names = names_of(o.get("row_names"));
                    m.col_names = names_of(o.get("col_names"));
                    Value::Matrix(m)
                }
                "dna" => Value::DNA(BioSequence { data: str_of(o.get("v")) }),
                "rna" => Value::RNA(BioSequence { data: str_of(o.get("v")) }),
                "prot" => Value::Protein(BioSequence { data: str_of(o.get("v")) }),
                other => return Err(err(format!("unknown type tag '{other}'"))),
            }
        }
    })
}

fn decode_arr(j: Option<&J>) -> Result<Vec<Value>> {
    j.and_then(|x| x.as_array())
        .ok_or_else(|| err("expected an array"))?
        .iter()
        .map(decode)
        .collect()
}

fn decode_obj(j: Option<&J>) -> Result<HashMap<String, Value>> {
    j.and_then(|x| x.as_object())
        .ok_or_else(|| err("expected an object"))?
        .iter()
        .map(|(k, v)| decode(v).map(|v| (k.clone(), v)))
        .collect()
}

fn str_of(j: Option<&J>) -> String {
    j.and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn names_of(j: Option<&J>) -> Option<Vec<String>> {
    j?.as_array()
        .map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect())
}

/// What a save call managed to record.
pub struct SaveReport {
    pub saved: Vec<String>,
    /// Names that could not be represented, with the type that blocked them.
    pub skipped: Vec<(String, String)>,
    pub bytes: u64,
}

/// Serialize the named bindings to `path`. A `.gz` suffix gzips the output.
pub fn save<'a>(
    path: &str,
    vars: impl IntoIterator<Item = (&'a str, &'a Value)>,
) -> Result<SaveReport> {
    let mut obj = JMap::new();
    let mut saved = Vec::new();
    let mut skipped = Vec::new();

    for (name, value) in vars {
        match encode(value) {
            Some(j) => {
                obj.insert(name.to_string(), j);
                saved.push(name.to_string());
            }
            None => skipped.push((name.to_string(), format!("{}", value.type_of()))),
        }
    }
    saved.sort();
    skipped.sort();

    let doc = json!({
        "biolang_workspace": 1,
        "vars": J::Object(obj),
    });
    let text = serde_json::to_vec(&doc).map_err(|e| err(format!("encode: {e}")))?;

    let bytes: Vec<u8> = if path.ends_with(".gz") {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&text).map_err(|e| err(format!("gzip: {e}")))?;
        enc.finish().map_err(|e| err(format!("gzip: {e}")))?
    } else {
        text
    };

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    std::fs::write(path, &bytes).map_err(|e| err(format!("cannot write '{path}': {e}")))?;

    Ok(SaveReport { saved, skipped, bytes: bytes.len() as u64 })
}

/// Read bindings back. Returns them sorted by name.
pub fn load(path: &str) -> Result<Vec<(String, Value)>> {
    let raw =
        std::fs::read(path).map_err(|e| err(format!("cannot read '{path}': {e}")))?;
    let text = if path.ends_with(".gz") || raw.starts_with(&[0x1f, 0x8b]) {
        let mut d = flate2::read::GzDecoder::new(&raw[..]);
        let mut s = Vec::new();
        d.read_to_end(&mut s).map_err(|e| err(format!("gunzip: {e}")))?;
        s
    } else {
        raw
    };

    let doc: J = serde_json::from_slice(&text).map_err(|e| err(format!("parse: {e}")))?;
    if doc.get("biolang_workspace").is_none() {
        return Err(err(format!("'{path}' is not a BioLang workspace file")));
    }
    let vars = doc
        .get("vars")
        .and_then(|v| v.as_object())
        .ok_or_else(|| err("workspace has no 'vars' object"))?;

    let mut out: Vec<(String, Value)> = vars
        .iter()
        .map(|(k, v)| decode(v).map(|val| (k.clone(), val)))
        .collect::<Result<_>>()?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
