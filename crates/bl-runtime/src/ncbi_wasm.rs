//! WASM-safe NCBI E-utilities builtins.
//!
//! These use the `__blFetch` hook (synchronous XHR bridge in WASM, ureq in native)
//! to call NCBI E-utilities directly from the browser. NCBI E-utilities support CORS,
//! so cross-origin requests from the browser work without a proxy.
//!
//! Provides: `ncbi_search`, `ncbi_summary`, `ncbi_fetch`, `ncbi_gene`, `ncbi_sequence`.
//!
//! On native builds these are shadowed by the full-featured `apis.rs` builtins which
//! use the typed `bl-apis` clients. This module is only reached when `apis.rs` is absent
//! (i.e., WASM builds without the `native` feature).

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

const EUTILS_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";

// ---------------------------------------------------------------------------
// Three-function registration pattern
// ---------------------------------------------------------------------------

pub fn ncbi_wasm_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("ncbi_search", Arity::Range(2, 3)),
        ("ncbi_summary", Arity::Exact(2)),
        ("ncbi_fetch", Arity::Range(2, 3)),
        ("ncbi_gene", Arity::Range(1, 2)),
        ("ncbi_sequence", Arity::Exact(1)),
    ]
}

pub fn is_ncbi_wasm_builtin(name: &str) -> bool {
    matches!(
        name,
        "ncbi_search" | "ncbi_summary" | "ncbi_fetch" | "ncbi_gene" | "ncbi_sequence"
    )
}

pub fn call_ncbi_wasm_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "ncbi_search" => builtin_ncbi_search(args),
        "ncbi_summary" => builtin_ncbi_summary(args),
        "ncbi_fetch" => builtin_ncbi_fetch(args),
        "ncbi_gene" => builtin_ncbi_gene(args),
        "ncbi_sequence" => builtin_ncbi_sequence(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown NCBI builtin '{name}'"),
            None,
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

/// Fetch a URL using the CSV fetch hook (WASM: `__blFetch.sync`, native: ureq fallback).
fn fetch_url(url: &str) -> Result<String> {
    // Try the WASM/CSV fetch hook first
    if let Some(result) = crate::csv::try_fetch_url(url) {
        match result {
            Ok(text) if !text.starts_with("ERROR:") => return Ok(text),
            Ok(err) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("NCBI fetch error: {err}"),
                    None,
                ))
            }
            Err(e) => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("NCBI fetch error: {e}"),
                    None,
                ))
            }
        }
    }

    // Native fallback: try ureq
    #[cfg(feature = "native")]
    {
        let resp = ureq::get(url).call().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("NCBI HTTP error: {e}"), None)
        })?;
        return resp.into_string().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("NCBI read error: {e}"), None)
        });
    }

    #[cfg(not(feature = "native"))]
    {
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            "No fetch hook available for NCBI API calls".to_string(),
            None,
        ))
    }
}

/// Convert a serde_json::Value to a BioLang Value.
fn json_to_value(j: &serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(arr) => Value::List(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v));
            }
            Value::Record(map)
        }
    }
}

/// URL-encode a query string value.
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
            ' ' => encoded.push('+'),
            _ => {
                for b in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{b:02X}"));
                }
            }
        }
    }
    encoded
}

// ---------------------------------------------------------------------------
// NCBI builtins
// ---------------------------------------------------------------------------

/// `ncbi_search(db, query, [max_results])` — Search an NCBI database.
/// Returns a List of ID strings.
fn builtin_ncbi_search(args: Vec<Value>) -> Result<Value> {
    let db = require_str(&args[0], "ncbi_search")?;
    let term = require_str(&args[1], "ncbi_search")?;
    let max = if args.len() > 2 {
        require_int(&args[2], "ncbi_search")? as usize
    } else {
        20
    };

    let url = format!(
        "{EUTILS_BASE}/esearch.fcgi?db={}&term={}&retmax={max}&retmode=json",
        url_encode(db),
        url_encode(term),
    );

    let text = fetch_url(&url)?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ncbi_search: invalid JSON response: {e}"),
            None,
        )
    })?;

    // Extract idlist from esearchresult
    let ids = json
        .get("esearchresult")
        .and_then(|r| r.get("idlist"))
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| Value::Str(s.to_string())))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(Value::List(ids))
}

/// `ncbi_summary(ids, db)` — Get document summaries. Pipe-friendly: ids first.
/// Returns a List of Records with summary fields.
fn builtin_ncbi_summary(args: Vec<Value>) -> Result<Value> {
    let ids: Vec<String> = match &args[0] {
        Value::Str(s) => vec![s.clone()],
        Value::Int(n) => vec![n.to_string()],
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                Value::Int(n) => Ok(n.to_string()),
                other => Err(BioLangError::type_error(
                    format!(
                        "ncbi_summary() ids must be Str or Int, got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "ncbi_summary() ids must be Str, Int, or List, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let db = require_str(&args[1], "ncbi_summary")?;

    let id_str = ids.join(",");
    let url = format!(
        "{EUTILS_BASE}/esummary.fcgi?db={}&id={}&retmode=json",
        url_encode(db),
        url_encode(&id_str),
    );

    let text = fetch_url(&url)?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ncbi_summary: invalid JSON response: {e}"),
            None,
        )
    })?;

    // Extract document summaries from result
    let result_obj = json.get("result");
    let mut items = Vec::new();

    if let Some(result) = result_obj.and_then(|r| r.as_object()) {
        for id in &ids {
            if let Some(doc) = result.get(id.as_str()) {
                let mut map = HashMap::new();
                map.insert("uid".to_string(), Value::Str(id.clone()));
                if let Some(obj) = doc.as_object() {
                    for (k, v) in obj {
                        if k != "uid" {
                            map.insert(k.clone(), json_to_value(v));
                        }
                    }
                }
                items.push(Value::Record(map));
            }
        }
    }

    Ok(Value::List(items))
}

/// `ncbi_fetch(ids, db, [rettype])` — Fetch records as text. Pipe-friendly: ids first.
/// Returns raw text (FASTA, XML, etc.).
fn builtin_ncbi_fetch(args: Vec<Value>) -> Result<Value> {
    let ids: Vec<String> = match &args[0] {
        Value::Str(s) => vec![s.clone()],
        Value::Int(n) => vec![n.to_string()],
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                Value::Int(n) => Ok(n.to_string()),
                other => Err(BioLangError::type_error(
                    format!(
                        "ncbi_fetch() ids must be Str or Int, got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "ncbi_fetch() ids must be Str, Int, or List, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let db = require_str(&args[1], "ncbi_fetch")?;
    let rettype = if args.len() > 2 {
        require_str(&args[2], "ncbi_fetch")?
    } else {
        match db {
            "nucleotide" | "nuccore" | "protein" => "fasta",
            "gene" => "xml",
            "pubmed" => "abstract",
            _ => "xml",
        }
    };

    let id_str = ids.join(",");
    let url = format!(
        "{EUTILS_BASE}/efetch.fcgi?db={}&id={}&rettype={}&retmode=text",
        url_encode(db),
        url_encode(&id_str),
        url_encode(rettype),
    );

    let text = fetch_url(&url)?;
    Ok(Value::Str(text))
}

/// `ncbi_gene(query, [max])` — Convenience: search gene db, return summary for top result.
/// Returns a Record with gene info if a single result, or a List of IDs.
fn builtin_ncbi_gene(args: Vec<Value>) -> Result<Value> {
    let term = require_str(&args[0], "ncbi_gene")?;
    let max = if args.len() > 1 {
        require_int(&args[1], "ncbi_gene")? as usize
    } else {
        10
    };

    // Step 1: Search the gene database
    let search_url = format!(
        "{EUTILS_BASE}/esearch.fcgi?db=gene&term={}&retmax={max}&retmode=json",
        url_encode(term),
    );

    let search_text = fetch_url(&search_url)?;
    let search_json: serde_json::Value = serde_json::from_str(&search_text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("ncbi_gene: invalid JSON from esearch: {e}"),
            None,
        )
    })?;

    let ids: Vec<String> = search_json
        .get("esearchresult")
        .and_then(|r| r.get("idlist"))
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if ids.is_empty() {
        return Ok(Value::List(Vec::new()));
    }

    // Step 2: If single result, get summary for richer output
    if ids.len() == 1 {
        let id = &ids[0];
        let summary_url = format!(
            "{EUTILS_BASE}/esummary.fcgi?db=gene&id={}&retmode=json",
            url_encode(id),
        );

        if let Ok(summary_text) = fetch_url(&summary_url) {
            if let Ok(summary_json) = serde_json::from_str::<serde_json::Value>(&summary_text) {
                if let Some(doc) = summary_json
                    .get("result")
                    .and_then(|r| r.get(id.as_str()))
                    .and_then(|d| d.as_object())
                {
                    let mut map = HashMap::new();
                    map.insert("id".to_string(), Value::Str(id.clone()));
                    // Extract common gene fields
                    if let Some(name) = doc.get("name").and_then(|v| v.as_str()) {
                        map.insert("symbol".to_string(), Value::Str(name.to_string()));
                    }
                    if let Some(desc) = doc.get("description").and_then(|v| v.as_str()) {
                        map.insert("name".to_string(), Value::Str(desc.to_string()));
                        map.insert("description".to_string(), Value::Str(desc.to_string()));
                    }
                    if let Some(org) = doc
                        .get("organism")
                        .and_then(|o| o.get("scientificname"))
                        .and_then(|v| v.as_str())
                    {
                        map.insert("organism".to_string(), Value::Str(org.to_string()));
                    }
                    if let Some(chrom) = doc.get("chromosome").and_then(|v| v.as_str()) {
                        map.insert("chromosome".to_string(), Value::Str(chrom.to_string()));
                    }
                    if let Some(loc) = doc.get("maplocation").and_then(|v| v.as_str()) {
                        map.insert("location".to_string(), Value::Str(loc.to_string()));
                    }
                    if let Some(summary) = doc.get("summary").and_then(|v| v.as_str()) {
                        map.insert("summary".to_string(), Value::Str(summary.to_string()));
                    }
                    return Ok(Value::Record(map));
                }
            }
        }
    }

    // Return list of IDs
    Ok(Value::List(ids.into_iter().map(Value::Str).collect()))
}

/// `ncbi_sequence(accession)` — Fetch a FASTA sequence by accession.
/// Returns raw FASTA text as Str.
fn builtin_ncbi_sequence(args: Vec<Value>) -> Result<Value> {
    let id = require_str(&args[0], "ncbi_sequence")?;

    let url = format!(
        "{EUTILS_BASE}/efetch.fcgi?db=nuccore&id={}&rettype=fasta&retmode=text",
        url_encode(id),
    );

    let text = fetch_url(&url)?;
    Ok(Value::Str(text))
}
