use wasm_bindgen::prelude::*;

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::builtins::{set_display_sink, set_output_buffer};
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
    static INTERPRETER: RefCell<Option<Interpreter>> = RefCell::new(Some(browser_interpreter()));
    static OUTPUT_BUF: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

fn browser_interpreter() -> Interpreter {
    let mut interpreter = Interpreter::new();
    interpreter.register_virtual_module(
        "statistics",
        include_str!("../../../packages/statistics/src/mod.bl"),
    );
    interpreter.register_virtual_module(
        "statistics/src/tests",
        include_str!("../../../packages/statistics/src/tests.bl"),
    );
    interpreter.register_virtual_module(
        "statistics/src/correction",
        include_str!("../../../packages/statistics/src/correction.bl"),
    );
    interpreter.register_virtual_module(
        "statistics/src/explore",
        include_str!("../../../packages/statistics/src/explore.bl"),
    );
    interpreter
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

/// Register an optional in-memory package supplied by the embedding page.
///
/// Modules supplied through this function remain outside the BioLang WASM
/// artifact. The default browser interpreter separately embeds the small core
/// statistics modules that are available without registration.
#[wasm_bindgen]
pub fn register_module(path: &str, source: &str) {
    INTERPRETER.with(|cell| {
        let mut slot = cell.borrow_mut();
        let interpreter = slot.get_or_insert_with(browser_interpreter);
        interpreter.register_virtual_module(path, source);
    });
}

/// Take the interpreter out of thread-local storage, creating a fresh one if a
/// previous call panicked and never put it back.
fn take_interpreter() -> Interpreter {
    INTERPRETER
        .with(|c| c.borrow_mut().take())
        .unwrap_or_else(browser_interpreter)
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

    // Values handed to print/println are promoted to typed results. Without
    // this, `println(table)` — the spelling every example uses — produced only
    // ASCII and the Tables view stayed empty.
    let displayed: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let collector = displayed.clone();
    // Every displayed value, with the line that produced it, so the editor can
    // annotate the source as well as fill the Output pane.
    let traced: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let tracer = traced.clone();
    let line_index = LineIndex::new(source);
    set_display_sink(Some(Arc::new(
        move |value: &Value, offset: Option<usize>| {
            if let (Some(offset), Ok(mut trace)) = (offset, tracer.lock()) {
                if trace.len() < MAX_DISPLAYED_RESULTS {
                    trace.push(serde_json::json!({
                        "line": line_index.line_of(offset),
                        "text": preview_of(value),
                    }));
                }
            }
            if let Some(structured) = structured_value(value) {
                if let Ok(mut seen) = collector.lock() {
                    if seen.len() < MAX_DISPLAYED_RESULTS {
                        seen.push(structured);
                    }
                }
            }
        },
    )));

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
                let final_structured = structured_value(&value);
                let mut results = displayed
                    .lock()
                    .map(|seen| seen.clone())
                    .unwrap_or_default();
                // A script that both prints a value and returns it would
                // otherwise report the same table twice.
                if let Some(ref last) = final_structured {
                    if results.last() != Some(last) {
                        results.push(last.clone());
                    }
                }
                serde_json::json!({
                    "ok": true,
                    "value": preview,
                    "type": type_name,
                    "structured": final_structured,
                    "results": results,
                    "trace": traced.lock().map(|trace| trace.clone()).unwrap_or_default(),
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
    set_display_sink(None);
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
            None => *slot = Some(browser_interpreter()),
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
                    "members": value_members(val),
                })
            })
            .collect();
        serde_json::Value::Array(entries).to_string()
    })
}

fn value_members(value: &Value) -> Vec<String> {
    let mut members = match value {
        Value::Record(fields) | Value::Map(fields) => fields.keys().cloned().collect(),
        Value::Table(table) => table.columns.clone(),
        Value::Gene { .. } => vec![
            "symbol",
            "gene_id",
            "chrom",
            "start",
            "end",
            "strand",
            "biotype",
            "description",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        Value::Variant { .. } => vec![
            "chrom",
            "pos",
            "id",
            "ref_allele",
            "alt_allele",
            "quality",
            "filter",
            "info",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        Value::Genome { .. } => vec!["name", "species", "assembly", "chromosomes"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    members.sort();
    members
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
    serde_json::to_string(&bl_import::validate_biolang(source, notebook)).unwrap_or_else(|error| {
        serde_json::json!({ "valid": false, "diagnostics": [{ "message": error.to_string() }] })
            .to_string()
    })
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

fn json_cell(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Int(value) => serde_json::json!(value),
        Value::Float(value) => serde_json::json!(value),
        Value::Str(value) => serde_json::Value::String(value.clone()),
        _ => serde_json::Value::String(format_value(value)),
    }
}

/// Upper bound on typed results collected from print/println in one run, so a
/// loop that prints a table per iteration cannot grow the response unbounded.
const MAX_DISPLAYED_RESULTS: usize = 32;

/// Start offsets of every line, for turning a statement span into a line number.
///
/// Built once per run rather than counting newlines per printed value, which
/// would be quadratic in a script that prints inside a loop.
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0usize];
        starts.extend(
            source
                .char_indices()
                .filter(|(_, character)| *character == '\n')
                .map(|(offset, _)| offset + 1),
        );
        Self { starts }
    }

    /// 1-based line containing `offset`.
    fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }
}

/// A one-line rendering of a value for an inline annotation.
fn preview_of(value: &Value) -> String {
    const MAX_WIDTH: usize = 80;
    let text = match value {
        // A whole SVG document beside the line that drew it is noise; the plot
        // itself is already in the Output pane.
        Value::Str(text) if text.trim_start().starts_with("<svg") => "<plot>".to_string(),
        other => format_value(other),
    };
    let single_line = text.replace(['\n', '\r'], " ");
    if single_line.chars().count() <= MAX_WIDTH {
        return single_line;
    }
    let truncated: String = single_line.chars().take(MAX_WIDTH).collect();
    format!("{truncated}…")
}

fn structured_value(value: &Value) -> Option<serde_json::Value> {
    const MAX_ROWS: usize = 500;
    match value {
        // Plots are SVG strings. Recognising them here matches the CLI event
        // stream and lets `println(plot)` render as a figure rather than
        // dumping the whole document into the text log.
        Value::Str(text) if text.trim_start().starts_with("<svg") => Some(serde_json::json!({
            "kind": "plot",
            "format": "svg",
            "data": text,
        })),
        Value::Table(table) => Some(serde_json::json!({
            "kind": "table",
            "name": "Result table",
            "columns": table.columns,
            "rows": table.rows.iter().take(MAX_ROWS)
                .map(|row| row.iter().map(json_cell).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "totalRows": table.rows.len(),
            "truncated": table.rows.len() > MAX_ROWS,
        })),
        Value::Matrix(matrix) => Some(serde_json::json!({
            "kind": "matrix",
            "name": "Result matrix",
            "columnNames": matrix.col_names.clone().unwrap_or_else(|| {
                (0..matrix.ncol).map(|column| format!("C{}", column + 1)).collect()
            }),
            "rows": (0..matrix.nrow.min(MAX_ROWS))
                .map(|row| matrix.row(row))
                .collect::<Vec<_>>(),
            "totalRows": matrix.nrow,
            "truncated": matrix.nrow > MAX_ROWS,
        })),
        Value::List(items)
            if !items.is_empty() && items.iter().all(|item| matches!(item, Value::Record(_))) =>
        {
            let mut columns = items
                .iter()
                .flat_map(|item| match item {
                    Value::Record(record) => record.keys().cloned().collect::<Vec<_>>(),
                    _ => Vec::new(),
                })
                .collect::<Vec<_>>();
            columns.sort();
            columns.dedup();
            let rows = items
                .iter()
                .take(MAX_ROWS)
                .map(|item| match item {
                    Value::Record(record) => columns
                        .iter()
                        .map(|column| {
                            record
                                .get(column)
                                .map(json_cell)
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                })
                .collect::<Vec<_>>();
            Some(serde_json::json!({
                "kind": "table",
                "name": "Result records",
                "columns": columns,
                "rows": rows,
                "totalRows": items.len(),
                "truncated": items.len() > MAX_ROWS,
            }))
        }
        _ => None,
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

/// Format BioLang source into the canonical layout.
///
/// The browser build has no language server, so without this the web workbench
/// would be the one place `bl fmt` cannot reach — and a formatter people cannot
/// rely on everywhere is one they stop running.
#[wasm_bindgen]
pub fn format(source: &str, indent: usize) -> String {
    let options = bl_fmt::FormatOptions {
        indent_width: indent.clamp(1, 16),
        ..bl_fmt::FormatOptions::default()
    };
    bl_fmt::format_source(source, options)
}

/// Quality metrics for a sequencing file preview, as JSON.
///
/// Returns `null` when the format has no metrics or the sample is unusable.
/// Shared with the Desktop build through `bl-qc` so both report the same
/// numbers rather than two implementations quietly disagreeing.
#[wasm_bindgen]
pub fn qc_metrics(kind: &str, text: &str) -> String {
    match bl_qc::metrics_for(kind, text) {
        Some(metrics) => serde_json::to_string(&metrics).unwrap_or_else(|_| "null".into()),
        None => "null".into(),
    }
}
