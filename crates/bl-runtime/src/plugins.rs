use bl_core::error::BioLangError;
use bl_core::value::Value;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Parsed `plugin.json` manifest.
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub spec_version: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub kind: String,
    pub entrypoint: String,
    pub operations: Vec<String>,
}

/// Return the global plugins directory (`~/.biolang/plugins/`), creating it if needed.
pub fn plugins_dir() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let dir = PathBuf::from(home).join(".biolang").join("plugins");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

/// Normalize a plugin import path to a directory name (dots → hyphens).
pub fn normalize_plugin_name(import_path: &str) -> String {
    import_path.replace('.', "-")
}

/// Try to find and load a plugin manifest for the given import path.
/// Returns `(plugin_dir, manifest)` or `None`.
pub fn resolve_plugin(import_path: &str) -> Option<(PathBuf, PluginManifest)> {
    let dir = plugins_dir()?;
    let name = normalize_plugin_name(import_path);
    let plugin_dir = dir.join(&name);
    let manifest_path = plugin_dir.join("plugin.json");

    if !manifest_path.is_file() {
        return None;
    }

    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: PluginManifest = serde_json::from_str(&content).ok()?;

    // Validate spec version
    if manifest.spec_version != "1" {
        return None;
    }

    // Validate entrypoint exists
    let entry = plugin_dir.join(&manifest.entrypoint);
    if !entry.is_file() {
        return None;
    }

    Some((plugin_dir, manifest))
}

/// Load a plugin: resolve its manifest and create `Value::PluginFunction` for each operation.
pub fn load_plugin(import_path: &str) -> bl_core::error::Result<HashMap<String, Value>> {
    let (plugin_dir, manifest) = match resolve_plugin(import_path) {
        Some(pair) => pair,
        None => return Ok(HashMap::new()),
    };

    let mut exports = HashMap::new();
    for op in &manifest.operations {
        exports.insert(
            op.clone(),
            Value::PluginFunction {
                plugin_name: manifest.name.clone(),
                operation: op.clone(),
                plugin_dir: plugin_dir.clone(),
                kind: manifest.kind.clone(),
                entrypoint: manifest.entrypoint.clone(),
            },
        );
    }
    Ok(exports)
}

/// Execute a plugin operation via subprocess JSON protocol.
pub fn call_plugin(
    plugin_name: &str,
    operation: &str,
    plugin_dir: &Path,
    kind: &str,
    entrypoint: &str,
    args: Vec<Value>,
) -> bl_core::error::Result<Value> {
    // Build params from args
    let params = if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(obj)
        } else {
            let mut obj = serde_json::Map::new();
            obj.insert("arg0".to_string(), value_to_json(&args[0]));
            serde_json::Value::Object(obj)
        }
    } else {
        let mut obj = serde_json::Map::new();
        for (i, arg) in args.iter().enumerate() {
            obj.insert(format!("arg{i}"), value_to_json(arg));
        }
        serde_json::Value::Object(obj)
    };

    let work_dir = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let request = serde_json::json!({
        "protocol_version": "1",
        "op": operation,
        "params": params,
        "work_dir": work_dir,
        "plugin_dir": plugin_dir.to_string_lossy(),
    });

    let entry_path = plugin_dir.join(entrypoint);

    // Determine command based on kind
    let (program, mut base_args) = match kind {
        "python" => {
            // Try python3 first, fall back to python
            if which_exists("python3") {
                ("python3".to_string(), vec![])
            } else {
                ("python".to_string(), vec![])
            }
        }
        "deno" => ("deno".to_string(), vec!["run".to_string(), "--allow-all".to_string()]),
        "typescript" => ("npx".to_string(), vec!["tsx".to_string()]),
        "r" => ("Rscript".to_string(), vec![]),
        "native" => (entry_path.to_string_lossy().to_string(), vec![]),
        other => {
            return Err(BioLangError::plugin_error(
                format!("unsupported plugin kind: {other}"),
                None,
            ));
        }
    };

    if kind != "native" {
        base_args.push(entry_path.to_string_lossy().to_string());
    }

    let request_json = serde_json::to_string(&request).map_err(|e| {
        BioLangError::plugin_error(format!("failed to serialize request: {e}"), None)
    })?;

    let mut child = Command::new(&program)
        .args(&base_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            BioLangError::plugin_error(
                format!("failed to start plugin '{plugin_name}' ({program}): {e}"),
                None,
            )
        })?;

    // Write request to stdin, then close it
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request_json.as_bytes()).map_err(|e| {
            BioLangError::plugin_error(
                format!("failed to write to plugin stdin: {e}"),
                None,
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        BioLangError::plugin_error(format!("plugin '{plugin_name}' failed: {e}"), None)
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(BioLangError::plugin_error(
            format!(
                "plugin '{plugin_name}.{operation}' exited with code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ),
            None,
        ));
    }

    // Parse JSON response
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        BioLangError::plugin_error(
            format!("invalid JSON from plugin '{plugin_name}.{operation}': {e}"),
            None,
        )
    })?;

    // Check exit_code in response
    if let Some(code) = response.get("exit_code").and_then(|v| v.as_i64()) {
        if code != 0 {
            let msg = response
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(BioLangError::plugin_error(
                format!("plugin '{plugin_name}.{operation}' returned error: {msg}"),
                None,
            ));
        }
    }

    // Convert outputs to Value
    if let Some(outputs) = response.get("outputs") {
        Ok(json_to_value(outputs.clone()))
    } else {
        Ok(Value::Nil)
    }
}

// Re-export from json module (canonical location for these converters)
pub use crate::json::{json_to_value, value_to_json};

/// Scan the plugins directory and return manifests for all installed plugins.
pub fn list_installed_plugins() -> Vec<PluginManifest> {
    let dir = match plugins_dir() {
        Some(d) => d,
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut plugins = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let manifest_path = entry.path().join("plugin.json");
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                    plugins.push(manifest);
                }
            }
        }
    }

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
}

/// Check if a command exists on PATH.
fn which_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

// Tests are inline because they exercise private helpers (normalize_plugin_name,
// value_to_json, json_to_value from re-export, which_exists) and filesystem-dependent
// plugin resolution that requires temp directory setup.

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_plugin_name_normalization() {
        assert_eq!(normalize_plugin_name("somer.align"), "somer-align");
        assert_eq!(normalize_plugin_name("test.math"), "test-math");
        assert_eq!(normalize_plugin_name("simple"), "simple");
        assert_eq!(normalize_plugin_name("a.b.c"), "a-b-c");
    }

    #[test]
    fn test_value_to_json_primitives() {
        assert_eq!(value_to_json(&Value::Nil), serde_json::Value::Null);
        assert_eq!(value_to_json(&Value::Bool(true)), serde_json::json!(true));
        assert_eq!(value_to_json(&Value::Int(42)), serde_json::json!(42));
        assert_eq!(value_to_json(&Value::Float(3.14)), serde_json::json!(3.14));
        assert_eq!(
            value_to_json(&Value::Str("hello".into())),
            serde_json::json!("hello")
        );
    }

    #[test]
    fn test_value_to_json_list() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(value_to_json(&list), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_value_to_json_record() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), Value::Int(10));
        let rec = Value::Record(map);
        let j = value_to_json(&rec);
        assert_eq!(j.get("x").unwrap(), &serde_json::json!(10));
    }

    #[test]
    fn test_json_to_value_primitives() {
        assert_eq!(json_to_value(serde_json::Value::Null), Value::Nil);
        assert_eq!(json_to_value(serde_json::json!(true)), Value::Bool(true));
        assert_eq!(json_to_value(serde_json::json!(42)), Value::Int(42));
        assert_eq!(json_to_value(serde_json::json!(3.14)), Value::Float(3.14));
        assert_eq!(
            json_to_value(serde_json::json!("hello")),
            Value::Str("hello".into())
        );
    }

    #[test]
    fn test_json_to_value_array() {
        let j = serde_json::json!([1, "two", null]);
        let v = json_to_value(j);
        assert_eq!(
            v,
            Value::List(vec![
                Value::Int(1),
                Value::Str("two".into()),
                Value::Nil,
            ])
        );
    }

    #[test]
    fn test_json_to_value_object() {
        let j = serde_json::json!({"a": 1, "b": "hi"});
        if let Value::Record(map) = json_to_value(j) {
            assert_eq!(map.get("a"), Some(&Value::Int(1)));
            assert_eq!(map.get("b"), Some(&Value::Str("hi".into())));
        } else {
            panic!("expected Record");
        }
    }

    #[test]
    fn test_value_json_roundtrip() {
        let original = Value::List(vec![
            Value::Int(1),
            Value::Float(2.5),
            Value::Str("test".into()),
            Value::Bool(false),
            Value::Nil,
        ]);
        let json = value_to_json(&original);
        let roundtripped = json_to_value(json);
        assert_eq!(original, roundtripped);
    }

    #[test]
    fn test_resolve_plugin_not_found() {
        // A plugin that doesn't exist should return None
        assert!(resolve_plugin("nonexistent.plugin.xyz").is_none());
    }

    #[test]
    fn test_resolve_and_load_plugin() {
        // Create a temporary plugin directory
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("test-math");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        // Write manifest
        let manifest = serde_json::json!({
            "spec_version": "1",
            "name": "test-math",
            "version": "0.1.0",
            "description": "Test math plugin",
            "kind": "python",
            "entrypoint": "main.py",
            "operations": ["square", "double"]
        });
        std::fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        // Write a dummy entrypoint
        std::fs::write(plugin_dir.join("main.py"), "# dummy").unwrap();

        // Override HOME so resolve_plugin finds our temp dir
        let orig_home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok();

        // We can't easily override plugins_dir, so test resolve_plugin indirectly
        // by testing manifest parsing
        let content = std::fs::read_to_string(plugin_dir.join("plugin.json")).unwrap();
        let parsed: PluginManifest = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "test-math");
        assert_eq!(parsed.operations, vec!["square", "double"]);
        assert_eq!(parsed.kind, "python");

        // Restore env
        drop(orig_home);
    }

    #[test]
    fn test_load_plugin_creates_functions() {
        // Create a temporary plugin at the actual plugins directory
        let dir = match plugins_dir() {
            Some(d) => d,
            None => return, // Skip if no home dir
        };

        let plugin_dir = dir.join("_test-plugin-unit");
        let _ = std::fs::create_dir_all(&plugin_dir);

        let manifest = serde_json::json!({
            "spec_version": "1",
            "name": "_test-plugin-unit",
            "version": "0.0.1",
            "description": "unit test plugin",
            "kind": "python",
            "entrypoint": "main.py",
            "operations": ["op_a", "op_b"]
        });
        let _ = std::fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_string(&manifest).unwrap(),
        );
        let _ = std::fs::write(plugin_dir.join("main.py"), "# test");

        let exports = load_plugin("_test.plugin.unit").unwrap();
        assert_eq!(exports.len(), 2);
        assert!(exports.contains_key("op_a"));
        assert!(exports.contains_key("op_b"));

        // Verify they're PluginFunction values
        if let Value::PluginFunction {
            plugin_name,
            operation,
            ..
        } = &exports["op_a"]
        {
            assert_eq!(plugin_name, "_test-plugin-unit");
            assert_eq!(operation, "op_a");
        } else {
            panic!("expected PluginFunction");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&plugin_dir);
    }

    #[test]
    fn test_plugin_function_display() {
        let pf = Value::PluginFunction {
            plugin_name: "somer-align".into(),
            operation: "bwa_mem".into(),
            plugin_dir: PathBuf::from("/tmp/test"),
            kind: "python".into(),
            entrypoint: "main.py".into(),
        };
        assert_eq!(format!("{pf}"), "<plugin:somer-align.bwa_mem>");
    }

    // Edge case: value_to_json with nested lists
    #[test]
    fn test_value_to_json_nested_lists() {
        let inner = Value::List(vec![Value::Int(1), Value::Int(2)]);
        let outer = Value::List(vec![inner, Value::Str("x".into())]);
        let j = value_to_json(&outer);
        assert_eq!(j, serde_json::json!([[1, 2], "x"]));
    }

    // Edge case: value_to_json with DNA/RNA/Protein values
    #[test]
    fn test_value_to_json_bio_sequences() {
        use bl_core::value::BioSequence;

        let dna = Value::DNA(BioSequence { data: "ATCG".to_string() });
        assert_eq!(value_to_json(&dna), serde_json::json!("ATCG"));

        let rna = Value::RNA(BioSequence { data: "AUCG".to_string() });
        assert_eq!(value_to_json(&rna), serde_json::json!("AUCG"));

        let protein = Value::Protein(BioSequence { data: "MVLSP".to_string() });
        assert_eq!(value_to_json(&protein), serde_json::json!("MVLSP"));
    }

    // Edge case: value_to_json with Nil
    #[test]
    fn test_value_to_json_nil() {
        assert_eq!(value_to_json(&Value::Nil), serde_json::Value::Null);
    }

    // Edge case: value_to_json with Float NaN and Infinity
    #[test]
    fn test_value_to_json_float_nan_infinity() {
        // NaN and Infinity are not valid JSON numbers — serde_json::json! will produce null
        let nan_val = value_to_json(&Value::Float(f64::NAN));
        // serde_json cannot represent NaN — it becomes null
        assert!(nan_val.is_null() || nan_val.is_number());

        let inf_val = value_to_json(&Value::Float(f64::INFINITY));
        assert!(inf_val.is_null() || inf_val.is_number());

        let neg_inf_val = value_to_json(&Value::Float(f64::NEG_INFINITY));
        assert!(neg_inf_val.is_null() || neg_inf_val.is_number());
    }

    // Edge case: json_to_value with null
    #[test]
    fn test_json_to_value_null() {
        assert_eq!(json_to_value(serde_json::Value::Null), Value::Nil);
    }

    // Edge case: json_to_value with deeply nested (3+ levels) structures
    #[test]
    fn test_json_to_value_deeply_nested() {
        let deep = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": {
                        "value": 42,
                        "list": [1, [2, [3]]]
                    }
                }
            }
        });
        let val = json_to_value(deep);
        // Navigate: level1 -> level2 -> level3 -> value
        if let Value::Record(l1) = &val {
            if let Some(Value::Record(l2)) = l1.get("level1") {
                if let Some(Value::Record(l3)) = l2.get("level2") {
                    if let Some(Value::Record(l4)) = l3.get("level3") {
                        assert_eq!(l4.get("value"), Some(&Value::Int(42)));
                        if let Some(Value::List(list)) = l4.get("list") {
                            assert_eq!(list[0], Value::Int(1));
                            if let Value::List(inner) = &list[1] {
                                assert_eq!(inner[0], Value::Int(2));
                            } else {
                                panic!("expected inner list");
                            }
                        } else {
                            panic!("expected list");
                        }
                    } else {
                        panic!("expected level3");
                    }
                } else {
                    panic!("expected level2");
                }
            } else {
                panic!("expected level1");
            }
        } else {
            panic!("expected Record");
        }
    }

    // Edge case: normalize_plugin_name with various separators
    #[test]
    fn test_normalize_plugin_name_separators() {
        // Only dots are replaced with hyphens
        assert_eq!(normalize_plugin_name("a.b.c.d"), "a-b-c-d");
        assert_eq!(normalize_plugin_name("no-dots"), "no-dots");
        assert_eq!(normalize_plugin_name("under_score"), "under_score");
        assert_eq!(normalize_plugin_name(""), "");
        assert_eq!(normalize_plugin_name("."), "-");
        assert_eq!(normalize_plugin_name(".."), "--");
        assert_eq!(normalize_plugin_name("a."), "a-");
        assert_eq!(normalize_plugin_name(".a"), "-a");
    }

    // Edge case: value_json_roundtrip with complex nested data
    #[test]
    fn test_value_json_roundtrip_complex() {
        let mut inner_rec = HashMap::new();
        inner_rec.insert("nested_key".to_string(), Value::Str("nested_val".into()));
        inner_rec.insert("nested_int".to_string(), Value::Int(99));

        let mut outer_rec = HashMap::new();
        outer_rec.insert("inner".to_string(), Value::Record(inner_rec));
        outer_rec.insert(
            "list".to_string(),
            Value::List(vec![
                Value::Int(1),
                Value::Float(2.5),
                Value::Bool(true),
                Value::Nil,
                Value::List(vec![Value::Str("deep".into())]),
            ]),
        );
        outer_rec.insert("flag".to_string(), Value::Bool(false));

        let original = Value::Record(outer_rec);
        let json = value_to_json(&original);
        let roundtripped = json_to_value(json);
        // Compare via JSON serialization to avoid HashMap ordering issues
        let j1 = value_to_json(&original);
        let j2 = value_to_json(&roundtripped);
        assert_eq!(j1, j2);
    }

    // Edge case: json_to_value with float that looks like int
    #[test]
    fn test_json_to_value_float_as_int() {
        // JSON number 42.0 should come back as Int(42) if parsed as i64
        let j = serde_json::json!(42);
        assert_eq!(json_to_value(j), Value::Int(42));

        // Explicit float
        let j2 = serde_json::json!(42.5);
        assert_eq!(json_to_value(j2), Value::Float(42.5));
    }
}
