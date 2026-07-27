use wasm_bindgen::prelude::*;

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::builtins::set_output_buffer;
use bl_runtime::csv::set_fetch_hook;
use bl_runtime::Interpreter;

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    /// `Option` so `evaluate` can TAKE the interpreter out, run user code while
    /// holding no borrow, and put it back. wasm32 cannot unwind, so a panic
    /// inside user code would otherwise leak the `RefMut` guard forever and
    /// every later call would fail with "RefCell already borrowed" — one bad
    /// example on a docs page broke every Run button after it until reload.
    /// With the value taken out, a panic just leaves `None` and the next call
    /// starts from a fresh interpreter.
    static INTERPRETER: RefCell<Option<Interpreter>> = RefCell::new(Some(Interpreter::new()));
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
    let hook: Arc<dyn Fn(&str) -> std::result::Result<String, String>> = Arc::new(js_fetch_closure);
    set_fetch_hook(Some(hook));
}

/// Initialize the WASM module (set panic hook for better error messages).
#[wasm_bindgen]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // Install fetch hooks for CSV and bio I/O (FASTA, FASTQ, VCF, BED, GFF)
    install_fetch_hooks();
}

/// Take the interpreter out of thread-local storage, creating a fresh one if a
/// previous call panicked and never put it back.
fn take_interpreter() -> Interpreter {
    INTERPRETER
        .with(|c| c.borrow_mut().take())
        .unwrap_or_else(Interpreter::new)
}

fn put_interpreter(interp: Interpreter) {
    INTERPRETER.with(|c| *c.borrow_mut() = Some(interp));
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

    // Held as a plain local, NOT a RefCell borrow, so a panic below cannot
    // poison the module for every subsequent call.
    let mut owned = take_interpreter();
    let result = (|interp: &mut Interpreter| -> serde_json::Value {
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
            let msg = parse_result
                .errors
                .iter()
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
    })(&mut owned);

    put_interpreter(owned);
    set_output_buffer(None);
    result.to_string()
}

/// Reset the interpreter state.
#[wasm_bindgen]
pub fn reset() {
    // Always install a working interpreter, even if a previous panic left None.
    INTERPRETER.with(|c| {
        let mut slot = c.borrow_mut();
        match slot.as_mut() {
            Some(i) => i.reset(),
            None => *slot = Some(Interpreter::new()),
        }
    });
}

/// List all variables in the current environment. Returns JSON array.
#[wasm_bindgen]
pub fn list_variables() -> String {
    INTERPRETER.with(|c| {
        let slot = c.borrow();
        let Some(interp) = slot.as_ref() else {
            return "[]".to_string();
        };
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

/// Convert Python, R, Jupyter, or R Markdown and return a structured validation result.
#[wasm_bindgen]
pub fn import_source(source: &str, format: &str, filename: &str) -> String {
    match bl_import::import_source(source, format, filename) {
        Ok(result) => serde_json::json!({ "ok": true, "result": result }).to_string(),
        Err(error) => serde_json::json!({ "ok": false, "error": error }).to_string(),
    }
}

/// Validate a BioLang script or BioLang notebook without executing it.
#[wasm_bindgen]
pub fn validate_import(source: &str, notebook: bool) -> String {
    serde_json::to_string(&bl_import::validate_biolang(source, notebook))
        .unwrap_or_else(|error| serde_json::json!({ "valid": false, "diagnostics": [{ "message": error.to_string() }] }).to_string())
}

/// List all builtin functions. Returns JSON array of {name, signature, category}.
#[wasm_bindgen]
pub fn list_builtins() -> String {
    // Return the full catalog from the REPL catalog constants embedded here
    INTERPRETER.with(|c| {
        let slot = c.borrow();
        let Some(interp) = slot.as_ref() else {
            return "[]".to_string();
        };
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
        Let | Fn | If | Else | For | In | While | Break | Continue | Match | Return | Assert
        | Try | Catch | Pipeline | Import | Yield | Enum | Struct | Async | Await | Trait
        | Impl | Const | With | Then | Unless | Guard | Do | End | When | Defer | As | Stage
        | Parallel | Not | From | Given | Otherwise | Retry | Where | Into => "keyword",
        True | False | Nil => "literal",
        PipeOp | TapPipe => "pipe",
        Plus | PlusPlus | Minus | Star | StarStar | Slash | Percent | PlusEq | MinusEq | StarEq
        | SlashEq | QuestionQuestion | QuestionDot | QuestionEq | EqEq | Neq | Lt | Gt | Le
        | Ge | And | Or | Bang | Eq | Tilde | Dot | Arrow | FatArrow | DotDot | DotDotEq
        | DotDotDot | At | Amp | Caret | Shl | Shr => "operator",
        RegexLit(_, _) => "string",
        LParen | RParen | LBrace | RBrace | LBracket | RBracket | Bar | HashLBrace => "delimiter",
        Colon | Comma => "punctuation",
        DocComment(_) => "comment",
        Newline | Eof => "whitespace",
    }
}
