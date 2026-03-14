use wasm_bindgen::prelude::*;

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;
use bl_runtime::builtins::set_output_buffer;
use bl_runtime::csv::set_fetch_hook;
use bl_bio::io::set_bio_fetch_hook;

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
    static OUTPUT_BUF: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

// JavaScript binding for synchronous XHR fetch
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__blFetch"], js_name = "sync")]
    fn js_fetch_sync(url: &str) -> JsValue;
}

/// Bridge JS __blFetch.sync to a Rust closure for CSV and bio I/O.
fn js_fetch_closure(url: &str) -> std::result::Result<String, String> {
    let result = js_fetch_sync(url);
    if result.is_null() || result.is_undefined() {
        return Err(format!("fetch failed for '{url}'"));
    }
    if let Some(text) = result.as_string() {
        if text.starts_with("ERROR:") {
            Err(text[6..].to_string())
        } else {
            Ok(text)
        }
    } else {
        Err("fetch returned non-string".into())
    }
}

/// Set up fetch hooks so read_csv/read_fasta/read_fastq/read_vcf/read_bed/read_gff
/// can access local files and URLs in WASM via the JS __blFetch bridge.
fn install_fetch_hooks() {
    let hook: Arc<dyn Fn(&str) -> std::result::Result<String, String>> =
        Arc::new(js_fetch_closure);
    set_fetch_hook(Some(hook.clone()));
    set_bio_fetch_hook(Some(hook));
}

/// Initialize the WASM module (set panic hook for better error messages).
#[wasm_bindgen]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // Install fetch hooks for CSV and bio I/O (FASTA, FASTQ, VCF, BED, GFF)
    install_fetch_hooks();
}

/// Evaluate BioLang source code. Returns JSON: `{ok, value, type, output, error}`
#[wasm_bindgen]
pub fn evaluate(source: &str) -> String {
    // Set up output capture
    let buf = OUTPUT_BUF.with(|b| {
        if let Ok(mut s) = b.lock() {
            s.clear();
        }
        b.clone()
    });
    set_output_buffer(Some(buf.clone()));

    let result = INTERPRETER.with(|interp| {
        let mut interp = interp.borrow_mut();

        // Lex
        let tokens = match Lexer::new(source).tokenize() {
            Ok(t) => t,
            Err(e) => {
                return serde_json::json!({
                    "ok": false,
                    "error": e.message,
                    "output": drain_output(&buf),
                });
            }
        };

        // Parse
        let parse_result = match Parser::new(tokens).parse() {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "ok": false,
                    "error": e.message,
                    "output": drain_output(&buf),
                });
            }
        };

        if parse_result.has_errors() {
            let msg = parse_result.errors.iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return serde_json::json!({
                "ok": false,
                "error": msg,
                "output": drain_output(&buf),
            });
        }

        // Execute
        match interp.run(&parse_result.program) {
            Ok(value) => {
                let type_name = value.type_of().to_string();
                let preview = format_value(&value);
                serde_json::json!({
                    "ok": true,
                    "value": preview,
                    "type": type_name,
                    "output": drain_output(&buf),
                })
            }
            Err(e) => {
                serde_json::json!({
                    "ok": false,
                    "error": e.message,
                    "output": drain_output(&buf),
                })
            }
        }
    });

    set_output_buffer(None);
    result.to_string()
}

/// Reset the interpreter state.
#[wasm_bindgen]
pub fn reset() {
    INTERPRETER.with(|interp| {
        interp.borrow_mut().reset();
    });
}

/// List all variables in the current environment. Returns JSON array.
#[wasm_bindgen]
pub fn list_variables() -> String {
    INTERPRETER.with(|interp| {
        let interp = interp.borrow();
        let vars = interp.env().list_global_vars();
        let entries: Vec<serde_json::Value> = vars
            .into_iter()
            .filter(|(_, v)| !matches!(v, Value::NativeFunction { .. }))
            .map(|(name, val)| {
                serde_json::json!({
                    "name": name,
                    "type": val.type_of().to_string(),
                    "preview": format_value(val),
                })
            })
            .collect();
        serde_json::Value::Array(entries).to_string()
    })
}

/// Tokenize source code for syntax highlighting. Returns JSON array of token spans.
#[wasm_bindgen]
pub fn tokenize(source: &str) -> String {
    match Lexer::new(source).tokenize() {
        Ok(tokens) => {
            let spans: Vec<serde_json::Value> = tokens
                .iter()
                .map(|tok| {
                    serde_json::json!({
                        "kind": token_kind_class(&tok.kind),
                        "start": tok.span.start,
                        "end": tok.span.end,
                    })
                })
                .collect();
            serde_json::Value::Array(spans).to_string()
        }
        Err(_) => "[]".to_string(),
    }
}

/// List all builtin functions. Returns JSON array of {name, signature, category}.
#[wasm_bindgen]
pub fn list_builtins() -> String {
    // Return the full catalog from the REPL catalog constants embedded here
    INTERPRETER.with(|interp| {
        let interp = interp.borrow();
        let vars = interp.env().list_global_vars();
        let builtins: Vec<serde_json::Value> = vars
            .into_iter()
            .filter_map(|(name, val)| {
                if let Value::NativeFunction { arity, .. } = val {
                    Some(serde_json::json!({
                        "name": name,
                        "arity": format!("{:?}", arity),
                    }))
                } else {
                    None
                }
            })
            .collect();
        serde_json::Value::Array(builtins).to_string()
    })
}

fn drain_output(buf: &Arc<Mutex<String>>) -> String {
    if let Ok(mut s) = buf.lock() {
        let out = s.clone();
        s.clear();
        out
    } else {
        String::new()
    }
}

fn format_value(val: &Value) -> String {
    match val {
        Value::Nil => "nil".into(),
        Value::Bool(b) => format!("{b}"),
        Value::Int(n) => format!("{n}"),
        Value::Float(f) => format!("{f}"),
        Value::Str(s) => format!("\"{s}\""),
        _ => format!("{val}"),
    }
}

fn token_kind_class(kind: &bl_lexer::TokenKind) -> &'static str {
    use bl_lexer::TokenKind::*;
    match kind {
        Int(_) | Float(_) => "number",
        Str(_) | FStr(_) => "string",
        DnaLit(_) | RnaLit(_) | ProteinLit(_) | QualLit(_) => "bio",
        Ident(_) => "ident",
        Let | Fn | If | Else | For | In | While | Break | Continue | Match | Return
        | Assert | Try | Catch | Pipeline | Import | Yield | Enum
        | Struct | Async | Await | Trait | Impl | Const | With | Then | Unless
        | Guard | Do | End | When | Defer | As | Stage | Parallel | Not | From
        | Given | Otherwise | Retry | Where | Into => "keyword",
        True | False | Nil => "literal",
        PipeOp | TapPipe => "pipe",
        Plus | Minus | Star | StarStar | Slash | Percent | PlusEq | MinusEq | StarEq | SlashEq
        | QuestionQuestion | QuestionDot | QuestionEq | EqEq | Neq | Lt | Gt | Le | Ge
        | And | Or | Bang | Eq | Tilde | Dot | Arrow | FatArrow
        | DotDot | DotDotEq | DotDotDot | At | Amp | Caret | Shl | Shr => "operator",
        RegexLit(_, _) => "string",
        LParen | RParen | LBrace | RBrace | LBracket | RBracket | Bar | HashLBrace => "delimiter",
        Colon | Comma => "punctuation",
        DocComment(_) => "comment",
        Newline | Eof => "whitespace",
    }
}
