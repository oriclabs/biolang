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
    interpreter.register_virtual_module(
        "statistics/src/tasks",
        include_str!("../../../packages/statistics/src/tasks.bl"),
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

/// Version of the Rust runtime compiled into this WebAssembly module.
///
/// JavaScript packages must use this value for compatibility checks rather
/// than reporting their independently versioned package.json version.
#[wasm_bindgen]
pub fn runtime_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
        let mut entries: Vec<serde_json::Value> = vars
            .into_iter()
            .filter(|(name, v)| {
                *name != "_"
                    && !name.starts_with("__const_")
                    && !matches!(v, Value::NativeFunction { .. })
            })
            .map(|(name, val)| {
                let (length, rows, columns) = value_shape(val);
                let type_name = val.type_of().to_string();
                let (members, members_truncated) = value_members(val);
                serde_json::json!({
                    "name": name,
                    // `type` remains for older browser-console consumers.
                    "type": type_name,
                    "typeName": type_name,
                    "preview": value_preview(val),
                    "sizeBytes": approximate_value_bytes(val),
                    "sizeApproximate": value_size_is_approximate(val),
                    "length": length,
                    "rows": rows,
                    "columns": columns,
                    "members": members,
                    "membersTruncated": members_truncated,
                })
            })
            .collect();
        entries.sort_by(|left, right| {
            left.get("name")
                .and_then(serde_json::Value::as_str)
                .cmp(&right.get("name").and_then(serde_json::Value::as_str))
        });
        serde_json::Value::Array(entries).to_string()
    })
}

/// Return one bounded page of a variable. Container values are never formatted
/// wholesale: at most 100 rows and 50 columns cross the WASM boundary per call.
#[wasm_bindgen]
pub fn inspect_variable(name: &str, offset: usize, limit: usize) -> String {
    const MAX_ROWS: usize = 100;
    INTERPRETER.with(|cell| {
        let slot = cell.borrow();
        let Some(interpreter) = slot.as_ref() else {
            return serde_json::json!({ "ok": false, "error": "The browser interpreter is not initialized." }).to_string();
        };
        let Some(value) = interpreter.env().lookup(name) else {
            return serde_json::json!({ "ok": false, "error": format!("Variable '{name}' no longer exists.") }).to_string();
        };
        let page = variable_page(name, value, offset, limit.clamp(1, MAX_ROWS));
        serde_json::json!({ "ok": true, "page": page }).to_string()
    })
}

/// Serialize one variable exactly, stopping before the response can exceed the
/// caller's byte cap. Large native exports use the streaming path in
/// `bl_runtime::value_export` instead of crossing this in-memory boundary.
#[wasm_bindgen]
pub fn export_variable(name: &str, format: &str, maximum_bytes: usize) -> Result<Vec<u8>, JsValue> {
    let format = bl_runtime::value_export::ValueExportFormat::parse(format)
        .map_err(|error| JsValue::from_str(&error))?;
    INTERPRETER.with(|cell| {
        let slot = cell.borrow();
        let interpreter = slot
            .as_ref()
            .ok_or_else(|| JsValue::from_str("The browser interpreter is not initialized."))?;
        let value = interpreter
            .env()
            .lookup(name)
            .ok_or_else(|| JsValue::from_str(&format!("Variable '{name}' no longer exists.")))?;
        bl_runtime::value_export::export_value_capped(value, format, maximum_bytes)
            .map_err(|error| JsValue::from_str(&error))
    })
}

fn variable_page(name: &str, value: &Value, offset: usize, limit: usize) -> serde_json::Value {
    const MAX_COLUMNS: usize = 50;
    let type_name = value.type_of().to_string();
    match value {
        Value::Table(table) => {
            let end = offset.saturating_add(limit).min(table.rows.len());
            let shown_columns = table.columns.len().min(MAX_COLUMNS);
            let mut columns = vec!["#".to_string()];
            columns.extend(table.columns.iter().take(shown_columns).cloned());
            let rows = table
                .rows
                .get(offset..end)
                .unwrap_or(&[])
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let mut cells = vec![serde_json::json!(offset + index + 1)];
                    cells.extend(row.iter().take(shown_columns).map(variable_cell));
                    cells
                })
                .collect::<Vec<_>>();
            page_json(
                name,
                &type_name,
                "table",
                offset,
                table.rows.len(),
                columns,
                rows,
                table.columns.len() > shown_columns,
            )
        }
        Value::Matrix(matrix) => matrix_page(
            name,
            &type_name,
            "matrix",
            offset,
            limit,
            matrix.nrow,
            matrix.ncol,
            matrix.row_names.as_ref(),
            matrix.col_names.as_ref(),
            |row, column| serde_json::json!(matrix.get(row, column)),
        ),
        Value::SparseMatrix(matrix) => matrix_page(
            name,
            &type_name,
            "sparse-matrix",
            offset,
            limit,
            matrix.nrow,
            matrix.ncol,
            matrix.row_names.as_ref(),
            matrix.col_names.as_ref(),
            |row, column| serde_json::json!(matrix.get(row, column)),
        ),
        Value::List(items) => collection_page(name, &type_name, items, offset, limit),
        Value::Set(items) | Value::Tuple(items) => {
            collection_page(name, &type_name, items, offset, limit)
        }
        Value::Map(fields) | Value::Record(fields) => {
            // Hash-map iteration is stable while the value is unchanged. Avoid
            // allocating and sorting every key merely to show one small page.
            let rows = fields
                .iter()
                .skip(offset)
                .take(limit)
                .map(|(key, item)| {
                    vec![
                        serde_json::json!(key),
                        serde_json::json!(item.type_of().to_string()),
                        variable_cell(item),
                    ]
                })
                .collect::<Vec<_>>();
            page_json(
                name,
                &type_name,
                "record",
                offset,
                fields.len(),
                vec!["Field".into(), "Type".into(), "Value".into()],
                rows,
                false,
            )
        }
        Value::Str(text) => text_page(name, &type_name, "text", text, offset, limit, 240),
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => text_page(
            name,
            &type_name,
            "sequence",
            &sequence.data,
            offset,
            limit,
            120,
        ),
        Value::Quality(scores) => {
            let end = offset.saturating_add(limit).min(scores.len());
            let rows = scores
                .get(offset..end)
                .unwrap_or(&[])
                .iter()
                .enumerate()
                .map(|(index, score)| {
                    vec![
                        serde_json::json!(offset + index + 1),
                        serde_json::json!(score),
                    ]
                })
                .collect();
            page_json(
                name,
                &type_name,
                "quality",
                offset,
                scores.len(),
                vec!["Position".into(), "Score".into()],
                rows,
                false,
            )
        }
        _ => page_json(
            name,
            &type_name,
            "scalar",
            0,
            1,
            vec!["Value".into()],
            vec![vec![variable_cell(value)]],
            false,
        ),
    }
}

fn collection_page(
    name: &str,
    type_name: &str,
    items: &[Value],
    offset: usize,
    limit: usize,
) -> serde_json::Value {
    let end = offset.saturating_add(limit).min(items.len());
    let rows = items
        .get(offset..end)
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(index, item)| {
            vec![
                serde_json::json!(offset + index + 1),
                serde_json::json!(item.type_of().to_string()),
                variable_cell(item),
            ]
        })
        .collect::<Vec<_>>();
    page_json(
        name,
        type_name,
        "collection",
        offset,
        items.len(),
        vec!["#".into(), "Type".into(), "Value".into()],
        rows,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn matrix_page(
    name: &str,
    type_name: &str,
    kind: &str,
    offset: usize,
    limit: usize,
    nrow: usize,
    ncol: usize,
    row_names: Option<&Vec<String>>,
    column_names: Option<&Vec<String>>,
    value_at: impl Fn(usize, usize) -> serde_json::Value,
) -> serde_json::Value {
    const MAX_COLUMNS: usize = 50;
    let shown_columns = ncol.min(MAX_COLUMNS);
    let end = offset.saturating_add(limit).min(nrow);
    let mut columns = vec!["Row".to_string()];
    columns.extend((0..shown_columns).map(|column| {
        column_names
            .and_then(|names| names.get(column))
            .cloned()
            .unwrap_or_else(|| (column + 1).to_string())
    }));
    let rows = (offset..end)
        .map(|row| {
            let mut cells = vec![serde_json::json!(row_names
                .and_then(|names| names.get(row))
                .cloned()
                .unwrap_or_else(|| (row + 1).to_string()))];
            cells.extend((0..shown_columns).map(|column| value_at(row, column)));
            cells
        })
        .collect();
    page_json(
        name,
        type_name,
        kind,
        offset,
        nrow,
        columns,
        rows,
        ncol > shown_columns,
    )
}

fn text_page(
    name: &str,
    type_name: &str,
    kind: &str,
    text: &str,
    offset: usize,
    limit: usize,
    chunk: usize,
) -> serde_json::Value {
    let character_count = text.chars().count();
    let total = character_count.div_ceil(chunk);
    let end = offset.saturating_add(limit).min(total);
    let mut characters = text.chars().skip(offset.saturating_mul(chunk));
    let rows = (offset..end)
        .map(|index| {
            vec![
                serde_json::json!(index + 1),
                serde_json::json!(characters.by_ref().take(chunk).collect::<String>()),
            ]
        })
        .collect();
    page_json(
        name,
        type_name,
        kind,
        offset,
        total,
        vec!["Chunk".into(), "Value".into()],
        rows,
        false,
    )
}

fn page_json(
    name: &str,
    type_name: &str,
    kind: &str,
    offset: usize,
    total: usize,
    columns: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
    columns_truncated: bool,
) -> serde_json::Value {
    let next_offset = offset.saturating_add(rows.len());
    serde_json::json!({
        "name": name, "typeName": type_name, "kind": kind, "offset": offset,
        "nextOffset": next_offset, "total": total, "columns": columns, "rows": rows,
        "truncated": next_offset < total, "columnsTruncated": columns_truncated,
    })
}

fn variable_cell(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Int(value) => serde_json::json!(value),
        Value::Float(value) if value.is_finite() => serde_json::json!(value),
        Value::Float(value) => serde_json::json!(value.to_string()),
        Value::Str(value) => serde_json::json!(truncate_chars(value, 240)),
        _ => serde_json::json!(value_preview(value)),
    }
}

fn truncate_chars(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        return value.to_string();
    }
    let mut shortened = value
        .chars()
        .take(maximum.saturating_sub(1))
        .collect::<String>();
    shortened.push('…');
    shortened
}

fn value_preview(value: &Value) -> String {
    match value {
        Value::Nil => "nil".into(),
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Str(value) => format!("\"{}\"", truncate_chars(value, 60)),
        Value::List(values) => format!("[{} items]", values.len()),
        Value::Map(values) => format!("{{{} entries}}", values.len()),
        Value::Record(values) => format!("{{{} fields}}", values.len()),
        Value::Table(table) => format!("[{} × {}]", table.num_rows(), table.num_cols()),
        Value::Matrix(matrix) => format!("Matrix({} × {})", matrix.nrow, matrix.ncol),
        Value::SparseMatrix(matrix) => format!(
            "Sparse({} × {}, {} non-zero)",
            matrix.nrow,
            matrix.ncol,
            matrix.nnz()
        ),
        Value::DNA(sequence) => format!("{} bp", sequence.data.len()),
        Value::RNA(sequence) => format!("{} nt", sequence.data.len()),
        Value::Protein(sequence) => format!("{} aa", sequence.data.len()),
        Value::Quality(values) => format!("Quality({} scores)", values.len()),
        Value::Set(values) => format!("Set({} items)", values.len()),
        Value::Tuple(values) => format!("Tuple({} items)", values.len()),
        Value::Function { params, .. } => format!("fn({} parameters)", params.len()),
        Value::NativeFunction { name, .. } => format!("<builtin {name}>"),
        Value::Formula(_) => "~expression".into(),
        Value::Stream(stream) => format!("Stream({})", truncate_chars(&stream.label, 60)),
        Value::Interval(interval) => truncate_chars(&interval.to_string(), 80),
        Value::Range {
            start,
            end,
            inclusive,
        } => {
            if *inclusive {
                format!("{start}..={end}")
            } else {
                format!("{start}..{end}")
            }
        }
        Value::EnumValue {
            enum_name,
            variant,
            fields,
            ..
        } => format!("{enum_name}::{variant}({} fields)", fields.len()),
        Value::PluginFunction {
            plugin_name,
            operation,
            ..
        } => format!("<plugin:{plugin_name}.{operation}>"),
        Value::Regex { pattern, flags } => format!("/{}/{flags}", truncate_chars(pattern, 60)),
        Value::Future(_) => "<future>".into(),
        Value::Kmer(kmer) => format!("Kmer({})", kmer.decode()),
        Value::CompiledClosure(_) => "<compiled function>".into(),
        Value::Gene { symbol, .. } => format!("Gene({})", truncate_chars(symbol, 60)),
        Value::Variant { chrom, pos, .. } => {
            format!("Variant({}:{pos})", truncate_chars(chrom, 50))
        }
        Value::Genome { name, .. } => format!("Genome({})", truncate_chars(name, 60)),
        Value::AlignedRead(read) => format!(
            "AlignedRead({} {}:{})",
            truncate_chars(&read.qname, 30),
            truncate_chars(&read.rname, 30),
            read.pos
        ),
    }
}

fn value_shape(value: &Value) -> (Option<usize>, Option<usize>, Option<usize>) {
    match value {
        Value::List(values) => (Some(values.len()), None, None),
        Value::Set(values) | Value::Tuple(values) => (Some(values.len()), None, None),
        Value::Map(values) | Value::Record(values) => (Some(values.len()), None, None),
        Value::Table(table) => (None, Some(table.num_rows()), Some(table.num_cols())),
        Value::Matrix(matrix) => (None, Some(matrix.nrow), Some(matrix.ncol)),
        Value::SparseMatrix(matrix) => (None, Some(matrix.nrow), Some(matrix.ncol)),
        Value::Str(value) => (Some(value.chars().count()), None, None),
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            (Some(sequence.data.len()), None, None)
        }
        Value::Quality(values) => (Some(values.len()), None, None),
        _ => (None, None, None),
    }
}

fn value_size_is_approximate(value: &Value) -> bool {
    matches!(
        value,
        Value::List(_)
            | Value::Map(_)
            | Value::Record(_)
            | Value::Table(_)
            | Value::Set(_)
            | Value::Tuple(_)
    )
}

fn approximate_value_bytes(value: &Value) -> usize {
    const SAMPLE: usize = 32;
    let base = std::mem::size_of::<Value>();
    let extrapolate = |sample: usize, measured: usize, total: usize| {
        if sample == 0 {
            base
        } else {
            base.saturating_add(measured.saturating_mul(total).div_ceil(sample))
        }
    };
    match value {
        Value::Str(value) => base + value.len(),
        Value::List(values) => {
            let sample = values.len().min(SAMPLE);
            extrapolate(
                sample,
                values
                    .iter()
                    .take(sample)
                    .map(approximate_value_bytes)
                    .sum(),
                values.len(),
            )
        }
        Value::Map(values) | Value::Record(values) => {
            let sample = values.len().min(SAMPLE);
            extrapolate(
                sample,
                values
                    .iter()
                    .take(sample)
                    .map(|(key, value)| key.len() + approximate_value_bytes(value))
                    .sum(),
                values.len(),
            )
        }
        Value::Table(table) => {
            let sample = table.rows.len().min(SAMPLE);
            let measured = table
                .rows
                .iter()
                .take(sample)
                .flatten()
                .map(approximate_value_bytes)
                .sum();
            table
                .columns
                .iter()
                .map(String::len)
                .sum::<usize>()
                .saturating_add(extrapolate(sample, measured, table.rows.len()))
        }
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            base + sequence.data.len()
        }
        Value::Matrix(matrix) => base + matrix.data.len() * std::mem::size_of::<f64>(),
        Value::SparseMatrix(matrix) => {
            base + matrix.data.len() * std::mem::size_of::<f64>()
                + matrix.indices.len() * std::mem::size_of::<usize>()
                + matrix.indptr.len() * std::mem::size_of::<usize>()
        }
        Value::Set(values) | Value::Tuple(values) => {
            let sample = values.len().min(SAMPLE);
            extrapolate(
                sample,
                values
                    .iter()
                    .take(sample)
                    .map(approximate_value_bytes)
                    .sum(),
                values.len(),
            )
        }
        Value::Quality(values) => base + values.len(),
        _ => base,
    }
}

fn value_members(value: &Value) -> (Vec<String>, bool) {
    const MAX_MEMBERS: usize = 50;
    let (mut members, total) = match value {
        Value::Record(fields) | Value::Map(fields) => (
            fields.keys().take(MAX_MEMBERS).cloned().collect(),
            fields.len(),
        ),
        Value::Table(table) => (
            table.columns.iter().take(MAX_MEMBERS).cloned().collect(),
            table.columns.len(),
        ),
        Value::Gene { .. } => (
            vec![
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
            8,
        ),
        Value::Variant { .. } => (
            vec![
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
            8,
        ),
        Value::Genome { .. } => (
            vec!["name", "species", "assembly", "chromosomes"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            4,
        ),
        _ => (Vec::new(), 0),
    };
    members.sort();
    (members, total > MAX_MEMBERS)
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

/// List all builtin functions available in this WASM build.
/// Returns a JSON array of `{name, arity}` records.
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
        Value::Record(record) => Some(serde_json::json!({
            "kind": "record",
            "name": "Result record",
            "value": record.iter()
                .map(|(key, value)| (key.clone(), json_cell(value)))
                .collect::<serde_json::Map<_, _>>(),
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
        Value::List(items) => Some(serde_json::json!({
            "kind": "table",
            "name": "Result list",
            "columns": ["Value"],
            "rows": items.iter().take(MAX_ROWS)
                .map(|value| vec![json_cell(value)])
                .collect::<Vec<_>>(),
            "totalRows": items.len(),
            "truncated": items.len() > MAX_ROWS,
        })),
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

#[cfg(test)]
mod variable_inspector_tests {
    use super::*;
    use bl_core::value::Table;

    #[test]
    fn table_pages_are_bounded_and_report_the_next_offset() {
        let table = Value::Table(Table::new(
            vec!["value".into()],
            (0..250).map(|value| vec![Value::Int(value)]).collect(),
        ));
        let page = variable_page("observations", &table, 40, 20);
        assert_eq!(page["rows"].as_array().unwrap().len(), 20);
        assert_eq!(page["nextOffset"], 60);
        assert_eq!(page["total"], 250);
        assert_eq!(page["truncated"], true);
    }

    #[test]
    fn wide_matrices_never_cross_more_than_fifty_columns() {
        let matrix = bl_core::matrix::Matrix::zeros(2, 75);
        let page = variable_page("wide", &Value::Matrix(matrix.into()), 0, 20);
        // One row-label column plus fifty data columns.
        assert_eq!(page["columns"].as_array().unwrap().len(), 51);
        assert_eq!(page["columnsTruncated"], true);
    }

    #[test]
    fn collection_summary_does_not_format_all_items() {
        let items = Value::List((0..10_000).map(Value::Int).collect::<Vec<_>>().into());
        assert_eq!(value_preview(&items), "[10000 items]");
        assert!(value_size_is_approximate(&items));
        assert!(approximate_value_bytes(&items) > 10_000);
    }

    #[test]
    fn member_summaries_and_text_pages_stay_bounded() {
        let table = Value::Table(Table::new(
            (0..75).map(|index| format!("column_{index}")).collect(),
            Vec::new(),
        ));
        let (members, truncated) = value_members(&table);
        assert_eq!(members.len(), 50);
        assert!(truncated);

        let text = "A".repeat(1_000_000);
        let page = variable_page("sequence", &Value::Str(text), 0, 2);
        assert_eq!(page["rows"].as_array().unwrap().len(), 2);
        assert_eq!(page["rows"][0][1].as_str().unwrap().len(), 240);
    }
}
