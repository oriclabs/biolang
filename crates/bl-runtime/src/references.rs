//! The `ref` builtin, over the shared reference registry.
//!
//! Storage and parsing live in `bl-refs` so the workbench can manage the same
//! registry without inheriting the runtime's native dependencies.

use std::collections::HashMap;

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use bl_refs::{build_names, load, registry_path};

pub fn reference_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![("ref", Arity::Range(1, 2)), ("ref_builds", Arity::Exact(0))]
}

pub fn is_reference_builtin(name: &str) -> bool {
    matches!(name, "ref" | "ref_builds")
}

fn require_str(value: &Value, context: &str) -> Result<String> {
    match value {
        Value::Str(text) => Ok(text.clone()),
        other => Err(BioLangError::new(
            ErrorKind::TypeError,
            format!("{context} expects a string, got {}", other.type_of()),
            None,
        )),
    }
}

pub fn call_reference_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    let registry = load();
    match name {
        "ref_builds" => Ok(Value::List(std::sync::Arc::new(
            build_names(&registry).into_iter().map(Value::Str).collect(),
        ))),
        "ref" => {
            let build = require_str(&args[0], "ref")?;
            let Some(assets) = registry.get(&build) else {
                let configured = build_names(&registry);
                // Naming what *is* configured turns a dead end into the next
                // step, which is the whole reason this exists.
                let hint = if configured.is_empty() {
                    format!(
                        "no reference builds are configured. Add one to {}",
                        registry_path().display()
                    )
                } else {
                    format!("configured builds: {}", configured.join(", "))
                };
                return Err(BioLangError::new(
                    ErrorKind::NameError,
                    format!("Unknown reference build '{build}' — {hint}"),
                    None,
                ));
            };

            if let Some(asset) = args.get(1) {
                let asset = require_str(asset, "ref")?;
                return match assets.get(&asset) {
                    Some(path) => Ok(Value::Str(path.clone())),
                    None => {
                        let mut available: Vec<&String> = assets.keys().collect();
                        available.sort();
                        Err(BioLangError::new(
                            ErrorKind::NameError,
                            format!(
                                "Reference build '{build}' has no '{asset}' — it defines: {}",
                                available
                                    .iter()
                                    .map(|key| key.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            None,
                        ))
                    }
                };
            }

            let record: HashMap<String, Value> = assets
                .iter()
                .map(|(key, path)| (key.clone(), Value::Str(path.clone())))
                .collect();
            Ok(Value::Record(std::sync::Arc::new(record)))
        }
        _ => Err(BioLangError::new(
            ErrorKind::NameError,
            format!("Unknown reference builtin: {name}"),
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_builtins_are_registered() {
        assert!(is_reference_builtin("ref"));
        assert!(is_reference_builtin("ref_builds"));
        assert!(!is_reference_builtin("reference"));
        assert_eq!(reference_builtin_list().len(), 2);
    }

    #[test]
    fn a_missing_build_names_what_is_configured() {
        // Storage and parsing are covered by bl-refs; this covers the message,
        // which is the part that has to point at the next step.
        let error = call_reference_builtin("ref", vec![Value::Str("NoSuchBuild".into())])
            .expect_err("unknown build");
        assert!(error.message.contains("NoSuchBuild"), "{}", error.message);
        assert!(
            error.message.contains("references.toml")
                || error.message.contains("configured builds"),
            "{}",
            error.message
        );
    }

    #[test]
    fn ref_rejects_a_non_string_build() {
        let error = call_reference_builtin("ref", vec![Value::Int(1)]).expect_err("type error");
        assert!(
            error.message.contains("expects a string"),
            "{}",
            error.message
        );
    }
}
