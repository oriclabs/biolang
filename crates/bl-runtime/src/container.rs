use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use std::collections::HashMap;
use std::process::Command;

/// Returns the list of (name, arity) for all container builtins.
pub fn container_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("container_available", Arity::Exact(0)),
        ("container_run", Arity::Range(2, 3)),
        ("container_pull", Arity::Exact(1)),
        ("tool", Arity::Range(2, 3)),
        ("tool_search", Arity::Range(1, 2)),
        ("tool_popular", Arity::Range(0, 1)),
        ("tool_info", Arity::Exact(1)),
        ("tool_pull", Arity::Range(1, 2)),
        ("tool_list", Arity::Exact(0)),
        ("tool_available", Arity::Exact(0)),
    ]
}

/// Check if a name is a known container builtin.
pub fn is_container_builtin(name: &str) -> bool {
    matches!(
        name,
        "container_available"
            | "container_run"
            | "container_pull"
            | "tool"
            | "tool_search"
            | "tool_popular"
            | "tool_info"
            | "tool_pull"
            | "tool_list"
            | "tool_available"
    )
}

/// Execute a container builtin by name.
pub fn call_container_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "container_available" | "tool_available" => builtin_container_available(),
        "container_run" => builtin_container_run(args),
        "container_pull" => builtin_container_pull(args),
        "tool" => builtin_tool(args),
        "tool_search" => builtin_tool_search(args),
        "tool_popular" => builtin_tool_popular(args),
        "tool_info" => builtin_tool_info(args),
        "tool_pull" => builtin_tool_pull(args),
        "tool_list" => builtin_tool_list(),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown container builtin '{name}'"),
            None,
        )),
    }
}

// ── Runtime Detection ───────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Runtime {
    Docker,
    Podman,
    Singularity,
    Apptainer,
}

impl Runtime {
    fn cmd(&self) -> &'static str {
        match self {
            Runtime::Docker => "docker",
            Runtime::Podman => "podman",
            Runtime::Singularity => "singularity",
            Runtime::Apptainer => "apptainer",
        }
    }

    fn is_oci(&self) -> bool {
        matches!(self, Runtime::Docker | Runtime::Podman)
    }
}

fn detect_runtime() -> Option<(Runtime, String)> {
    let candidates = [
        Runtime::Docker,
        Runtime::Podman,
        Runtime::Singularity,
        Runtime::Apptainer,
    ];
    for rt in candidates {
        if let Ok(output) = Command::new(rt.cmd()).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Some((rt, version));
            }
        }
    }
    None
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

fn run_output_to_record(output: std::process::Output) -> Value {
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
    Value::Record(rec)
}

/// Parse optional opts Record for mount, workdir, env.
fn parse_opts(args: &[Value]) -> Result<(Option<String>, Option<String>, Vec<(String, String)>)> {
    if args.len() <= 2 {
        return Ok((None, None, vec![]));
    }
    match &args[2] {
        Value::Record(m) | Value::Map(m) => {
            let mount = m.get("mount").and_then(|v| {
                if let Value::Str(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            });
            let workdir = m.get("workdir").and_then(|v| {
                if let Value::Str(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            });
            let mut env_vars = vec![];
            if let Some(Value::Record(env_map)) | Some(Value::Map(env_map)) = m.get("env") {
                for (k, v) in env_map {
                    env_vars.push((k.clone(), format!("{v}")));
                }
            }
            Ok((mount, workdir, env_vars))
        }
        other => Err(BioLangError::type_error(
            format!("container opts must be a Record, got {}", other.type_of()),
            None,
        )),
    }
}

// ── container_available / tool_available ─────────────────────────

fn builtin_container_available() -> Result<Value> {
    let mut rec = HashMap::new();
    match detect_runtime() {
        Some((rt, version)) => {
            rec.insert("runtime".to_string(), Value::Str(rt.cmd().to_string()));
            rec.insert("version".to_string(), Value::Str(version));
        }
        None => {
            rec.insert("runtime".to_string(), Value::Nil);
        }
    }
    // Report image cache directory
    match resolve_image_dir() {
        Some(dir) => rec.insert("image_dir".to_string(), Value::Str(dir)),
        None => rec.insert("image_dir".to_string(), Value::Nil),
    };
    Ok(Value::Record(rec))
}

// ── container_run ────────────────────────────────────────────────

fn builtin_container_run(args: Vec<Value>) -> Result<Value> {
    let image = require_str(&args[0], "container_run")?;
    let cmd = require_str(&args[1], "container_run")?;
    let (mount, workdir, env_vars) = parse_opts(&args)?;

    let (rt, _) = detect_runtime().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            "container_run(): no container runtime found (install docker, podman, singularity, or apptainer)",
            None,
        )
    })?;

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".to_string());
    let mount_src = mount.as_deref().unwrap_or(&cwd);
    let container_workdir = workdir.as_deref().unwrap_or("/data");

    let output = if rt.is_oci() {
        // docker/podman run --rm -v SRC:/data -w /data IMAGE sh -c CMD
        let mut c = Command::new(rt.cmd());
        c.arg("run").arg("--rm");
        c.arg("-v").arg(format!("{mount_src}:{container_workdir}"));
        c.arg("-w").arg(container_workdir);
        for (k, v) in &env_vars {
            c.arg("-e").arg(format!("{k}={v}"));
        }
        c.arg(image).arg("sh").arg("-c").arg(cmd);
        c.output()
    } else {
        // singularity/apptainer exec --bind SRC:/data --pwd /data IMAGE sh -c CMD
        let mut c = Command::new(rt.cmd());
        c.arg("exec");
        c.arg("--bind")
            .arg(format!("{mount_src}:{container_workdir}"));
        c.arg("--pwd").arg(container_workdir);
        for (k, v) in &env_vars {
            c.arg("--env").arg(format!("{k}={v}"));
        }
        c.arg(image).arg("sh").arg("-c").arg(cmd);
        c.output()
    };

    let output = output.map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("container_run() failed to spawn {}: {e}", rt.cmd()),
            None,
        )
    })?;

    Ok(run_output_to_record(output))
}

// ── container_pull ───────────────────────────────────────────────

fn builtin_container_pull(args: Vec<Value>) -> Result<Value> {
    let image = require_str(&args[0], "container_pull")?;

    let (rt, _) = detect_runtime().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            "container_pull(): no container runtime found",
            None,
        )
    })?;

    eprintln!("  pulling {image} via {} ...", rt.cmd());

    // Use spawn + inherit stderr so the user sees pull progress in real-time
    let mut cmd = if rt.is_oci() {
        let mut c = Command::new(rt.cmd());
        c.arg("pull").arg(image);
        c
    } else {
        let mut c = Command::new(rt.cmd());
        c.arg("pull");
        if let Some(dir) = resolve_image_dir() {
            let _ = std::fs::create_dir_all(&dir);
            c.arg("--dir").arg(&dir);
        }
        c.arg(image);
        c
    };

    let status = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("container_pull() failed to start: {e}"),
                None,
            )
        })?;

    if !status.success() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("container_pull() exited with status {}", status),
            None,
        ));
    }

    let mut rec = HashMap::new();
    rec.insert("image".to_string(), Value::Str(image.to_string()));
    if rt.is_oci() {
        rec.insert(
            "storage".to_string(),
            Value::Str(format!("{} daemon storage", rt.cmd())),
        );
    } else if let Some(dir) = resolve_image_dir() {
        rec.insert("storage".to_string(), Value::Str(dir));
    }
    rec.insert(
        "hint".to_string(),
        Value::Str("set BIOLANG_IMAGE_DIR to change image storage location".to_string()),
    );
    Ok(Value::Record(rec))
}

// ── tool ─────────────────────────────────────────────────────────

fn resolve_biocontainers_image(name: &str) -> Result<String> {
    // Try BioContainers API to find latest tag
    let url = format!(
        "https://api.biocontainers.pro/ga4gh/trs/v2/tools/{}",
        urlencoded(name)
    );
    match crate::http::shared_agent().get(&url).call() {
        Ok(resp) => {
            let text = resp.into_string().map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("tool(): failed to read BioContainers response: {e}"),
                    None,
                )
            })?;
            let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("tool(): failed to parse BioContainers response: {e}"),
                    None,
                )
            })?;
            // Look for the first docker image version
            if let Some(versions) = body.get("versions").and_then(serde_json::Value::as_array) {
                for ver in versions {
                    if let Some(images) = ver.get("images").and_then(serde_json::Value::as_array) {
                        for img in images {
                            if let Some(image_name) =
                                img.get("image_name").and_then(serde_json::Value::as_str)
                            {
                                if image_name.contains("quay.io")
                                    || image_name.contains("docker")
                                {
                                    return Ok(image_name.to_string());
                                }
                            }
                        }
                    }
                    // Fallback: construct from version name
                    if let Some(ver_name) = ver.get("name").and_then(serde_json::Value::as_str) {
                        return Ok(format!("quay.io/biocontainers/{name}:{ver_name}"));
                    }
                }
            }
            // Fallback: use latest
            Ok(format!("quay.io/biocontainers/{name}:latest"))
        }
        Err(_) => {
            // Fallback: assume quay.io/biocontainers/<name>:latest
            Ok(format!("quay.io/biocontainers/{name}:latest"))
        }
    }
}

fn builtin_tool(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "tool")?;
    let cmd = require_str(&args[1], "tool")?;

    let image = resolve_biocontainers_image(name)?;

    // Build a new args vec with the resolved image
    let mut run_args = vec![Value::Str(image), Value::Str(cmd.to_string())];
    if args.len() > 2 {
        run_args.push(args[2].clone());
    }

    builtin_container_run(run_args)
}

// ── tool_search ──────────────────────────────────────────────────

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
}

/// Shared: fetch tools from BioContainers API and parse results.
fn fetch_biocontainers_tools(func: &str, url: &str) -> Result<Vec<serde_json::Value>> {
    let resp = crate::http::shared_agent().get(url).call().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() HTTP request failed: {e}"),
            None,
        )
    })?;

    let text = resp.into_string().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() failed to read response: {e}"),
            None,
        )
    })?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{func}() failed to parse response: {e}"),
            None,
        )
    })?;

    Ok(body.as_array().cloned().unwrap_or_default())
}

/// Convert a JSON tool object to a BioLang Record with standard fields.
fn tool_json_to_record(tool: &serde_json::Value) -> Value {
    let mut rec = HashMap::new();
    let str_field = |key: &str| -> Value {
        Value::Str(
            tool.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
        )
    };
    rec.insert("name".to_string(), str_field("name"));
    rec.insert("description".to_string(), str_field("description"));
    rec.insert("url".to_string(), str_field("url"));
    rec.insert("organization".to_string(), str_field("organization"));
    rec.insert("license".to_string(), str_field("license"));

    // pulls (download count)
    let pulls = tool
        .get("pulls")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    rec.insert("pulls".to_string(), Value::Int(pulls));

    // version count
    let versions_count = tool
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .map(|v| v.len())
        .unwrap_or(0);
    rec.insert("versions".to_string(), Value::Int(versions_count as i64));

    Value::Record(rec)
}

fn builtin_tool_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "tool_search")?;

    // Parse optional opts Record
    let mut limit = 10;
    let mut offset = 0;
    let mut sort_field = String::new();
    let mut sort_order = String::new();
    let mut all_fields = false;
    let mut extra_params: Vec<(String, String)> = Vec::new();

    if args.len() > 1 {
        match &args[1] {
            Value::Record(m) | Value::Map(m) => {
                if let Some(Value::Int(n)) = m.get("limit") {
                    limit = *n;
                }
                if let Some(Value::Int(n)) = m.get("offset") {
                    offset = *n;
                }
                if let Some(Value::Str(s)) = m.get("sort") {
                    sort_field = s.clone();
                }
                if let Some(Value::Str(s)) = m.get("order") {
                    sort_order = s.clone();
                }
                if let Some(Value::Bool(true)) = m.get("all_fields") {
                    all_fields = true;
                }
                // Pass-through filters
                for key in &["toolclass", "license", "organization", "registry", "author"] {
                    if let Some(Value::Str(s)) = m.get(*key) {
                        extra_params.push((key.to_string(), s.clone()));
                    }
                }
            }
            other => {
                return Err(BioLangError::type_error(
                    format!("tool_search() opts must be a Record, got {}", other.type_of()),
                    None,
                ))
            }
        }
    }

    // Build URL
    let search_param = if all_fields {
        format!("all_fields_search={}", urlencoded(query))
    } else {
        format!("name={}", urlencoded(query))
    };
    let mut url = format!(
        "https://api.biocontainers.pro/ga4gh/trs/v2/tools?{search_param}&limit={limit}&offset={offset}"
    );
    if !sort_field.is_empty() {
        url.push_str(&format!("&sort_field={}", urlencoded(&sort_field)));
    }
    if !sort_order.is_empty() {
        url.push_str(&format!("&sort_order={}", urlencoded(&sort_order)));
    }
    for (k, v) in &extra_params {
        url.push_str(&format!("&{k}={}", urlencoded(v)));
    }

    let tools = fetch_biocontainers_tools("tool_search", &url)?;
    let results: Vec<Value> = tools.iter().map(tool_json_to_record).collect();
    Ok(Value::List(results))
}

// ── tool_popular ─────────────────────────────────────────────────

fn builtin_tool_popular(args: Vec<Value>) -> Result<Value> {
    let limit = if !args.is_empty() {
        match &args[0] {
            Value::Int(n) => *n,
            other => {
                return Err(BioLangError::type_error(
                    format!("tool_popular() limit must be Int, got {}", other.type_of()),
                    None,
                ))
            }
        }
    } else {
        20
    };

    let url = format!(
        "https://api.biocontainers.pro/ga4gh/trs/v2/tools?sort_field=pulls&sort_order=desc&limit={limit}"
    );

    let tools = fetch_biocontainers_tools("tool_popular", &url)?;
    let results: Vec<Value> = tools.iter().map(tool_json_to_record).collect();
    Ok(Value::List(results))
}

// ── tool_info ────────────────────────────────────────────────────

fn builtin_tool_info(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "tool_info")?;
    let url = format!(
        "https://api.biocontainers.pro/ga4gh/trs/v2/tools/{}",
        urlencoded(name)
    );

    let resp = crate::http::shared_agent().get(&url).call().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("tool_info() HTTP request failed: {e}"),
            None,
        )
    })?;

    let text = resp.into_string().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("tool_info() failed to read response: {e}"),
            None,
        )
    })?;
    let tool: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("tool_info() failed to parse response: {e}"),
            None,
        )
    })?;

    // Build detailed record including version list
    let mut rec = HashMap::new();
    let str_field = |key: &str| -> Value {
        Value::Str(
            tool.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
        )
    };
    rec.insert("name".to_string(), str_field("name"));
    rec.insert("description".to_string(), str_field("description"));
    rec.insert("organization".to_string(), str_field("organization"));
    rec.insert("license".to_string(), str_field("license"));
    rec.insert("url".to_string(), str_field("url"));
    rec.insert("tool_url".to_string(), str_field("tool_url"));

    let pulls = tool
        .get("pulls")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    rec.insert("pulls".to_string(), Value::Int(pulls));

    // Extract version names as a list
    let versions: Vec<Value> = tool
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let ver_name = v.get("name").and_then(serde_json::Value::as_str)?;
                    // Collect image references for this version
                    let images: Vec<Value> = v
                        .get("images")
                        .and_then(serde_json::Value::as_array)
                        .map(|imgs| {
                            imgs.iter()
                                .filter_map(|img| {
                                    let mut irec = HashMap::new();
                                    if let Some(iname) =
                                        img.get("image_name").and_then(serde_json::Value::as_str)
                                    {
                                        irec.insert(
                                            "image".to_string(),
                                            Value::Str(iname.to_string()),
                                        );
                                    }
                                    if let Some(size) =
                                        img.get("size").and_then(serde_json::Value::as_i64)
                                    {
                                        irec.insert("size".to_string(), Value::Int(size));
                                    }
                                    if let Some(updated) = img
                                        .get("updated")
                                        .and_then(serde_json::Value::as_str)
                                    {
                                        irec.insert(
                                            "updated".to_string(),
                                            Value::Str(updated.to_string()),
                                        );
                                    }
                                    if irec.is_empty() {
                                        None
                                    } else {
                                        Some(Value::Record(irec))
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut vrec = HashMap::new();
                    vrec.insert("name".to_string(), Value::Str(ver_name.to_string()));
                    vrec.insert("images".to_string(), Value::List(images));
                    Some(Value::Record(vrec))
                })
                .collect()
        })
        .unwrap_or_default();

    rec.insert("versions".to_string(), Value::List(versions));

    Ok(Value::Record(rec))
}

// ── tool_pull ────────────────────────────────────────────────────

fn builtin_tool_pull(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "tool_pull")?;
    let version = if args.len() > 1 {
        Some(require_str(&args[1], "tool_pull")?.to_string())
    } else {
        None
    };

    let image = match version {
        Some(ver) => format!("quay.io/biocontainers/{name}:{ver}"),
        None => resolve_biocontainers_image(name)?,
    };

    builtin_container_pull(vec![Value::Str(image)])
}

// ── tool_list ────────────────────────────────────────────────────

fn builtin_tool_list() -> Result<Value> {
    let (rt, _) = detect_runtime().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            "tool_list(): no container runtime found",
            None,
        )
    })?;

    let output = if rt.is_oci() {
        Command::new(rt.cmd())
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()
    } else {
        // singularity/apptainer: list cached SIF files
        if let Some(dir) = resolve_image_dir() {
            return list_sif_files(&dir);
        }
        return Ok(Value::List(vec![]));
    };

    let output = output.map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("tool_list() failed: {e}"),
            None,
        )
    })?;

    if !output.status.success() {
        return Ok(Value::List(vec![]));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let images: Vec<Value> = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("<none>"))
        .map(|l| Value::Str(l.trim().to_string()))
        .collect();

    Ok(Value::List(images))
}

/// Resolve the image cache directory. Priority:
/// 1. `BIOLANG_IMAGE_DIR` env var
/// 2. `SINGULARITY_CACHEDIR` / `APPTAINER_CACHEDIR` (for those runtimes)
/// 3. `~/.biolang/images/`
fn resolve_image_dir() -> Option<String> {
    if let Ok(dir) = std::env::var("BIOLANG_IMAGE_DIR") {
        return Some(dir);
    }
    if let Ok(dir) = std::env::var("SINGULARITY_CACHEDIR") {
        return Some(dir);
    }
    if let Ok(dir) = std::env::var("APPTAINER_CACHEDIR") {
        return Some(dir);
    }
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| format!("{h}/.biolang/images"))
}

fn list_sif_files(dir: &str) -> Result<Value> {
    let path = std::path::Path::new(dir);
    if !path.is_dir() {
        return Ok(Value::List(vec![]));
    }
    let mut images = vec![];
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "sif").unwrap_or(false) {
                if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
                    images.push(Value::Str(name.to_string()));
                }
            }
        }
    }
    Ok(Value::List(images))
}

// ── Tests ────────────────────────────────────────────────────────
// Tests are inline because they exercise private helpers (detect_runtime, parse_opts,
// run_output_to_record, resolve_image_dir, urlencoded, tool_json_to_record) and
// network-dependent builtins that are #[ignore]'d.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_available_returns_record() {
        let result = call_container_builtin("container_available", vec![]).unwrap();
        if let Value::Record(rec) = &result {
            assert!(rec.contains_key("runtime"));
        } else {
            panic!("expected Record");
        }
    }

    #[test]
    fn test_tool_available_is_alias() {
        let a = call_container_builtin("container_available", vec![]).unwrap();
        let b = call_container_builtin("tool_available", vec![]).unwrap();
        // Both should have same structure
        if let (Value::Record(ra), Value::Record(rb)) = (&a, &b) {
            assert_eq!(ra.contains_key("runtime"), rb.contains_key("runtime"));
        }
    }

    #[test]
    fn test_container_run_simple() {
        // Skip if no runtime available
        if detect_runtime().is_none() {
            eprintln!("skipping container_run test: no runtime");
            return;
        }
        let result = call_container_builtin(
            "container_run",
            vec![
                Value::Str("alpine:latest".into()),
                Value::Str("echo hello".into()),
            ],
        )
        .unwrap();
        if let Value::Record(rec) = &result {
            assert!(rec.contains_key("stdout"));
            assert!(rec.contains_key("exit_code"));
        } else {
            panic!("expected Record");
        }
    }

    #[test]
    #[ignore] // requires network
    fn test_tool_search_samtools() {
        let result =
            call_container_builtin("tool_search", vec![Value::Str("samtools".into())]).unwrap();
        if let Value::List(items) = &result {
            assert!(!items.is_empty());
            if let Value::Record(rec) = &items[0] {
                assert!(rec.contains_key("name"));
                assert!(rec.contains_key("description"));
                assert!(rec.contains_key("pulls"));
                assert!(rec.contains_key("license"));
                assert!(rec.contains_key("versions"));
            }
        } else {
            panic!("expected List");
        }
    }

    #[test]
    #[ignore] // requires network
    fn test_tool_search_with_opts() {
        let mut opts = HashMap::new();
        opts.insert("sort".to_string(), Value::Str("pulls".into()));
        opts.insert("order".to_string(), Value::Str("desc".into()));
        opts.insert("limit".to_string(), Value::Int(5));

        let result = call_container_builtin(
            "tool_search",
            vec![Value::Str("sam".into()), Value::Record(opts)],
        )
        .unwrap();
        if let Value::List(items) = &result {
            assert!(items.len() <= 5);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    #[ignore] // requires network
    fn test_tool_popular() {
        let result =
            call_container_builtin("tool_popular", vec![Value::Int(3)]).unwrap();
        if let Value::List(items) = &result {
            assert_eq!(items.len(), 3);
            // First should have most pulls
            if let Value::Record(rec) = &items[0] {
                if let Some(Value::Int(pulls)) = rec.get("pulls") {
                    assert!(*pulls > 0);
                }
            }
        } else {
            panic!("expected List");
        }
    }

    #[test]
    #[ignore] // requires network
    fn test_tool_info() {
        let result =
            call_container_builtin("tool_info", vec![Value::Str("samtools".into())]).unwrap();
        if let Value::Record(rec) = &result {
            assert!(rec.contains_key("name"));
            assert!(rec.contains_key("pulls"));
            assert!(rec.contains_key("versions"));
            if let Some(Value::List(versions)) = rec.get("versions") {
                assert!(!versions.is_empty());
            }
        } else {
            panic!("expected Record");
        }
    }

    #[test]
    fn test_tool_list_returns_list() {
        // May return empty list if no runtime, but should not error
        if detect_runtime().is_none() {
            return;
        }
        let result = call_container_builtin("tool_list", vec![]).unwrap();
        assert!(matches!(result, Value::List(_)));
    }

    // Edge case: container_available always returns a Record even when no runtime is present
    #[test]
    fn test_container_available_always_record_structure() {
        let result = call_container_builtin("container_available", vec![]).unwrap();
        match &result {
            Value::Record(rec) => {
                // Must always have "runtime" key (Str or Nil)
                assert!(rec.contains_key("runtime"));
                // Must always have "image_dir" key
                assert!(rec.contains_key("image_dir"));
            }
            _ => panic!("container_available must always return a Record"),
        }
    }

    // Edge case: container_run with a missing/invalid image should error or return non-zero
    #[test]
    fn test_container_run_missing_image() {
        if detect_runtime().is_none() {
            // No runtime available — container_run should error about missing runtime
            let result = call_container_builtin(
                "container_run",
                vec![
                    Value::Str("nonexistent_image_xyz_12345:latest".into()),
                    Value::Str("echo hello".into()),
                ],
            );
            assert!(result.is_err());
            return;
        }
        // With a runtime, it should either error or return a Record with non-zero exit_code
        let result = call_container_builtin(
            "container_run",
            vec![
                Value::Str("nonexistent_image_xyz_12345:latest".into()),
                Value::Str("echo hello".into()),
            ],
        );
        match result {
            Ok(Value::Record(rec)) => {
                // The container runtime may return exit_code != 0
                if let Some(Value::Int(code)) = rec.get("exit_code") {
                    assert_ne!(*code, 0, "expected non-zero exit for missing image");
                }
            }
            Err(_) => {} // Also acceptable
            _ => panic!("expected Record or error"),
        }
    }

    // Edge case: tool_search with empty query (requires network)
    #[test]
    #[ignore] // requires network
    fn test_tool_search_empty_query() {
        let result =
            call_container_builtin("tool_search", vec![Value::Str("".into())]);
        // Should either return an empty list or an error, but not panic
        match result {
            Ok(Value::List(_)) => {}
            Err(_) => {}
            other => panic!("unexpected result: {:?}", other),
        }
    }

    // Edge case: unknown builtin name
    #[test]
    fn test_unknown_container_builtin() {
        let result = call_container_builtin("nonexistent_builtin", vec![]);
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("unknown container builtin"));
    }

    // Edge case: urlencoded helper
    #[test]
    fn test_urlencoded_special_chars() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a&b"), "a%26b");
        assert_eq!(urlencoded("x#y"), "x%23y");
        assert_eq!(urlencoded("no_special"), "no_special");
        assert_eq!(urlencoded(""), "");
    }

    // Edge case: run_output_to_record
    #[test]
    fn test_run_output_to_record_structure() {
        let output = std::process::Command::new("echo")
            .arg("test")
            .output();
        if let Ok(out) = output {
            let rec = run_output_to_record(out);
            if let Value::Record(map) = &rec {
                assert!(map.contains_key("stdout"));
                assert!(map.contains_key("stderr"));
                assert!(map.contains_key("exit_code"));
            } else {
                panic!("expected Record");
            }
        }
    }

    // Edge case: parse_opts with no extra args
    #[test]
    fn test_parse_opts_empty() {
        let args = vec![Value::Str("img".into()), Value::Str("cmd".into())];
        let (mount, workdir, env) = parse_opts(&args).unwrap();
        assert!(mount.is_none());
        assert!(workdir.is_none());
        assert!(env.is_empty());
    }

    // Edge case: parse_opts with non-Record third arg
    #[test]
    fn test_parse_opts_invalid_type() {
        let args = vec![
            Value::Str("img".into()),
            Value::Str("cmd".into()),
            Value::Int(42),
        ];
        assert!(parse_opts(&args).is_err());
    }
}
