//! Runtime backend selection, provenance ledger, and parity metrics.
//!
//! Bridges the pure [`crate::capabilities`] registry to BioLang values:
//!
//! - `plan_backend(name, [opts])` — pick a backend for a capability, print a
//!   one-line note, append the decision to `.biolang/provenance.json`, and
//!   return it as a Record. This is how a script/package step declares *and*
//!   records "native vs container, and why" (explicit, never silent).
//! - `provenance_log()` — read the ledger back as a List of Records.
//! - `ari(labels_a, labels_b)` — adjusted Rand index, the parity metric for
//!   "do my clusters agree with Scanpy/Seurat" (identity isn't possible for
//!   stochastic clustering, so agreement is measured, not asserted).

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capabilities::{
    cached_env, find_capability, registry, select_backend, SelectionContext,
};

pub fn provenance_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("plan_backend", Arity::Range(1, 2)),
        ("provenance_log", Arity::Exact(0)),
        ("ari", Arity::Exact(2)),
    ]
}

pub fn is_provenance_builtin(name: &str) -> bool {
    matches!(name, "plan_backend" | "provenance_log" | "ari")
}

pub fn call_provenance_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "plan_backend" => builtin_plan_backend(args),
        "provenance_log" => builtin_provenance(),
        "ari" => builtin_ari(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown provenance builtin '{name}'"),
            None,
        )),
    }
}

// ── plan_backend ─────────────────────────────────────────────────────────────

fn ledger_path() -> PathBuf {
    PathBuf::from(".biolang").join("provenance.json")
}

fn builtin_plan_backend(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "plan_backend() requires a capability name (Str), got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let cap = find_capability(&name).ok_or_else(|| {
        let known: Vec<&str> = registry().iter().map(|c| c.name).collect();
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown capability '{name}'. Known: {}", known.join(", ")),
            None,
        )
    })?;

    // Optional opts: { strict: Bool, n_cells: Int }
    let mut ctx = SelectionContext::default();
    if let Some(Value::Record(opts) | Value::Map(opts)) = args.get(1) {
        if let Some(Value::Bool(b)) = opts.get("strict") {
            ctx.strict = *b;
        }
        if let Some(Value::Int(n)) = opts.get("n_cells") {
            ctx.n_cells = Some(*n as usize);
        }
    }

    let decision = select_backend(&cap, cached_env(), &ctx);

    // One-line note to stderr (keeps stdout data-clean).
    let backend_label = decision
        .backend
        .as_ref()
        .map(|b| b.label())
        .unwrap_or_else(|| "unavailable".into());
    eprintln!(
        "\u{2139} {}: {} — {}",
        cap.name, backend_label, decision.reason
    );
    for w in &decision.warnings {
        eprintln!("  \u{26a0} {w}");
    }

    append_ledger(
        cap.name,
        &backend_label,
        &decision.reason,
        &decision.warnings,
    );

    // Return as a Record.
    let mut rec = HashMap::new();
    rec.insert("capability".into(), Value::Str(cap.name.into()));
    rec.insert(
        "backend".into(),
        match &decision.backend {
            Some(b) => Value::Str(b.label()),
            None => Value::Nil,
        },
    );
    rec.insert("ok".into(), Value::Bool(decision.backend.is_some()));
    rec.insert("reason".into(), Value::Str(decision.reason.clone()));
    rec.insert(
        "warnings".into(),
        Value::List(
            decision
                .warnings
                .iter()
                .cloned()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    Ok(Value::Record((rec).into()))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn append_ledger(capability: &str, backend: &str, reason: &str, warnings: &[String]) {
    let path = ledger_path();
    let mut entries: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    entries.push(serde_json::json!({
        "ts": now_millis(),
        "capability": capability,
        "backend": backend,
        "reason": reason,
        "warnings": warnings,
    }));

    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(&path, text);
    }
}

// ── provenance ───────────────────────────────────────────────────────────────

fn builtin_provenance() -> Result<Value> {
    let entries: Vec<serde_json::Value> = std::fs::read_to_string(ledger_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let records: Vec<Value> = entries.iter().map(json_to_value).collect();
    Ok(Value::List((records).into()))
}

/// Minimal serde_json::Value → BioLang Value converter for ledger entries.
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
        serde_json::Value::Array(a) => {
            Value::List(a.iter().map(json_to_value).collect::<Vec<_>>().into())
        }
        serde_json::Value::Object(o) => {
            let mut rec = HashMap::new();
            for (k, v) in o {
                rec.insert(k.clone(), json_to_value(v));
            }
            Value::Record((rec).into())
        }
    }
}

// ── ari (adjusted Rand index) ────────────────────────────────────────────────

fn to_labels(v: &Value, func: &str) -> Result<Vec<String>> {
    match v {
        Value::List(items) => Ok(items
            .iter()
            .map(|x| match x {
                Value::Str(s) => s.clone(),
                Value::Int(n) => n.to_string(),
                other => format!("{other:?}"),
            })
            .collect()),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires a List of labels, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn choose2(n: u64) -> f64 {
    (n as f64) * (n.saturating_sub(1) as f64) / 2.0
}

/// Adjusted Rand index between two label assignments.
pub fn adjusted_rand_index(a: &[String], b: &[String]) -> f64 {
    let n = a.len();
    if n == 0 || n != b.len() {
        return f64::NAN;
    }

    // Contingency table counts.
    let mut contingency: HashMap<(&str, &str), u64> = HashMap::new();
    let mut a_counts: HashMap<&str, u64> = HashMap::new();
    let mut b_counts: HashMap<&str, u64> = HashMap::new();
    for i in 0..n {
        *contingency
            .entry((a[i].as_str(), b[i].as_str()))
            .or_insert(0) += 1;
        *a_counts.entry(a[i].as_str()).or_insert(0) += 1;
        *b_counts.entry(b[i].as_str()).or_insert(0) += 1;
    }

    let sum_index: f64 = contingency.values().map(|&c| choose2(c)).sum();
    let sum_a: f64 = a_counts.values().map(|&c| choose2(c)).sum();
    let sum_b: f64 = b_counts.values().map(|&c| choose2(c)).sum();
    let total = choose2(n as u64);

    let expected = if total > 0.0 {
        sum_a * sum_b / total
    } else {
        0.0
    };
    let max_index = 0.5 * (sum_a + sum_b);
    let denom = max_index - expected;

    if denom.abs() < 1e-12 {
        // Both labelings are trivial (all-same or all-distinct) → define as 1.0.
        1.0
    } else {
        (sum_index - expected) / denom
    }
}

fn builtin_ari(args: Vec<Value>) -> Result<Value> {
    let a = to_labels(&args[0], "ari")?;
    let b = to_labels(&args[1], "ari")?;
    if a.len() != b.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "ari() label lists differ in length ({} vs {})",
                a.len(),
                b.len()
            ),
            None,
        ));
    }
    Ok(Value::Float(adjusted_rand_index(&a, &b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ari_identical_is_one() {
        let a = vec!["0".into(), "0".into(), "1".into(), "1".into()];
        assert!((adjusted_rand_index(&a, &a) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ari_label_permutation_invariant() {
        let a = vec!["0".into(), "0".into(), "1".into(), "1".into()];
        let b = vec!["x".into(), "x".into(), "y".into(), "y".into()];
        assert!((adjusted_rand_index(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ari_one_cluster_vs_singletons_is_zero() {
        let a = vec!["0".into(), "0".into(), "0".into(), "0".into()];
        let b = vec!["0".into(), "1".into(), "2".into(), "3".into()];
        assert!(adjusted_rand_index(&a, &b).abs() < 1e-9);
    }
}
