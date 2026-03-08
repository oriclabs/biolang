//! Nextflow DSL2 file parser.
//!
//! Read-only extraction of processes, params, channels, and includes
//! from .nf files. No Nextflow runtime required.

use std::collections::HashMap;

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

// ---------------------------------------------------------------------------
// Public three-function interface
// ---------------------------------------------------------------------------

pub fn nf_parse_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("nf_parse", Arity::Exact(1)),
        ("nf_to_bl", Arity::Exact(1)),
        ("galaxy_to_bl", Arity::Exact(1)),
    ]
}

pub fn is_nf_parse_builtin(name: &str) -> bool {
    matches!(name, "nf_parse" | "nf_to_bl" | "galaxy_to_bl")
}

pub fn call_nf_parse_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "nf_parse" => {
            let path = require_str(&args[0], "nf_parse")?;
            parse_nextflow(path)
        }
        "nf_to_bl" => {
            let rec = require_record(&args[0], "nf_to_bl")?;
            Ok(Value::Str(generate_bl_from_nf(rec)?))
        }
        "galaxy_to_bl" => {
            let rec = require_record(&args[0], "galaxy_to_bl")?;
            Ok(Value::Str(generate_bl_from_galaxy(rec)?))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown nf_parse builtin '{name}'"),
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

fn require_record<'a>(val: &'a Value, func: &str) -> Result<&'a HashMap<String, Value>> {
    match val {
        Value::Record(r) => Ok(r),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Record, got {}", other.type_of()),
            None,
        )),
    }
}

/// Strip surrounding quotes (single or double) from a value string.
fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Convert a raw value string into an appropriate `Value`.
fn parse_value_str(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed == "true" {
        return Value::Bool(true);
    }
    if trimmed == "false" {
        return Value::Bool(false);
    }
    if trimmed == "null" || trimmed == "nil" {
        return Value::Nil;
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return Value::Int(n);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }
    // Quoted string — strip quotes
    Value::Str(strip_quotes(trimmed))
}

// ---------------------------------------------------------------------------
// Core parser
// ---------------------------------------------------------------------------

/// Parse a Nextflow (.nf) file and return its structure as a BioLang Record.
pub fn parse_nextflow(path: &str) -> Result<Value> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("nf_parse: cannot read '{}': {}", path, e),
            None,
        )
    })?;

    let params = extract_params(&content);
    let processes = extract_processes(&content);
    let includes = extract_includes(&content);
    let workflow = extract_workflow(&content);
    let dsl = detect_dsl(&content);

    let mut record = HashMap::new();
    record.insert("params".to_string(), params);
    record.insert("processes".to_string(), Value::List(processes));
    record.insert("includes".to_string(), Value::List(includes));
    record.insert("workflow".to_string(), Value::List(workflow));
    record.insert("dsl".to_string(), Value::Str(dsl));
    Ok(Value::Record(record))
}

// ---------------------------------------------------------------------------
// Params extraction
// ---------------------------------------------------------------------------

fn extract_params(content: &str) -> Value {
    let mut params: HashMap<String, Value> = HashMap::new();

    // Pattern 1: params.NAME = VALUE
    let re = regex::Regex::new(r#"(?m)^\s*params\.(\w+)\s*=\s*(.+?)\s*$"#).unwrap();
    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        let raw = cap[2].trim();
        params.insert(name, parse_value_str(raw));
    }

    // Pattern 2: params { NAME = VALUE } blocks
    let block_re = regex::Regex::new(r"(?s)params\s*\{([^}]*)\}").unwrap();
    let line_re = regex::Regex::new(r#"(?m)^\s*(\w+)\s*=\s*(.+?)\s*$"#).unwrap();
    for block in block_re.captures_iter(content) {
        let body = &block[1];
        for cap in line_re.captures_iter(body) {
            let name = cap[1].to_string();
            let raw = cap[2].trim();
            params.insert(name, parse_value_str(raw));
        }
    }

    Value::Record(params)
}

// ---------------------------------------------------------------------------
// Process extraction
// ---------------------------------------------------------------------------

fn extract_processes(content: &str) -> Vec<Value> {
    let mut processes = Vec::new();

    // Find each "process NAME {" and extract its body via brace matching.
    let proc_re = regex::Regex::new(r"(?m)^\s*process\s+(\w+)\s*\{").unwrap();
    for cap in proc_re.captures_iter(content) {
        let name = cap[1].to_string();
        let start = cap.get(0).unwrap().end(); // position after the opening brace
        if let Some(body) = extract_brace_body(content, start) {
            let inputs = extract_directive_block(&body, "input");
            let outputs = extract_directive_block(&body, "output");
            let script = extract_script_block(&body);
            let container = extract_simple_directive(&body, "container");
            let cpus = extract_simple_directive(&body, "cpus");
            let memory = extract_simple_directive(&body, "memory");
            let time = extract_simple_directive(&body, "time");

            let mut proc = HashMap::new();
            proc.insert("name".to_string(), Value::Str(name));
            proc.insert(
                "inputs".to_string(),
                Value::List(inputs.into_iter().map(|s| Value::Str(s)).collect()),
            );
            proc.insert(
                "outputs".to_string(),
                Value::List(outputs.into_iter().map(|s| Value::Str(s)).collect()),
            );
            proc.insert(
                "script".to_string(),
                match script {
                    Some(s) => Value::Str(s),
                    None => Value::Nil,
                },
            );
            proc.insert("container".to_string(), opt_to_value(container));
            proc.insert("cpus".to_string(), opt_to_value(cpus));
            proc.insert("memory".to_string(), opt_to_value(memory));
            proc.insert("time".to_string(), opt_to_value(time));
            processes.push(Value::Record(proc));
        }
    }

    processes
}

/// Given a position right after an opening `{`, return the body up to the
/// matching `}`. Handles nested braces (but not braces inside strings/comments
/// — good enough for structural parsing).
fn extract_brace_body(content: &str, start: usize) -> Option<String> {
    let bytes = content.as_bytes();
    let mut depth: i32 = 1;
    let mut i = start;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth == 0 {
        // body is everything between start and the matching `}`
        Some(content[start..i - 1].to_string())
    } else {
        None
    }
}

/// Extract the lines inside a Nextflow directive block (e.g., `input:` or `output:`).
/// Directive blocks end at the next directive label or at end of body.
fn extract_directive_block(body: &str, directive: &str) -> Vec<String> {
    let label = format!("{}:", directive);
    let lines: Vec<&str> = body.lines().collect();
    let mut collecting = false;
    let mut result = Vec::new();
    let directive_re = regex::Regex::new(r"^\s*\w+\s*:").unwrap();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with(&label) {
            collecting = true;
            // If there is content after the colon on the same line, include it
            let after = trimmed[label.len()..].trim();
            if !after.is_empty() {
                result.push(after.to_string());
            }
            continue;
        }
        if collecting {
            // Stop at the next directive label
            if directive_re.is_match(trimmed) && !trimmed.is_empty() {
                break;
            }
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
        }
    }
    result
}

/// Extract the script block content. Nextflow uses triple-quoted strings
/// (`"""..."""` or `'''...'''`) or a bare `script:` section.
fn extract_script_block(body: &str) -> Option<String> {
    // Try triple-double-quote first
    if let Some(s) = extract_between_triple_quotes(body, "\"\"\"") {
        return Some(s);
    }
    // Try triple-single-quote
    if let Some(s) = extract_between_triple_quotes(body, "'''") {
        return Some(s);
    }
    // Fallback: look for `script:` section and grab non-directive lines
    let lines: Vec<&str> = body.lines().collect();
    let directive_re = regex::Regex::new(r"^\s*\w+\s*:").unwrap();
    let mut collecting = false;
    let mut script_lines = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "script:" || trimmed.starts_with("script:") {
            collecting = true;
            let after = trimmed.strip_prefix("script:").unwrap_or("").trim();
            if !after.is_empty() {
                script_lines.push(after.to_string());
            }
            continue;
        }
        if collecting {
            if directive_re.is_match(trimmed) && !trimmed.is_empty() {
                break;
            }
            if !trimmed.is_empty() {
                script_lines.push(trimmed.to_string());
            }
        }
    }
    if script_lines.is_empty() {
        None
    } else {
        Some(script_lines.join("\n"))
    }
}

fn extract_between_triple_quotes(body: &str, quote: &str) -> Option<String> {
    let first = body.find(quote)?;
    let start = first + quote.len();
    let rest = &body[start..];
    let end = rest.find(quote)?;
    let inner = rest[..end].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

/// Extract a simple process directive like `container 'image'` or `cpus 4`.
fn extract_simple_directive(body: &str, directive: &str) -> Option<String> {
    let re_str = format!(r"(?m)^\s*{}\s+(.+?)\s*$", regex::escape(directive));
    let re = regex::Regex::new(&re_str).unwrap();
    re.captures(body)
        .map(|cap| strip_quotes(cap[1].trim()))
}

fn opt_to_value(opt: Option<String>) -> Value {
    match opt {
        Some(s) => Value::Str(s),
        None => Value::Nil,
    }
}

// ---------------------------------------------------------------------------
// Includes extraction
// ---------------------------------------------------------------------------

fn extract_includes(content: &str) -> Vec<Value> {
    let mut includes = Vec::new();

    // Pattern: include { NAME } from 'path'
    // Pattern: include { NAME as ALIAS } from 'path'
    // Pattern: include { NAME; NAME2 } from 'path' — multiple imports
    let include_re = regex::Regex::new(
        r#"include\s*\{([^}]+)\}\s*from\s*['"]([^'"]+)['"]"#,
    )
    .unwrap();
    let item_re = regex::Regex::new(r"(\w+)(?:\s+as\s+(\w+))?").unwrap();

    for cap in include_re.captures_iter(content) {
        let items = &cap[1];
        let from = cap[2].to_string();
        for item in item_re.captures_iter(items) {
            let name = item[1].to_string();
            let alias = item.get(2).map(|m| m.as_str().to_string());
            let mut rec = HashMap::new();
            rec.insert("name".to_string(), Value::Str(name));
            rec.insert(
                "alias".to_string(),
                match alias {
                    Some(a) => Value::Str(a),
                    None => Value::Nil,
                },
            );
            rec.insert("from".to_string(), Value::Str(from.clone()));
            includes.push(Value::Record(rec));
        }
    }

    includes
}

// ---------------------------------------------------------------------------
// Workflow extraction
// ---------------------------------------------------------------------------

fn extract_workflow(content: &str) -> Vec<Value> {
    // Match: workflow { ... } or workflow NAME { ... }
    let wf_re = regex::Regex::new(r"(?m)^\s*workflow\s*(?:\w+\s*)?\{").unwrap();
    if let Some(m) = wf_re.find(content) {
        let start = m.end();
        if let Some(body) = extract_brace_body(content, start) {
            return body
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| Value::Str(l.to_string()))
                .collect();
        }
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// DSL detection
// ---------------------------------------------------------------------------

fn detect_dsl(content: &str) -> String {
    // Check for explicit DSL declaration
    let dsl_re = regex::Regex::new(r"(?m)^\s*nextflow\.enable\.dsl\s*=\s*(\d)").unwrap();
    if let Some(cap) = dsl_re.captures(content) {
        return format!("DSL{}", &cap[1]);
    }
    // Heuristic: if there is a `workflow {` block, assume DSL2
    if regex::Regex::new(r"(?m)^\s*workflow\s*\{").unwrap().is_match(content) {
        return "DSL2".to_string();
    }
    // Default
    "DSL2".to_string()
}

// ---------------------------------------------------------------------------
// Code generation: nf_to_bl
// ---------------------------------------------------------------------------

/// Generate BioLang pipeline code from a parsed Nextflow Record (output of nf_parse).
fn generate_bl_from_nf(rec: &HashMap<String, Value>) -> Result<String> {
    let mut out = String::new();

    // Header comment
    out.push_str("# Generated from Nextflow pipeline\n\n");

    // Extract params as variables
    if let Some(Value::Record(params)) = rec.get("params") {
        if !params.is_empty() {
            out.push_str("# Parameters\n");
            let mut keys: Vec<&String> = params.keys().collect();
            keys.sort();
            for key in keys {
                let val = &params[key];
                out.push_str(&format!("{} = {}\n", key, value_to_bl_literal(val)));
            }
            out.push('\n');
        }
    }

    // Extract processes as pipeline stages
    let processes = match rec.get("processes") {
        Some(Value::List(l)) => l.clone(),
        _ => Vec::new(),
    };

    // Extract workflow steps to determine ordering
    let workflow = match rec.get("workflow") {
        Some(Value::List(l)) => l.clone(),
        _ => Vec::new(),
    };

    if !processes.is_empty() {
        out.push_str("pipeline {\n");

        for proc_val in &processes {
            if let Value::Record(proc) = proc_val {
                let name = match proc.get("name") {
                    Some(Value::Str(s)) => s.clone(),
                    _ => continue,
                };
                let snake = to_snake_case(&name);

                out.push_str(&format!("  stage {} {{\n", snake));

                // Container
                if let Some(Value::Str(img)) = proc.get("container") {
                    out.push_str(&format!("    container: \"{}\"\n", img));
                }

                // Resources
                if let Some(Value::Str(cpus)) = proc.get("cpus") {
                    out.push_str(&format!("    cpus: {}\n", cpus));
                }
                if let Some(Value::Str(mem)) = proc.get("memory") {
                    out.push_str(&format!("    memory: \"{}\"\n", mem));
                }

                // Inputs
                if let Some(Value::List(inputs)) = proc.get("inputs") {
                    if !inputs.is_empty() {
                        out.push_str("    inputs: [");
                        let strs: Vec<String> = inputs
                            .iter()
                            .filter_map(|v| match v {
                                Value::Str(s) => Some(format!("\"{}\"", s)),
                                _ => None,
                            })
                            .collect();
                        out.push_str(&strs.join(", "));
                        out.push_str("]\n");
                    }
                }

                // Outputs
                if let Some(Value::List(outputs)) = proc.get("outputs") {
                    if !outputs.is_empty() {
                        out.push_str("    outputs: [");
                        let strs: Vec<String> = outputs
                            .iter()
                            .filter_map(|v| match v {
                                Value::Str(s) => Some(format!("\"{}\"", s)),
                                _ => None,
                            })
                            .collect();
                        out.push_str(&strs.join(", "));
                        out.push_str("]\n");
                    }
                }

                // Script as a run block
                if let Some(Value::Str(script)) = proc.get("script") {
                    out.push_str(&format!(
                        "    run: \"{}\"\n",
                        script.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
                    ));
                }

                out.push_str("  }\n\n");
            }
        }

        // Workflow connections
        if !workflow.is_empty() {
            out.push_str("  # Workflow\n");
            for step in &workflow {
                if let Value::Str(s) = step {
                    out.push_str(&format!("  # {}\n", s));
                }
            }
            out.push('\n');
        }

        out.push_str("}\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Code generation: galaxy_to_bl
// ---------------------------------------------------------------------------

/// Generate BioLang pipeline code from a Galaxy workflow Record.
///
/// Expects a Record with:
///   - name: Str (workflow name)
///   - steps: List of Records with {name, tool_id, inputs, outputs}
///   - annotation: Str (optional description)
fn generate_bl_from_galaxy(rec: &HashMap<String, Value>) -> Result<String> {
    let mut out = String::new();

    let wf_name = match rec.get("name") {
        Some(Value::Str(s)) => s.clone(),
        _ => "galaxy_workflow".to_string(),
    };

    out.push_str(&format!("# Generated from Galaxy workflow: {}\n", wf_name));

    if let Some(Value::Str(ann)) = rec.get("annotation") {
        if !ann.is_empty() {
            out.push_str(&format!("# {}\n", ann));
        }
    }
    out.push('\n');

    // Extract parameters/inputs at workflow level
    if let Some(Value::Record(params)) = rec.get("params") {
        if !params.is_empty() {
            out.push_str("# Parameters\n");
            let mut keys: Vec<&String> = params.keys().collect();
            keys.sort();
            for key in keys {
                let val = &params[key];
                out.push_str(&format!("{} = {}\n", key, value_to_bl_literal(val)));
            }
            out.push('\n');
        }
    }

    let steps = match rec.get("steps") {
        Some(Value::List(l)) => l.clone(),
        _ => Vec::new(),
    };

    if !steps.is_empty() {
        out.push_str("pipeline {\n");

        for step_val in &steps {
            if let Value::Record(step) = step_val {
                let name = match step.get("name") {
                    Some(Value::Str(s)) => to_snake_case(s),
                    _ => continue,
                };
                let tool_id = match step.get("tool_id") {
                    Some(Value::Str(s)) => Some(s.clone()),
                    _ => None,
                };

                out.push_str(&format!("  stage {} {{\n", name));

                if let Some(tid) = &tool_id {
                    out.push_str(&format!("    tool: \"{}\"\n", tid));
                }

                // Inputs
                if let Some(Value::List(inputs)) = step.get("inputs") {
                    if !inputs.is_empty() {
                        out.push_str("    inputs: [");
                        let strs: Vec<String> = inputs
                            .iter()
                            .filter_map(|v| match v {
                                Value::Str(s) => Some(format!("\"{}\"", s)),
                                _ => None,
                            })
                            .collect();
                        out.push_str(&strs.join(", "));
                        out.push_str("]\n");
                    }
                }

                // Outputs
                if let Some(Value::List(outputs)) = step.get("outputs") {
                    if !outputs.is_empty() {
                        out.push_str("    outputs: [");
                        let strs: Vec<String> = outputs
                            .iter()
                            .filter_map(|v| match v {
                                Value::Str(s) => Some(format!("\"{}\"", s)),
                                _ => None,
                            })
                            .collect();
                        out.push_str(&strs.join(", "));
                        out.push_str("]\n");
                    }
                }

                // Tool parameters
                if let Some(Value::Record(tool_params)) = step.get("params") {
                    if !tool_params.is_empty() {
                        let mut keys: Vec<&String> = tool_params.keys().collect();
                        keys.sort();
                        for key in keys {
                            let val = &tool_params[key];
                            out.push_str(&format!(
                                "    {}: {}\n",
                                key,
                                value_to_bl_literal(val)
                            ));
                        }
                    }
                }

                out.push_str("  }\n\n");
            }
        }

        out.push_str("}\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Codegen helpers
// ---------------------------------------------------------------------------

fn value_to_bl_literal(val: &Value) -> String {
    match val {
        Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format!("{f}"),
        Value::Bool(b) => b.to_string(),
        Value::Nil => "nil".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(value_to_bl_literal).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Record(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|k| format!("{}: {}", k, value_to_bl_literal(&map[*k])))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        other => format!("\"{}\"", other),
    }
}

fn to_snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '-' || ch == ' ' {
            result.push('_');
        } else if ch.is_uppercase() {
            // Insert underscore before uppercase if:
            // - not at start
            // - previous char is lowercase, OR
            // - next char is lowercase (handles "FastQC" -> "fast_qc")
            if i > 0 && !result.ends_with('_') {
                let prev_lower = chars[i - 1].is_lowercase();
                let next_lower = chars.get(i + 1).map_or(false, |c| c.is_lowercase());
                if prev_lower || next_lower {
                    result.push('_');
                }
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_nf(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::with_suffix(".nf").unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_parse_params_dot_syntax() {
        let nf = write_temp_nf(
            r#"
params.reads = "data/*.fastq.gz"
params.genome = "GRCh38"
params.threads = 8
params.flag = true
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let params = match rec.get("params").unwrap() {
            Value::Record(p) => p,
            _ => panic!("expected params Record"),
        };
        assert_eq!(params.get("reads").unwrap(), &Value::Str("data/*.fastq.gz".to_string()));
        assert_eq!(params.get("genome").unwrap(), &Value::Str("GRCh38".to_string()));
        assert_eq!(params.get("threads").unwrap(), &Value::Int(8));
        assert_eq!(params.get("flag").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn test_parse_params_block_syntax() {
        let nf = write_temp_nf(
            r#"
params {
    outdir = "./results"
    min_quality = 20
}
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let params = match rec.get("params").unwrap() {
            Value::Record(p) => p,
            _ => panic!("expected params Record"),
        };
        assert_eq!(params.get("outdir").unwrap(), &Value::Str("./results".to_string()));
        assert_eq!(params.get("min_quality").unwrap(), &Value::Int(20));
    }

    #[test]
    fn test_parse_process() {
        let nf = write_temp_nf(
            r#"
process ALIGN {
    container 'quay.io/biocontainers/bwa:0.7.17'
    cpus 4
    memory '8 GB'

    input:
    tuple val(sample_id), path(reads)
    path genome

    output:
    tuple val(sample_id), path("*.bam")

    script:
    """
    bwa mem ${genome} ${reads} | samtools sort -o ${sample_id}.bam
    """
}
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let procs = match rec.get("processes").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(procs.len(), 1);
        let proc = match &procs[0] {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(proc.get("name").unwrap(), &Value::Str("ALIGN".to_string()));
        assert_eq!(
            proc.get("container").unwrap(),
            &Value::Str("quay.io/biocontainers/bwa:0.7.17".to_string())
        );
        assert_eq!(proc.get("cpus").unwrap(), &Value::Str("4".to_string()));
        assert_eq!(proc.get("memory").unwrap(), &Value::Str("8 GB".to_string()));

        let inputs = match proc.get("inputs").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], Value::Str("tuple val(sample_id), path(reads)".to_string()));
        assert_eq!(inputs[1], Value::Str("path genome".to_string()));

        let outputs = match proc.get("outputs").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(outputs.len(), 1);

        let script = match proc.get("script").unwrap() {
            Value::Str(s) => s.clone(),
            _ => panic!("expected Str"),
        };
        assert!(script.contains("bwa mem"));
    }

    #[test]
    fn test_parse_includes() {
        let nf = write_temp_nf(
            r#"
include { FASTQC } from './modules/fastqc'
include { TRIMGALORE as TRIM } from './modules/trimgalore'
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let includes = match rec.get("includes").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(includes.len(), 2);

        let inc0 = match &includes[0] {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(inc0.get("name").unwrap(), &Value::Str("FASTQC".to_string()));
        assert_eq!(inc0.get("alias").unwrap(), &Value::Nil);
        assert_eq!(inc0.get("from").unwrap(), &Value::Str("./modules/fastqc".to_string()));

        let inc1 = match &includes[1] {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(inc1.get("name").unwrap(), &Value::Str("TRIMGALORE".to_string()));
        assert_eq!(inc1.get("alias").unwrap(), &Value::Str("TRIM".to_string()));
        assert_eq!(inc1.get("from").unwrap(), &Value::Str("./modules/trimgalore".to_string()));
    }

    #[test]
    fn test_parse_workflow() {
        let nf = write_temp_nf(
            r#"
workflow {
    reads_ch = Channel.fromFilePairs(params.reads)
    FASTQC(reads_ch)
    TRIMGALORE(reads_ch)
}
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let wf = match rec.get("workflow").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(wf.len(), 3);
        assert_eq!(
            wf[0],
            Value::Str("reads_ch = Channel.fromFilePairs(params.reads)".to_string())
        );
        assert_eq!(wf[1], Value::Str("FASTQC(reads_ch)".to_string()));
        assert_eq!(wf[2], Value::Str("TRIMGALORE(reads_ch)".to_string()));
    }

    #[test]
    fn test_dsl_detection() {
        assert_eq!(detect_dsl("nextflow.enable.dsl = 2\n"), "DSL2");
        assert_eq!(detect_dsl("nextflow.enable.dsl = 1\n"), "DSL1");
        assert_eq!(detect_dsl("workflow {\n  FOO()\n}\n"), "DSL2");
        assert_eq!(detect_dsl("// just a comment"), "DSL2");
    }

    #[test]
    fn test_full_pipeline() {
        let nf = write_temp_nf(
            r#"
nextflow.enable.dsl = 2

params.reads = "data/*.fastq.gz"
params.genome = "GRCh38"
params.outdir = "./results"

include { FASTQC } from './modules/fastqc'
include { TRIMGALORE } from './modules/trimgalore'

process ALIGN {
    input:
    tuple val(sample_id), path(reads)
    path genome

    output:
    tuple val(sample_id), path("*.bam")

    script:
    """
    bwa mem ${genome} ${reads} | samtools sort -o ${sample_id}.bam
    """
}

workflow {
    reads_ch = Channel.fromFilePairs(params.reads)
    FASTQC(reads_ch)
    TRIMGALORE(reads_ch)
    ALIGN(reads_ch, params.genome)
}
"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.get("dsl").unwrap(), &Value::Str("DSL2".to_string()));

        // Params
        let params = match rec.get("params").unwrap() {
            Value::Record(p) => p,
            _ => panic!("expected Record"),
        };
        assert_eq!(params.len(), 3);

        // Processes
        let procs = match rec.get("processes").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(procs.len(), 1);

        // Includes
        let includes = match rec.get("includes").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(includes.len(), 2);

        // Workflow
        let wf = match rec.get("workflow").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(wf.len(), 4);
    }

    #[test]
    fn test_file_not_found() {
        let result = parse_nextflow("/nonexistent/file.nf");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_includes_single_line() {
        let nf = write_temp_nf(
            r#"include { FOO; BAR } from './modules/shared'"#,
        );
        let result = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let includes = match rec.get("includes").unwrap() {
            Value::List(l) => l,
            _ => panic!("expected List"),
        };
        assert_eq!(includes.len(), 2);
    }

    #[test]
    fn test_nf_to_bl_basic() {
        let nf = write_temp_nf(
            r#"
params.reads = "data/*.fastq.gz"
params.genome = "GRCh38"

process FASTQC {
    container 'quay.io/biocontainers/fastqc:0.12.1'
    cpus 2

    input:
    path reads

    output:
    path "*.html"

    script:
    """
    fastqc ${reads}
    """
}

workflow {
    FASTQC(params.reads)
}
"#,
        );
        let parsed = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match &parsed {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let bl = generate_bl_from_nf(rec).unwrap();
        assert!(bl.contains("# Generated from Nextflow pipeline"));
        assert!(bl.contains("reads = \"data/*.fastq.gz\""));
        assert!(bl.contains("genome = \"GRCh38\""));
        assert!(bl.contains("pipeline {"));
        assert!(bl.contains("stage fastqc"));
        assert!(bl.contains("container: \"quay.io/biocontainers/fastqc:0.12.1\""));
        assert!(bl.contains("cpus: 2"));
    }

    #[test]
    fn test_nf_to_bl_roundtrip() {
        let nf = write_temp_nf(
            r#"
params.input = "samples.csv"

process ALIGN {
    input:
    tuple val(id), path(reads)

    output:
    path "*.bam"

    script:
    """
    bwa mem ref.fa ${reads} > ${id}.bam
    """
}

process SORT {
    input:
    path bam

    output:
    path "*.sorted.bam"

    script:
    """
    samtools sort ${bam} -o ${bam}.sorted.bam
    """
}

workflow {
    ALIGN(params.input)
    SORT(ALIGN.out)
}
"#,
        );
        let parsed = parse_nextflow(nf.path().to_str().unwrap()).unwrap();
        let rec = match &parsed {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let bl = generate_bl_from_nf(rec).unwrap();
        assert!(bl.contains("stage align"));
        assert!(bl.contains("stage sort"));
        assert!(bl.contains("# ALIGN(params.input)"));
        assert!(bl.contains("# SORT(ALIGN.out)"));
    }

    #[test]
    fn test_galaxy_to_bl() {
        let mut wf = HashMap::new();
        wf.insert("name".to_string(), Value::Str("RNA-seq Analysis".to_string()));
        wf.insert("annotation".to_string(), Value::Str("Basic RNA-seq pipeline".to_string()));

        let mut step1 = HashMap::new();
        step1.insert("name".to_string(), Value::Str("FastQC".to_string()));
        step1.insert("tool_id".to_string(), Value::Str("toolshed.g2.bx.psu.edu/repos/devteam/fastqc/fastqc/0.74".to_string()));
        step1.insert("inputs".to_string(), Value::List(vec![Value::Str("input_file".to_string())]));
        step1.insert("outputs".to_string(), Value::List(vec![Value::Str("html_file".to_string())]));

        let mut step2 = HashMap::new();
        step2.insert("name".to_string(), Value::Str("HISAT2".to_string()));
        step2.insert("tool_id".to_string(), Value::Str("toolshed.g2.bx.psu.edu/repos/iuc/hisat2/hisat2/2.2.1".to_string()));
        step2.insert("inputs".to_string(), Value::List(vec![Value::Str("input_reads".to_string())]));
        step2.insert("outputs".to_string(), Value::List(vec![Value::Str("aligned_bam".to_string())]));

        wf.insert("steps".to_string(), Value::List(vec![
            Value::Record(step1),
            Value::Record(step2),
        ]));

        let bl = generate_bl_from_galaxy(&wf).unwrap();
        assert!(bl.contains("# Generated from Galaxy workflow: RNA-seq Analysis"));
        assert!(bl.contains("# Basic RNA-seq pipeline"));
        assert!(bl.contains("pipeline {"));
        assert!(bl.contains("stage fast_qc") || bl.contains("stage fastqc"),
            "expected stage name, got:\n{}", bl);
        assert!(bl.contains("stage hisat2"));
        assert!(bl.contains("tool: \"toolshed.g2.bx.psu.edu/repos/devteam/fastqc/fastqc/0.74\""));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("FASTQC"), "fastqc");
        assert_eq!(to_snake_case("FastQC"), "fast_qc");
        assert_eq!(to_snake_case("align"), "align");
        assert_eq!(to_snake_case("ALIGN"), "align");
        assert_eq!(to_snake_case("MyProcess"), "my_process");
        assert_eq!(to_snake_case("bwa-mem"), "bwa_mem");
        assert_eq!(to_snake_case("RNA-seq Analysis"), "rna_seq_analysis");
    }

    #[test]
    fn test_value_to_bl_literal() {
        assert_eq!(value_to_bl_literal(&Value::Str("hello".into())), "\"hello\"");
        assert_eq!(value_to_bl_literal(&Value::Int(42)), "42");
        assert_eq!(value_to_bl_literal(&Value::Bool(true)), "true");
        assert_eq!(value_to_bl_literal(&Value::Nil), "nil");
        assert_eq!(
            value_to_bl_literal(&Value::List(vec![Value::Int(1), Value::Int(2)])),
            "[1, 2]"
        );
    }
}
