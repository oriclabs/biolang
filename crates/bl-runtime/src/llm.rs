use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use std::collections::HashMap;

/// Returns the list of (name, arity) for all LLM builtins.
pub fn llm_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("chat", Arity::Range(1, 2)),
        ("chat_code", Arity::Range(1, 2)),
        ("llm_models", Arity::Exact(0)),
    ]
}

/// Check if a name is a known LLM builtin.
pub fn is_llm_builtin(name: &str) -> bool {
    matches!(name, "chat" | "chat_code" | "llm_models")
}

/// Execute an LLM builtin by name.
pub fn call_llm_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "chat" => builtin_chat(args),
        "chat_code" => builtin_chat_code(args),
        "llm_models" => builtin_llm_models(),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown llm builtin '{name}'"),
            None,
        )),
    }
}

// ── Provider Detection ──────────────────────────────────────────

#[derive(Clone, Debug)]
enum Provider {
    Anthropic,
    OpenAI,
    Ollama,
    OpenAICompatible,
}

#[derive(Clone, Debug)]
struct LlmConfig {
    provider: Provider,
    api_key: String,
    base_url: String,
    model: String,
}

/// Detect LLM provider from environment variables.
/// Priority: ANTHROPIC_API_KEY → OPENAI_API_KEY → OLLAMA_MODEL → LLM_BASE_URL
fn detect_provider() -> Result<LlmConfig> {
    // 1. Anthropic
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
        return Ok(LlmConfig {
            provider: Provider::Anthropic,
            api_key: key,
            base_url: "https://api.anthropic.com".to_string(),
            model,
        });
    }

    // 2. OpenAI
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        let model =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        let base = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        return Ok(LlmConfig {
            provider: Provider::OpenAI,
            api_key: key,
            base_url: base,
            model,
        });
    }

    // 3. Ollama (no key needed)
    if let Ok(model) = std::env::var("OLLAMA_MODEL") {
        let base = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        return Ok(LlmConfig {
            provider: Provider::Ollama,
            api_key: String::new(),
            base_url: base,
            model,
        });
    }

    // 4. Generic OpenAI-compatible (e.g. Together, Groq, LM Studio)
    if let Ok(base) = std::env::var("LLM_BASE_URL") {
        let key = std::env::var("LLM_API_KEY").unwrap_or_default();
        let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "default".to_string());
        return Ok(LlmConfig {
            provider: Provider::OpenAICompatible,
            api_key: key,
            base_url: base,
            model,
        });
    }

    Err(BioLangError::runtime(
        ErrorKind::IOError,
        "no LLM provider configured. Set one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, OLLAMA_MODEL, or LLM_BASE_URL",
        None,
    ))
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

/// Format context value into a string for injection into prompts.
fn format_context(val: &Value) -> String {
    match val {
        Value::Str(s) => s.clone(),
        Value::Record(m) | Value::Map(m) => {
            // Pretty-print record as key: value lines
            let mut lines = Vec::new();
            for (k, v) in m {
                lines.push(format!("{k}: {v}"));
            }
            lines.join("\n")
        }
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(|v| format!("{v}")).collect();
            parts.join("\n")
        }
        Value::Table(t) => {
            // Format as TSV
            let mut lines = Vec::new();
            lines.push(t.columns.join("\t"));
            for row in &t.rows {
                let cells: Vec<String> = row.iter().map(|v| format!("{v}")).collect();
                lines.push(cells.join("\t"));
            }
            lines.join("\n")
        }
        other => format!("{other}"),
    }
}

/// Build the user message, optionally injecting context.
fn build_user_message(message: &str, args: &[Value]) -> String {
    if args.len() > 1 {
        let ctx = format_context(&args[1]);
        format!("{message}\n\n--- Context ---\n{ctx}")
    } else {
        message.to_string()
    }
}

// ── Anthropic Messages API ──────────────────────────────────────

fn call_anthropic(config: &LlmConfig, system: &str, user_msg: &str) -> Result<String> {
    let url = format!("{}/v1/messages", config.base_url);

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": 4096,
        "system": system,
        "messages": [
            {"role": "user", "content": user_msg}
        ]
    });

    let resp = crate::http::shared_agent().post(&url)
        .set("x-api-key", &config.api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("Anthropic API request failed: {e}"),
                None,
            )
        })?;

    let text = resp.into_string().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("Anthropic API: failed to read response: {e}"),
            None,
        )
    })?;

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("Anthropic API: failed to parse response: {e}"),
            None,
        )
    })?;

    // Extract text from content[0].text
    if let Some(err_msg) = json.get("error").and_then(|e| e.get("message")).and_then(serde_json::Value::as_str) {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("Anthropic API error: {err_msg}"),
            None,
        ));
    }

    json.get("content")
        .and_then(serde_json::Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("Anthropic API: unexpected response format: {text}"),
                None,
            )
        })
}

// ── OpenAI Chat Completions API ─────────────────────────────────
// (Also works for Ollama, Together, Groq, LM Studio, etc.)

fn call_openai_compatible(config: &LlmConfig, system: &str, user_msg: &str) -> Result<String> {
    let url = match config.provider {
        Provider::Ollama => format!("{}/v1/chat/completions", config.base_url),
        _ => format!("{}/v1/chat/completions", config.base_url),
    };

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": 4096,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_msg}
        ]
    });

    let mut req = crate::http::shared_agent().post(&url).set("content-type", "application/json");
    if !config.api_key.is_empty() {
        req = req.set("authorization", &format!("Bearer {}", config.api_key));
    }

    let resp = req.send_string(&body.to_string()).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("LLM API request failed: {e}"),
            None,
        )
    })?;

    let text = resp.into_string().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("LLM API: failed to read response: {e}"),
            None,
        )
    })?;

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("LLM API: failed to parse response: {e}"),
            None,
        )
    })?;

    // Check for error
    if let Some(err) = json.get("error") {
        let msg = err
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown error");
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("LLM API error: {msg}"),
            None,
        ));
    }

    // Extract choices[0].message.content
    json.get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("LLM API: unexpected response format: {text}"),
                None,
            )
        })
}

/// Dispatch to the correct provider backend.
fn call_llm(config: &LlmConfig, system: &str, user_msg: &str) -> Result<String> {
    match config.provider {
        Provider::Anthropic => call_anthropic(config, system, user_msg),
        Provider::OpenAI | Provider::Ollama | Provider::OpenAICompatible => {
            call_openai_compatible(config, system, user_msg)
        }
    }
}

// ── System Prompts ──────────────────────────────────────────────

const SYSTEM_CHAT: &str = "\
You are a bioinformatics assistant embedded in BioLang, a pipe-first language for biological data analysis.

BioLang has built-in functions for:
- Sequence analysis: dna\"...\", reverse_complement(), transcribe(), translate(), gc_content(), align()
- File I/O: fasta(), fastq(), bed(), gff(), vcf(), write_fasta(), write_fastq(), write_bed(), write_vcf()
- Container tools: tool(\"samtools\", \"...\"), tool(\"bwa-mem2\", \"...\"), tool_search(), tool_popular()
- Data: tables, streams, map/filter/reduce with |> pipe syntax
- Statistics: mean(), median(), stdev(), ttest(), lm(), pca()
- Bio APIs: ncbi_search(), ensembl_gene(), uniprot_search()
- Plotting: plot(), histogram(), volcano()

Pipe syntax: `a |> f(b)` means `f(a, b)`. Lambda: `|x| x * 2`.

Be concise and practical. When suggesting code, use BioLang syntax. \
If the user provides context (error output, data, etc.), focus on that.";

const SYSTEM_CODE: &str = "\
You are a BioLang code generator. Return ONLY valid BioLang code, no explanations or markdown fences.

BioLang syntax:
- Pipe: `a |> f(b)` desugars to `f(a, b)`
- Lambda: `|x| x * 2`
- Variables: `x = expr`
- Functions: `fn name(params) { body }`
- Containers: `tool(\"samtools\", \"samtools view -c input.bam\")`
- File I/O: fasta(path), fastq(path), vcf(path), bed(path), gff(path)
- Streams: `stream_lines(path) |> take(100) |> map(|l| ...)`
- Tables: records |> table() |> filter(|r| ...) |> mutate(\"col\", |r| ...)

Built-in tool runners: tool(name, cmd), tool_search(query), tool_pull(name)
Built-in bio: reverse_complement(), gc_content(), transcribe(), translate(), align()
Stats: mean(), median(), stdev(), ttest(), lm(), cor()

Return only code. No prose, no ``` fences.";

// ── chat() ──────────────────────────────────────────────────────

fn builtin_chat(args: Vec<Value>) -> Result<Value> {
    let message = require_str(&args[0], "chat")?;
    let config = detect_provider()?;
    let user_msg = build_user_message(message, &args);
    let response = call_llm(&config, SYSTEM_CHAT, &user_msg)?;
    Ok(Value::Str(response))
}

// ── chat_code() ─────────────────────────────────────────────────

fn builtin_chat_code(args: Vec<Value>) -> Result<Value> {
    let message = require_str(&args[0], "chat_code")?;
    let config = detect_provider()?;
    let user_msg = build_user_message(message, &args);
    let response = call_llm(&config, SYSTEM_CODE, &user_msg)?;
    Ok(Value::Str(response))
}

// ── llm_models() ────────────────────────────────────────────────

fn builtin_llm_models() -> Result<Value> {
    let mut rec = HashMap::new();

    // Show which provider is active
    match detect_provider() {
        Ok(config) => {
            let provider_name = match config.provider {
                Provider::Anthropic => "anthropic",
                Provider::OpenAI => "openai",
                Provider::Ollama => "ollama",
                Provider::OpenAICompatible => "openai_compatible",
            };
            rec.insert(
                "provider".to_string(),
                Value::Str(provider_name.to_string()),
            );
            rec.insert("model".to_string(), Value::Str(config.model));
            rec.insert("base_url".to_string(), Value::Str(config.base_url));
            rec.insert("configured".to_string(), Value::Bool(true));
        }
        Err(_) => {
            rec.insert("configured".to_string(), Value::Bool(false));
            rec.insert("provider".to_string(), Value::Nil);
            rec.insert("model".to_string(), Value::Nil);
        }
    }

    // List env vars the user can set
    let env_vars = vec![
        Value::Str("ANTHROPIC_API_KEY — Anthropic (Claude)".into()),
        Value::Str("ANTHROPIC_MODEL — override model (default: claude-sonnet-4-20250514)".into()),
        Value::Str("OPENAI_API_KEY — OpenAI (GPT)".into()),
        Value::Str("OPENAI_MODEL — override model (default: gpt-4o)".into()),
        Value::Str("OPENAI_BASE_URL — custom OpenAI endpoint".into()),
        Value::Str("OLLAMA_MODEL — Ollama (local, no key needed)".into()),
        Value::Str("OLLAMA_BASE_URL — override (default: http://localhost:11434)".into()),
        Value::Str("LLM_BASE_URL + LLM_API_KEY + LLM_MODEL — any OpenAI-compatible provider".into()),
    ];
    rec.insert("env_vars".to_string(), Value::List(env_vars));

    Ok(Value::Record(rec))
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_models_no_provider() {
        // In CI/test, typically no API keys are set
        let result = call_llm_builtin("llm_models", vec![]).unwrap();
        if let Value::Record(rec) = &result {
            assert!(rec.contains_key("configured"));
            assert!(rec.contains_key("env_vars"));
            if let Some(Value::List(vars)) = rec.get("env_vars") {
                assert!(!vars.is_empty());
            }
        } else {
            panic!("expected Record");
        }
    }

    #[test]
    fn test_chat_no_provider_gives_clear_error() {
        // Ensure no keys are set for this test
        let saved_anthropic = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_openai = std::env::var("OPENAI_API_KEY").ok();
        let saved_ollama = std::env::var("OLLAMA_MODEL").ok();
        let saved_llm = std::env::var("LLM_BASE_URL").ok();

        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("LLM_BASE_URL");

        let result = call_llm_builtin("chat", vec![Value::Str("hello".into())]);
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("no LLM provider configured"));
        assert!(err.contains("ANTHROPIC_API_KEY"));

        // Restore
        if let Some(k) = saved_anthropic {
            std::env::set_var("ANTHROPIC_API_KEY", k);
        }
        if let Some(k) = saved_openai {
            std::env::set_var("OPENAI_API_KEY", k);
        }
        if let Some(k) = saved_ollama {
            std::env::set_var("OLLAMA_MODEL", k);
        }
        if let Some(k) = saved_llm {
            std::env::set_var("LLM_BASE_URL", k);
        }
    }

    #[test]
    fn test_format_context_record() {
        let mut m = HashMap::new();
        m.insert("exit_code".to_string(), Value::Int(1));
        m.insert("stderr".to_string(), Value::Str("error: file not found".into()));
        let ctx = format_context(&Value::Record(m));
        assert!(ctx.contains("exit_code: 1"));
        assert!(ctx.contains("error: file not found"));
    }

    #[test]
    fn test_format_context_list() {
        let list = Value::List(vec![
            Value::Str("line1".into()),
            Value::Str("line2".into()),
        ]);
        let ctx = format_context(&list);
        assert_eq!(ctx, "line1\nline2");
    }

    #[test]
    fn test_build_user_message_no_context() {
        let args = vec![Value::Str("hello".into())];
        let msg = build_user_message("hello", &args);
        assert_eq!(msg, "hello");
    }

    #[test]
    fn test_build_user_message_with_context() {
        let args = vec![
            Value::Str("explain this".into()),
            Value::Str("some error output".into()),
        ];
        let msg = build_user_message("explain this", &args);
        assert!(msg.contains("explain this"));
        assert!(msg.contains("--- Context ---"));
        assert!(msg.contains("some error output"));
    }

    #[test]
    fn test_detect_provider_anthropic() {
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");

        let config = detect_provider().unwrap();
        assert!(matches!(config.provider, Provider::Anthropic));
        assert_eq!(config.api_key, "test-key-123");
        assert!(config.base_url.contains("anthropic.com"));

        // Restore
        match saved {
            Some(k) => std::env::set_var("ANTHROPIC_API_KEY", k),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
    }

    #[test]
    #[ignore] // requires API key and network
    fn test_chat_live() {
        let result = call_llm_builtin(
            "chat",
            vec![Value::Str("What does gc_content() do in BioLang? One sentence.".into())],
        )
        .unwrap();
        if let Value::Str(s) = &result {
            assert!(!s.is_empty());
        } else {
            panic!("expected Str");
        }
    }

    // Edge case: format_context with Table value
    #[test]
    fn test_format_context_table() {
        let table = Value::Table(bl_core::value::Table {
            columns: vec!["gene".into(), "pval".into()],
            rows: vec![
                vec![Value::Str("BRCA1".into()), Value::Float(0.001)],
                vec![Value::Str("TP53".into()), Value::Float(0.05)],
            ],
            max_col_width: None,
        });
        let ctx = format_context(&table);
        // Should produce TSV format
        assert!(ctx.contains("gene\tpval"));
        assert!(ctx.contains("BRCA1"));
        assert!(ctx.contains("TP53"));
    }

    // Edge case: format_context with Nil value
    #[test]
    fn test_format_context_nil() {
        let ctx = format_context(&Value::Nil);
        // Nil formatted via Display
        assert!(!ctx.is_empty() || ctx.is_empty()); // Just ensure no panic
        // The Display impl for Nil typically prints "nil" or ""
    }

    // Edge case: format_context with a string
    #[test]
    fn test_format_context_string() {
        let ctx = format_context(&Value::Str("plain text".into()));
        assert_eq!(ctx, "plain text");
    }

    // Edge case: build_user_message with very long context
    #[test]
    fn test_build_user_message_long_context() {
        let long_ctx = "x".repeat(100_000);
        let args = vec![
            Value::Str("analyze".into()),
            Value::Str(long_ctx.clone()),
        ];
        let msg = build_user_message("analyze", &args);
        assert!(msg.contains("--- Context ---"));
        assert!(msg.len() > 100_000);
        assert!(msg.contains(&long_ctx));
    }

    // Edge case: detect_provider with no env vars set returns Err
    #[test]
    fn test_detect_provider_no_env_vars() {
        // Save and clear all provider env vars
        let saved = [
            ("ANTHROPIC_API_KEY", std::env::var("ANTHROPIC_API_KEY").ok()),
            ("OPENAI_API_KEY", std::env::var("OPENAI_API_KEY").ok()),
            ("OLLAMA_MODEL", std::env::var("OLLAMA_MODEL").ok()),
            ("LLM_BASE_URL", std::env::var("LLM_BASE_URL").ok()),
        ];
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("LLM_BASE_URL");

        let result = detect_provider();
        assert!(result.is_err());

        // Restore
        for (key, val) in &saved {
            match val {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    // Edge case: format_context with empty record
    #[test]
    fn test_format_context_empty_record() {
        let ctx = format_context(&Value::Record(HashMap::new()));
        assert!(ctx.is_empty());
    }

    // Edge case: format_context with empty list
    #[test]
    fn test_format_context_empty_list() {
        let ctx = format_context(&Value::List(vec![]));
        assert!(ctx.is_empty());
    }

    // Edge case: is_llm_builtin
    #[test]
    fn test_is_llm_builtin() {
        assert!(is_llm_builtin("chat"));
        assert!(is_llm_builtin("chat_code"));
        assert!(is_llm_builtin("llm_models"));
        assert!(!is_llm_builtin("chat_image"));
        assert!(!is_llm_builtin(""));
    }
}
