//! BioLang notebook (.bln and .bl.md) literate format.
//!
//! A `.bln` file interleaves Markdown prose with BioLang code blocks.
//!
//! Code block syntaxes:
//! - Fenced: ` ```biolang ... ``` ` or ` ```bl ... ``` ` or bare ` ``` ... ``` `
//! - Legacy: lines between `---` separators
//!
//! Cell directives (comments at top of code block):
//! - `# @hide` / `# @hide-code` -- execute but don't display code
//! - `# @skip` -- don't execute
//! - `# @echo` -- print code before executing
//! - `# @hide-output` -- execute but suppress printed output
//! - `# @chat` -- send cell content to LLM via chat() builtin instead of executing
//!
//! Optional front matter (for `--export typst`/`pdf` research papers): a leading
//! `---` fenced block of `key: value` lines at the very top of the file. Keys:
//! `title`, `authors` (comma-separated), `date`, `abstract`, `bibliography`.
//! Prose may cite with `@key` (rendered as a Typst citation).
//!
//! Export formats: `--export html`, `--export typst` (`.typ` + SVG figures),
//! `--export pdf` (compiles the Typst via the `typst` binary / `TYPST_BIN`).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum CellDirective {
    Hide,
    Skip,
    Echo,
    HideOutput,
    Chat, // Send cell content to LLM via chat() builtin
}

#[derive(Debug, Clone)]
struct CodeBlock {
    code: String,
    directives: Vec<CellDirective>,
}

#[derive(Debug)]
enum Block {
    Prose(String),
    Code(CodeBlock),
}

struct ExecutedBlock {
    block: Block,
    output: Option<String>,
}

// ── Parser ───────────────────────────────────────────────────────────────────

fn is_biolang_fence(line: &str) -> bool {
    let t = line.trim();
    t == "```" || t == "```biolang" || t == "```bl"
}

fn is_fence_close(line: &str) -> bool {
    line.trim() == "```"
}

fn is_other_fence_open(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("```") && !is_biolang_fence(line)
}

fn parse_directives(raw: &str) -> (Vec<CellDirective>, String) {
    let mut directives = Vec::new();
    let mut remaining_lines = Vec::new();
    let mut still_scanning = true;

    for line in raw.lines() {
        if still_scanning {
            let t = line.trim();
            if t == "# @hide" || t == "# @hide-code" {
                directives.push(CellDirective::Hide);
                continue;
            } else if t == "# @skip" {
                directives.push(CellDirective::Skip);
                continue;
            } else if t == "# @echo" {
                directives.push(CellDirective::Echo);
                continue;
            } else if t == "# @hide-output" {
                directives.push(CellDirective::HideOutput);
                continue;
            } else if t == "# @chat" {
                directives.push(CellDirective::Chat);
                continue;
            }
            still_scanning = false;
        }
        remaining_lines.push(line);
    }

    (directives, remaining_lines.join("\n"))
}

fn flush_block(blocks: &mut Vec<Block>, text: &str, is_code: bool) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    if is_code {
        let (directives, code) = parse_directives(trimmed);
        blocks.push(Block::Code(CodeBlock { code, directives }));
    } else {
        blocks.push(Block::Prose(text.to_string()));
    }
}

fn parse_notebook(source: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut in_dash_code = false;
    let mut in_fenced_code = false;
    let mut in_other_fence = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Inside a non-BioLang fence (e.g. ```python) — treat as prose
        if in_other_fence {
            if is_fence_close(line) {
                in_other_fence = false;
            }
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            continue;
        }

        // Inside a fenced code block
        if in_fenced_code {
            if is_fence_close(line) {
                flush_block(&mut blocks, &std::mem::take(&mut current), true);
                in_fenced_code = false;
            } else {
                if !current.is_empty() {
                    current.push('\n');
                }
                current.push_str(line);
            }
            continue;
        }

        // Opening a fenced block?
        if !in_dash_code && is_biolang_fence(line) {
            flush_block(&mut blocks, &std::mem::take(&mut current), false);
            in_fenced_code = true;
            continue;
        }

        // Opening a non-BioLang fence?
        if !in_dash_code && is_other_fence_open(line) {
            in_other_fence = true;
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            continue;
        }

        // Legacy --- delimiter
        if trimmed == "---" {
            flush_block(&mut blocks, &std::mem::take(&mut current), in_dash_code);
            in_dash_code = !in_dash_code;
            continue;
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    // Flush remaining
    flush_block(&mut blocks, &current, in_dash_code || in_fenced_code);
    blocks
}

// ── Execution helper ─────────────────────────────────────────────────────────

fn execute_notebook(path: &str) -> Vec<ExecutedBlock> {
    let source = read_file(path);
    // Strip YAML-style front matter so it is never executed or rendered as prose.
    let (_front, body) = split_front_matter(&source);
    let blocks = parse_notebook(&body);
    let mut interpreter = bl_runtime::Interpreter::new();

    if let Ok(canonical) = std::fs::canonicalize(path) {
        interpreter.set_current_file(Some(canonical));
    } else {
        interpreter.set_current_file(Some(PathBuf::from(path)));
    }

    let mut results = Vec::new();

    for block in blocks {
        match block {
            Block::Prose(text) => {
                results.push(ExecutedBlock {
                    block: Block::Prose(text),
                    output: None,
                });
            }
            Block::Code(ref cb) => {
                if cb.directives.contains(&CellDirective::Skip) {
                    results.push(ExecutedBlock {
                        block,
                        output: None,
                    });
                    continue;
                }

                // @chat cells: send text to LLM instead of interpreting as code
                if cb.directives.contains(&CellDirective::Chat) {
                    let prompt = cb.code.trim().to_string();
                    let output = match bl_runtime::llm::call_llm_builtin(
                        "chat",
                        vec![bl_core::value::Value::Str(prompt)],
                    ) {
                        Ok(bl_core::value::Value::Str(s)) => Some(s + "\n"),
                        Ok(other) => Some(format!("{other}\n")),
                        Err(e) => Some(format!("Chat error: {e}\n")),
                    };
                    results.push(ExecutedBlock { block, output });
                    continue;
                }

                let buf = Arc::new(Mutex::new(String::new()));
                bl_runtime::builtins::set_output_buffer(Some(buf.clone()));

                let output = match run_code(&cb.code, &mut interpreter) {
                    Ok(()) => {
                        bl_runtime::builtins::set_output_buffer(None);
                        let captured = buf.lock().unwrap().clone();
                        Some(captured)
                    }
                    Err(msg) => {
                        bl_runtime::builtins::set_output_buffer(None);
                        let mut captured = buf.lock().unwrap().clone();
                        captured.push_str(&msg);
                        Some(captured)
                    }
                };
                results.push(ExecutedBlock { block, output });
            }
        }
    }

    results
}

fn run_code(code: &str, interpreter: &mut bl_runtime::Interpreter) -> Result<(), String> {
    let tokens = bl_lexer::Lexer::new(code)
        .tokenize()
        .map_err(|e| e.format_with_source(code))?;

    let parse_result = bl_parser::Parser::new(tokens)
        .parse()
        .map_err(|e| e.format_with_source(code))?;

    if parse_result.has_errors() {
        let msgs: Vec<String> = parse_result
            .errors
            .iter()
            .map(|e| e.format_with_source(code))
            .collect();
        return Err(msgs.join("\n"));
    }

    interpreter
        .run(&parse_result.program)
        .map(|_| ())
        .map_err(|e| e.format_with_source(code))
}

fn read_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{path}': {e}");
            std::process::exit(1);
        }
    }
}

// ── Terminal run (ANSI) ──────────────────────────────────────────────────────

pub fn run_notebook(path: &str) {
    let filename = PathBuf::from(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    eprintln!("\x1b[2m▶ running notebook {filename}\x1b[0m");
    eprintln!(
        "\x1b[2m  compute backend: {}\x1b[0m",
        bl_runtime::gpu::execution_summary()
    );
    let executed = execute_notebook(path);

    for eb in &executed {
        match &eb.block {
            Block::Prose(text) => {
                println!("{}", render_prose_ansi(text));
            }
            Block::Code(cb) => {
                if cb.directives.contains(&CellDirective::Skip) {
                    continue;
                }
                if cb.directives.contains(&CellDirective::Echo)
                    && !cb.directives.contains(&CellDirective::Hide)
                {
                    for line in cb.code.lines() {
                        eprintln!("\x1b[2m  {line}\x1b[0m");
                    }
                }
                if let Some(output) = &eb.output {
                    if !cb.directives.contains(&CellDirective::HideOutput) && !output.is_empty() {
                        print!("{output}");
                    }
                }
            }
        }
    }
}

// ── ANSI Markdown rendering ──────────────────────────────────────────────────

fn render_prose_ansi(text: &str) -> String {
    let mut out = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();

        // Headings
        if trimmed.starts_with("######") {
            out.push_str(&format!(
                "\x1b[1m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        } else if trimmed.starts_with("#####") {
            out.push_str(&format!(
                "\x1b[1m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        } else if trimmed.starts_with("####") {
            out.push_str(&format!(
                "\x1b[1m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        } else if trimmed.starts_with("###") {
            out.push_str(&format!(
                "\x1b[1m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        } else if trimmed.starts_with("##") {
            out.push_str(&format!(
                "\x1b[1;4m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        } else if trimmed.starts_with('#') && trimmed.chars().nth(1) == Some(' ') {
            out.push_str(&format!(
                "\x1b[1;4m{}\x1b[0m\n",
                trimmed.trim_start_matches('#').trim()
            ));
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push_str(&format!("\x1b[2m{}\x1b[0m\n", "-".repeat(40)));
        }
        // Block quote
        else if trimmed.starts_with("> ") {
            let content = render_inline_ansi(&trimmed[2..]);
            out.push_str(&format!("\x1b[2m  | {content}\x1b[0m\n"));
        }
        // Unordered list
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = render_inline_ansi(&trimmed[2..]);
            out.push_str(&format!("  {content}\n",));
        }
        // Blank line
        else if trimmed.is_empty() {
            out.push('\n');
        }
        // Normal paragraph line
        else {
            out.push_str(&render_inline_ansi(trimmed));
            out.push('\n');
        }
    }

    out
}

fn render_inline_ansi(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code: `...`
        if chars[i] == '`' {
            if let Some(end) = find_char(&chars, '`', i + 1) {
                out.push_str("\x1b[36m");
                for c in &chars[i + 1..end] {
                    out.push(*c);
                }
                out.push_str("\x1b[0m");
                i = end + 1;
                continue;
            }
        }
        // Bold: **...**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_char(&chars, '*', i + 2) {
                out.push_str("\x1b[1m");
                for c in &chars[i + 2..end] {
                    out.push(*c);
                }
                out.push_str("\x1b[0m");
                i = end + 2;
                continue;
            }
        }
        // Italic: *...*
        if chars[i] == '*' {
            if let Some(end) = find_char(&chars, '*', i + 1) {
                out.push_str("\x1b[3m");
                for c in &chars[i + 1..end] {
                    out.push(*c);
                }
                out.push_str("\x1b[0m");
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn find_char(chars: &[char], target: char, from: usize) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == target)
}

fn find_double_char(chars: &[char], target: char, from: usize) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&i| chars[i] == target && chars[i + 1] == target)
}

// ── HTML export ──────────────────────────────────────────────────────────────

pub fn export_html(path: &str) {
    let executed = execute_notebook(path);
    let filename = PathBuf::from(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "notebook".into());

    let mut body = String::new();

    for eb in &executed {
        match &eb.block {
            Block::Prose(text) => {
                body.push_str(&markdown_to_html(text));
            }
            Block::Code(cb) => {
                if cb.directives.contains(&CellDirective::Skip) {
                    continue;
                }
                if !cb.directives.contains(&CellDirective::Hide) {
                    body.push_str("<div class=\"cell-code\"><pre>");
                    body.push_str(&highlight_biolang(&cb.code));
                    body.push_str("</pre></div>\n");
                }
                if let Some(output) = &eb.output {
                    if !cb.directives.contains(&CellDirective::HideOutput) && !output.is_empty() {
                        emit_output(&mut body, output);
                    }
                }
            }
        }
    }

    println!(
        "{}",
        HTML_TEMPLATE
            .replace("{title}", &html_escape(&filename))
            .replace("{body}", &body)
            .replace("{figure_runtime}", FIGURE_FALLBACK_RUNTIME)
    );
}

/// Encode a value as a JavaScript string literal.
///
/// `</script>` inside embedded data would otherwise close the surrounding tag
/// and break the page, so the sequence is split as well as JSON-escaped.
fn js_string(value: &str) -> String {
    serde_json::Value::String(value.to_string())
        .to_string()
        .replace("</", "<\\/")
}

/// Collect the data files a notebook reads, so the exported page can carry them.
///
/// Without this the page renders correctly and every button is dead: the reader
/// has no `examples/sample-data/contigs.fa` to fetch, and the playground marks
/// a block unrunnable when it cannot account for a file the code opens. Reading
/// them at export time makes the artifact self-contained — code, output and
/// data in one file.
///
/// Anything missing or too large to embed is reported on stderr and left out
/// rather than silently dropped; those blocks stay marked CLI-only, which is
/// the truthful outcome.
fn collect_data_files(source: &str, notebook_path: &str) -> Vec<(String, String)> {
    const MAX_EMBEDDED_BYTES: usize = 4 * 1024 * 1024;

    let readers = [
        "read_csv",
        "read_tsv",
        "read_fasta",
        "read_fastq",
        "read_vcf",
        "read_bed",
        "read_gff",
        "csv",
        "tsv",
        "fasta",
        "fastq",
        "vcf",
        "bed",
        "gff",
    ];
    let notebook_dir = PathBuf::from(notebook_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();

    let mut found: Vec<(String, String)> = Vec::new();
    let mut total = 0usize;

    for reader in readers {
        let mut from = 0usize;
        while let Some(idx) = source[from..].find(reader) {
            let start = from + idx;
            from = start + reader.len();
            // Require a call: `reader` then optional spaces then `("literal"`.
            let rest = source[from..].trim_start();
            let Some(rest) = rest.strip_prefix('(') else {
                continue;
            };
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix('"') else {
                continue;
            };
            let Some(end) = rest.find('"') else { continue };
            let literal = &rest[..end];
            if literal.starts_with("http://") || literal.starts_with("https://") {
                continue;
            }
            let key = literal.replace('\\', "/");
            if found.iter().any(|(k, _)| k == &key) {
                continue;
            }
            // Try the path as written, then relative to the notebook.
            let candidates = [PathBuf::from(&key), notebook_dir.join(&key)];
            let Some(contents) = candidates
                .iter()
                .find_map(|c| std::fs::read_to_string(c).ok())
            else {
                eprintln!("note: {key} not found; blocks reading it stay CLI-only");
                continue;
            };
            if total + contents.len() > MAX_EMBEDDED_BYTES {
                eprintln!(
                    "note: {key} skipped, embedding it would exceed {} MB",
                    MAX_EMBEDDED_BYTES / (1024 * 1024)
                );
                continue;
            }
            total += contents.len();
            found.push((key, contents));
        }
    }
    found
}

/// Export a notebook as a page whose blocks are runnable in the browser.
///
/// The difference from `export_html` is what the reader can do, not what the
/// page says: both bake in the output of a real CLI run, so the page is
/// complete with JavaScript disabled. This one additionally ships a plain-HTML
/// editor, so a reader can edit cells and run them incrementally against one
/// shared WebAssembly interpreter.
///
/// The runtime itself is *not* inlined. It is ~5.9 MB (1.9 MB gzipped), and a
/// reader working through a set of tutorials should download it once and have
/// it cached, rather than once per page. `--wasm-base` points at wherever it is
/// served from.
///
/// The dependency-free editor and SVG/canvas fallback runtimes are embedded
/// from the website source at compile time rather than copied next to every
/// exported notebook.
pub fn export_html_wasm(path: &str, wasm_base: &str) {
    let executed = execute_notebook(path);
    let filename = PathBuf::from(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "notebook".into());

    let mut body = String::from(
        "<div class=\"bl-notebook-bar\" role=\"toolbar\" aria-label=\"Notebook controls\">\n\
           <span id=\"bl-kernel-status\">Browser kernel · starts on first run</span>\n\
           <button id=\"bl-run-all\" type=\"button\">Run all</button>\n\
         </div>\n\
         <p class=\"bl-runtime-note\">Runs locally in this browser with WebAssembly. Native libraries, GPU, unrestricted file access, and very large analyses require the <code>bl</code> CLI.</p>\n",
    );
    let mut cell_index = 0usize;
    for eb in &executed {
        match &eb.block {
            Block::Prose(text) => {
                body.push_str(&markdown_to_html(text));
            }
            Block::Code(cb) => {
                if cb.directives.contains(&CellDirective::Skip) {
                    continue;
                }
                if !cb.directives.contains(&CellDirective::Hide) {
                    cell_index += 1;
                    let hide_output = if cb.directives.contains(&CellDirective::HideOutput) {
                        " data-hide-output=\"true\""
                    } else {
                        ""
                    };
                    body.push_str(&format!(
                        "<section class=\"bl-notebook-cell\" data-cell=\"{cell_index}\"{hide_output}>\n\
                           <div class=\"bl-cell-toolbar\">\n\
                             <span class=\"bl-cell-count\">In [ ]</span>\n\
                             <span class=\"bl-cell-timing\"></span>\n\
                             <button class=\"bl-cell-run\" type=\"button\">Run</button>\n\
                           </div>\n\
                           <textarea class=\"bl-cell-editor\" aria-label=\"BioLang code cell {cell_index}\" spellcheck=\"false\" autocapitalize=\"off\" autocomplete=\"off\" wrap=\"off\">"
                    ));
                    body.push_str(&html_escape(&cb.code));
                    body.push_str("</textarea>\n<div class=\"bl-live-output\" hidden></div>\n");
                    if let Some(output) = &eb.output {
                        if !cb.directives.contains(&CellDirective::HideOutput) && !output.is_empty()
                        {
                            body.push_str("<div class=\"bl-saved-output\"><div class=\"bl-saved-label\">Saved output</div>");
                            emit_output(&mut body, output);
                            body.push_str("</div>\n");
                        }
                    }
                    body.push_str("</section>\n");
                    continue;
                }
                // Hidden cells are notebook setup, not discarded code. The
                // browser kernel evaluates them once when it starts so visible
                // cells see the same initial state as the exported CLI run.
                body.push_str(
                    "<div class=\"bl-notebook-bootstrap\" hidden><textarea aria-hidden=\"true\">",
                );
                body.push_str(&html_escape(&cb.code));
                body.push_str("</textarea></div>\n");
                if let Some(output) = &eb.output {
                    if !cb.directives.contains(&CellDirective::HideOutput) && !output.is_empty() {
                        emit_output(&mut body, output);
                    }
                }
            }
        }
    }

    // Everything the notebook reads, inlined, so the page works on its own.
    let all_code: String = executed
        .iter()
        .filter_map(|eb| match &eb.block {
            Block::Code(cb) => Some(cb.code.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let mut data = String::from(
        "window.__blFiles = window.__blFiles || {};
window.__blDataFiles = window.__blDataFiles || {};
",
    );
    for (key, contents) in collect_data_files(&all_code, path) {
        data.push_str(&format!(
            "window.__blFiles[{}] = {};
window.__blDataFiles[{}] = true;
",
            js_string(&key),
            js_string(&contents),
            js_string(&key)
        ));
    }

    let runtime = include_str!("../../../website/js/notebook-runtime.js");
    println!(
        "{}",
        HTML_WASM_TEMPLATE
            .replace("{title}", &html_escape(&filename))
            .replace("{wasm_base}", &html_escape(wasm_base.trim_end_matches('/')))
            .replace("{body}", &body)
            .replace("{data}", &data)
            .replace("{figure_runtime}", FIGURE_FALLBACK_RUNTIME)
            .replace("{runtime}", runtime)
    );
}

/// Write one cell's output, rendering an SVG figure instead of escaping it.
///
/// Escaping everything meant a notebook that plots showed the markup of its
/// figures as text, when seeing the plots is the main reason to export one.
/// Anything that is not a whole SVG document is escaped exactly as before.
///
/// The SVG is emitted as markup, which crosses no trust boundary the execution
/// did not already cross: it is the output of code the reader just ran locally,
/// on a page generated for them.
fn emit_output(body: &mut String, output: &str) {
    let mut cursor = 0usize;
    let mut found_figure = false;
    while let Some(relative_start) = output[cursor..].find("<svg") {
        let start = cursor + relative_start;
        let Some(relative_end) = output[start..].find("</svg>") else {
            break;
        };
        let end = start + relative_end + "</svg>".len();
        emit_text_output(body, &output[cursor..start]);
        body.push_str("<figure class=\"cell-figure\">");
        body.push_str(output[start..end].trim());
        body.push_str("</figure>\n");
        found_figure = true;
        cursor = end;
    }

    if found_figure {
        emit_text_output(body, &output[cursor..]);
    } else {
        emit_text_output(body, output);
    }
}

fn emit_text_output(body: &mut String, output: &str) {
    if output.trim().is_empty() {
        return;
    }
    body.push_str("<div class=\"cell-output\">");
    body.push_str(&html_escape(output));
    body.push_str("</div>\n");
}

const FIGURE_FALLBACK_RUNTIME: &str = include_str!("../../../website/js/figure-fallback.js");

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Typst export (research papers / PDF) ─────────────────────────────────────
//
// Walks the same executed blocks as `export_html`, emitting a Typst document
// (`<stem>.typ`) plus any SVG figures as sidecar files (`<stem>-fig-N.svg`)
// next to the notebook. Compile to PDF with `typst compile <stem>.typ`.

const TYPST_PREAMBLE: &str = "\
#set page(margin: 2.5cm, numbering: \"1\")
#set par(justify: true)
#set heading(numbering: \"1.1\")
#show raw.where(block: true): set text(size: 9pt)
";

/// Front matter for research-paper output (title block, abstract, bibliography).
#[derive(Default)]
struct FrontMatter {
    title: Option<String>,
    authors: Vec<String>,
    date: Option<String>,
    abstract_text: Option<String>,
    bibliography: Option<String>,
}

/// Split leading `key: value` front matter (delimited by `---` lines) from the
/// notebook body. Only recognized when the *first* line is `---` and every line
/// up to the closing `---` is a `key: value` pair — otherwise the leading `---`
/// is left intact for the body parser (it doubles as a dash code delimiter).
fn split_front_matter(source: &str) -> (Option<FrontMatter>, String) {
    let src = source.strip_prefix('\u{feff}').unwrap_or(source);
    let lines: Vec<&str> = src.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return (None, source.to_string());
    }
    let Some(close) = (1..lines.len()).find(|&i| lines[i].trim() == "---") else {
        return (None, source.to_string());
    };

    let mut fm = FrontMatter::default();
    let mut found_field = false;
    for line in &lines[1..close] {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let Some((key, value)) = t.split_once(':') else {
            return (None, source.to_string()); // not metadata — leave for body parser
        };
        let key = key.trim();
        let valid_key = key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !valid_key {
            return (None, source.to_string());
        }
        let value = value.trim().to_string();
        match key.to_ascii_lowercase().as_str() {
            "title" => fm.title = Some(value),
            "author" | "authors" => {
                fm.authors = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "date" => fm.date = Some(value),
            "abstract" => fm.abstract_text = Some(value),
            "bibliography" | "bib" => fm.bibliography = Some(value),
            _ => {} // unknown keys are ignored but still count as metadata
        }
        found_field = true;
    }
    if !found_field {
        return (None, source.to_string());
    }
    (Some(fm), lines[close + 1..].join("\n"))
}

/// Render the front-matter title block (title, authors, date, abstract).
fn front_matter_block(fm: &FrontMatter, fallback_title: &str) -> String {
    let title = fm.title.as_deref().unwrap_or(fallback_title);
    let mut s = String::new();
    s.push_str("#align(center)[\n");
    s.push_str(&format!(
        "  #text(17pt, weight: \"bold\")[{}]\n",
        typst_escape(title)
    ));
    if !fm.authors.is_empty() {
        let authors = fm
            .authors
            .iter()
            .map(|a| typst_escape(a))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("  #v(0.7em)\n  #text(11pt)[{authors}]\n"));
    }
    if let Some(date) = &fm.date {
        s.push_str(&format!(
            "  #v(0.3em)\n  #text(10pt, style: \"italic\")[{}]\n",
            typst_escape(date)
        ));
    }
    s.push_str("]\n#v(1em)\n\n");
    if let Some(abs) = &fm.abstract_text {
        s.push_str(&format!(
            "#block(inset: (x: 1.5em), below: 1.2em)[\n  #text(weight: \"bold\")[Abstract.] {}\n]\n\n",
            inline_to_typst(abs)
        ));
    }
    s
}

/// Build the `.typ` document (plus SVG sidecars) from a notebook.
/// Returns the written `.typ` path and the figure file names.
fn build_typst_document(path: &str) -> (PathBuf, Vec<String>) {
    let source = read_file(path);
    let (front, _body) = split_front_matter(&source);
    let executed = execute_notebook(path);

    let nb_path = PathBuf::from(path);
    let stem = nb_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "notebook".into());
    let out_dir = nb_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut body = String::new();
    let mut fig_count = 0usize;
    let mut written_figs: Vec<String> = Vec::new();

    for eb in &executed {
        match &eb.block {
            Block::Prose(text) => {
                body.push_str(&markdown_to_typst(text));
            }
            Block::Code(cb) => {
                if cb.directives.contains(&CellDirective::Skip) {
                    continue;
                }
                if !cb.directives.contains(&CellDirective::Hide) {
                    body.push_str(&typst_code_block(&cb.code));
                }
                if let Some(output) = &eb.output {
                    if !cb.directives.contains(&CellDirective::HideOutput)
                        && !output.trim().is_empty()
                    {
                        // A cell that prints an SVG plot becomes a figure; anything
                        // else renders as a verbatim output block.
                        if let Some(svg) = extract_svg(output) {
                            fig_count += 1;
                            let fig_name = format!("{stem}-fig-{fig_count}.svg");
                            let fig_path = out_dir.join(&fig_name);
                            match std::fs::write(&fig_path, svg) {
                                Ok(()) => {
                                    written_figs.push(fig_name.clone());
                                    body.push_str(&format!(
                                        "#figure(\n  image(\"{fig_name}\", width: 90%),\n  caption: [Figure {fig_count}.],\n)\n\n"
                                    ));
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Warning: cannot write figure {}: {e}",
                                        fig_path.display()
                                    );
                                }
                            }
                        } else {
                            body.push_str(&typst_output_block(output));
                        }
                    }
                }
            }
        }
    }

    let front = front.unwrap_or_default();
    if let Some(bib) = &front.bibliography {
        body.push_str(&format!("\n#bibliography(\"{}\")\n", bib));
    }

    let doc = format!(
        "{TYPST_PREAMBLE}\n{}{body}",
        front_matter_block(&front, &stem)
    );

    let typ_path = out_dir.join(format!("{stem}.typ"));
    if let Err(e) = std::fs::write(&typ_path, doc) {
        eprintln!("Error writing '{}': {e}", typ_path.display());
        std::process::exit(1);
    }
    (typ_path, written_figs)
}

pub fn export_typst(path: &str) {
    let (typ_path, figs) = build_typst_document(path);
    eprintln!("Wrote {}", typ_path.display());
    for fig in &figs {
        eprintln!("  + {fig}");
    }
    eprintln!("Compile: typst compile {}", typ_path.display());
}

/// Resolve the `typst` binary via `TYPST_BIN`, then PATH.
fn find_typst() -> Option<String> {
    if let Some(value) = std::env::var_os("TYPST_BIN") {
        let path = PathBuf::from(&value);
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    let on_path = Command::new("typst")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    on_path.then(|| "typst".to_string())
}

pub fn export_pdf(path: &str) {
    let (typ_path, _figs) = build_typst_document(path);
    let pdf_path = typ_path.with_extension("pdf");

    let Some(typst) = find_typst() else {
        eprintln!("Wrote {}", typ_path.display());
        eprintln!(
            "Typst is not installed, so the PDF was not produced.\n\
             Install it (e.g. `winget install Typst.Typst` or `cargo install typst-cli`)\n\
             or set TYPST_BIN, then run: typst compile {}",
            typ_path.display()
        );
        std::process::exit(1);
    };

    let status = Command::new(&typst)
        .arg("compile")
        .arg(&typ_path)
        .arg(&pdf_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("Wrote {}", pdf_path.display());
        }
        Ok(s) => {
            eprintln!("typst compile failed (exit {})", s.code().unwrap_or(-1));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Cannot run typst: {e}");
            std::process::exit(1);
        }
    }
}

/// Pull the first `<svg>…</svg>` block out of captured cell output, if present.
fn extract_svg(output: &str) -> Option<String> {
    let start = output.find("<svg")?;
    let end = output.rfind("</svg>")? + "</svg>".len();
    (end > start).then(|| output[start..end].to_string())
}

/// A biolang code cell as a Typst raw block.
fn typst_code_block(code: &str) -> String {
    format!("```biolang\n{}\n```\n\n", code.trim_end())
}

/// Verbatim cell output inside a shaded box.
fn typst_output_block(output: &str) -> String {
    format!(
        "#block(fill: luma(245), inset: 8pt, radius: 3pt, width: 100%, stroke: 0.5pt + luma(210))[\n```\n{}\n```\n]\n\n",
        output.trim_end()
    )
}

/// Escape Typst markup-significant characters in literal text.
fn typst_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        typst_escape_char(&mut out, c);
    }
    out
}

fn typst_escape_char(out: &mut String, c: char) {
    if matches!(c, '\\' | '#' | '$' | '*' | '_' | '`' | '@' | '[' | ']') {
        out.push('\\');
    }
    out.push(c);
}

/// Convert Markdown prose to Typst markup. Typst is markdown-adjacent, so this
/// is line-oriented and lighter than `markdown_to_html`.
fn markdown_to_typst(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count().min(6);
            let content = trimmed[level..].trim();
            out.push_str(&"=".repeat(level));
            out.push(' ');
            out.push_str(&inline_to_typst(content));
            out.push_str("\n\n");
        } else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            out.push_str("#line(length: 100%)\n\n");
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            out.push_str("#quote(block: true)[");
            out.push_str(&inline_to_typst(rest));
            out.push_str("]\n\n");
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            out.push_str("- ");
            out.push_str(&inline_to_typst(&trimmed[2..]));
            out.push('\n');
        } else if trimmed.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&inline_to_typst(trimmed));
            out.push('\n');
        }
    }
    out.push('\n');
    out
}

/// Inline Markdown → Typst: `` `code` ``, `**bold**` → `*bold*`, `*italic*` → `_italic_`.
fn inline_to_typst(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut out = String::new();
    let mut i = 0;

    while i < len {
        // Inline code — raw, kept literal.
        if chars[i] == '`' {
            if let Some(end) = find_char(&chars, '`', i + 1) {
                out.push('`');
                for c in &chars[i + 1..end] {
                    out.push(*c);
                }
                out.push('`');
                i = end + 1;
                continue;
            }
        }
        // Bold **x** → *x*
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_char(&chars, '*', i + 2) {
                out.push('*');
                out.push_str(&typst_escape(&chars[i + 2..end].iter().collect::<String>()));
                out.push('*');
                i = end + 2;
                continue;
            }
        }
        // Italic *x* → _x_
        if chars[i] == '*' {
            if let Some(end) = find_char(&chars, '*', i + 1) {
                out.push('_');
                out.push_str(&typst_escape(&chars[i + 1..end].iter().collect::<String>()));
                out.push('_');
                i = end + 1;
                continue;
            }
        }
        // Citation @key → Typst reference. Only at a word boundary (Pandoc rule),
        // so emails like `a@b` are left alone. A bare @ is escaped by the fallthrough.
        let at_word_boundary = i == 0 || !chars[i - 1].is_ascii_alphanumeric();
        if chars[i] == '@'
            && at_word_boundary
            && chars.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic())
        {
            let mut j = i + 1;
            while j < len
                && (chars[j].is_ascii_alphanumeric() || matches!(chars[j], '_' | '-' | ':' | '.'))
            {
                j += 1;
            }
            out.push('@');
            for c in &chars[i + 1..j] {
                out.push(*c);
            }
            i = j;
            continue;
        }
        typst_escape_char(&mut out, chars[i]);
        i += 1;
    }

    out
}

fn markdown_table_cells(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }
    let without_left = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let content = without_left.strip_suffix('|').unwrap_or(without_left);
    let cells = content.split('|').map(str::trim).collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn is_markdown_table_separator(line: &str, columns: usize) -> bool {
    markdown_table_cells(line).is_some_and(|cells| {
        cells.len() == columns
            && cells.iter().all(|cell| {
                let dashes = cell.trim().trim_start_matches(':').trim_end_matches(':');
                dashes.len() >= 3 && dashes.chars().all(|character| character == '-')
            })
    })
}

pub(crate) fn markdown_to_html(text: &str) -> String {
    let mut html = String::new();
    let mut in_list = false;
    let mut in_blockquote = false;
    let mut paragraph = String::new();
    let lines = text.lines().collect::<Vec<_>>();

    let flush_paragraph = |p: &mut String, h: &mut String| {
        let t = p.trim();
        if !t.is_empty() {
            h.push_str("<p>");
            h.push_str(&inline_to_html(t));
            h.push_str("</p>\n");
        }
        p.clear();
    };

    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();

        // Close list if we're no longer in one
        if in_list && !trimmed.starts_with("- ") && !trimmed.starts_with("* ") {
            html.push_str("</ul>\n");
            in_list = false;
        }

        // Close blockquote
        if in_blockquote && !trimmed.starts_with("> ") {
            html.push_str("</blockquote>\n");
            in_blockquote = false;
        }

        // GitHub-flavoured Markdown table. Detect it from the header followed
        // by a dash separator before treating either line as paragraph text.
        if let Some(headers) = markdown_table_cells(trimmed) {
            if lines
                .get(index + 1)
                .is_some_and(|separator| is_markdown_table_separator(separator, headers.len()))
            {
                flush_paragraph(&mut paragraph, &mut html);
                html.push_str("<div class=\"markdown-table-wrap\"><table>\n<thead><tr>");
                for header in &headers {
                    html.push_str("<th>");
                    html.push_str(&inline_to_html(header));
                    html.push_str("</th>");
                }
                html.push_str("</tr></thead>\n<tbody>\n");
                index += 2;
                while let Some(row) = lines.get(index).and_then(|line| markdown_table_cells(line)) {
                    if row.len() != headers.len() {
                        break;
                    }
                    html.push_str("<tr>");
                    for cell in row {
                        html.push_str("<td>");
                        html.push_str(&inline_to_html(cell));
                        html.push_str("</td>");
                    }
                    html.push_str("</tr>\n");
                    index += 1;
                }
                html.push_str("</tbody></table></div>\n");
                continue;
            }
        }

        // Headings
        if trimmed.starts_with('#') {
            flush_paragraph(&mut paragraph, &mut html);
            let level = trimmed.chars().take_while(|c| *c == '#').count().min(6);
            let content = trimmed[level..].trim();
            html.push_str(&format!(
                "<h{level}>{}</h{level}>\n",
                inline_to_html(content)
            ));
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            flush_paragraph(&mut paragraph, &mut html);
            html.push_str("<hr>\n");
        }
        // Block quote
        else if trimmed.starts_with("> ") {
            flush_paragraph(&mut paragraph, &mut html);
            if !in_blockquote {
                html.push_str("<blockquote>\n");
                in_blockquote = true;
            }
            html.push_str(&format!("<p>{}</p>\n", inline_to_html(&trimmed[2..])));
        }
        // Unordered list
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            flush_paragraph(&mut paragraph, &mut html);
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", inline_to_html(&trimmed[2..])));
        }
        // Blank line
        else if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut html);
        }
        // Regular text
        else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
        index += 1;
    }

    // Flush remaining
    {
        let t = paragraph.trim();
        if !t.is_empty() {
            html.push_str("<p>");
            html.push_str(&inline_to_html(t));
            html.push_str("</p>\n");
        }
    }
    if in_list {
        html.push_str("</ul>\n");
    }
    if in_blockquote {
        html.push_str("</blockquote>\n");
    }

    html
}

fn inline_to_html(text: &str) -> String {
    let escaped = html_escape(text);
    let chars: Vec<char> = escaped.chars().collect();
    let len = chars.len();
    let mut out = String::new();
    let mut i = 0;

    while i < len {
        // Inline code
        if chars[i] == '`' {
            if let Some(end) = find_char(&chars, '`', i + 1) {
                out.push_str("<code>");
                for c in &chars[i + 1..end] {
                    out.push(*c);
                }
                out.push_str("</code>");
                i = end + 1;
                continue;
            }
        }
        // Bold
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_char(&chars, '*', i + 2) {
                out.push_str("<strong>");
                for c in &chars[i + 2..end] {
                    out.push(*c);
                }
                out.push_str("</strong>");
                i = end + 2;
                continue;
            }
        }
        // Italic
        if chars[i] == '*' {
            if let Some(end) = find_char(&chars, '*', i + 1) {
                out.push_str("<em>");
                for c in &chars[i + 1..end] {
                    out.push(*c);
                }
                out.push_str("</em>");
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

// ── Syntax highlighting (HTML) ───────────────────────────────────────────────

fn highlight_biolang(code: &str) -> String {
    let keywords = [
        "let", "fn", "if", "else", "then", "for", "in", "while", "return", "match", "import",
        "true", "false", "nil", "and", "or", "not", "pipeline", "stage", "parallel", "defer",
        "break", "continue", "try", "catch", "given", "unless", "struct", "enum", "trait", "impl",
    ];
    let mut out = String::new();

    for line in code.lines() {
        let trimmed = line.trim_start();
        // Comment line
        if trimmed.starts_with('#') {
            out.push_str(&format!(
                "<span class=\"cmt\">{}</span>\n",
                html_escape(line)
            ));
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            // Inline comment
            if chars[i] == '#' {
                let rest: String = chars[i..].iter().collect();
                out.push_str(&format!(
                    "<span class=\"cmt\">{}</span>",
                    html_escape(&rest)
                ));
                break;
            }
            // String
            if chars[i] == '"' {
                let mut j = i + 1;
                while j < len && chars[j] != '"' {
                    if chars[j] == '\\' {
                        j += 1;
                    }
                    j += 1;
                }
                if j < len {
                    j += 1;
                }
                let s: String = chars[i..j].iter().collect();
                out.push_str(&format!("<span class=\"str\">{}</span>", html_escape(&s)));
                i = j;
                continue;
            }
            // Pipe operator
            if i + 1 < len && chars[i] == '|' && chars[i + 1] == '>' {
                out.push_str("<span class=\"op\">|&gt;</span>");
                i += 2;
                continue;
            }
            // Number
            if chars[i].is_ascii_digit()
                || (chars[i] == '-' && i + 1 < len && chars[i + 1].is_ascii_digit())
            {
                let start = i;
                if chars[i] == '-' {
                    i += 1;
                }
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class=\"num\">{}</span>", html_escape(&s)));
                continue;
            }
            // Identifier / keyword
            if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if keywords.contains(&word.as_str()) {
                    out.push_str(&format!("<span class=\"kw\">{word}</span>"));
                } else if i < len && chars[i] == '(' {
                    out.push_str(&format!("<span class=\"fn\">{word}</span>"));
                } else {
                    out.push_str(&word);
                }
                continue;
            }
            // Bio literals: dna"...", rna"...", protein"...", qual"..."
            out.push(chars[i]);
            i += 1;
        }
        out.push('\n');
    }

    // Remove trailing newline
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

const HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} — BioLang Notebook</title>
  <style>
    :root { --bg: #0f172a; --fg: #e2e8f0; --muted: #94a3b8; --accent: #8b5cf6; --code-bg: #1e293b; --output-bg: #1a1a2e; --border: #334155; }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--fg); max-width: 960px; margin: 0 auto; padding: 2rem 1.5rem; line-height: 1.7; }
    h1, h2, h3, h4, h5, h6 { color: #f8fafc; margin: 1.5rem 0 0.75rem; font-weight: 700; }
    h1 { font-size: 2rem; border-bottom: 2px solid var(--accent); padding-bottom: 0.5rem; }
    h2 { font-size: 1.5rem; }
    h3 { font-size: 1.25rem; }
    p { margin: 0.75rem 0; }
    .markdown-table-wrap { overflow-x: auto; margin: 0.9rem 0; }
    table { width: 100%; border-collapse: collapse; background: var(--output-bg); }
    th, td { padding: 0.5rem 0.65rem; border: 1px solid var(--border); text-align: left; vertical-align: top; }
    th { background: var(--code-bg); color: #f8fafc; }
    code { background: var(--code-bg); padding: 0.15em 0.4em; border-radius: 4px; font-size: 0.9em; font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; }
    strong { color: #f8fafc; }
    em { color: var(--muted); }
    hr { border: none; border-top: 1px solid var(--border); margin: 1.5rem 0; }
    ul { margin: 0.5rem 0; padding-left: 1.5rem; }
    li { margin: 0.25rem 0; }
    blockquote { border-left: 3px solid var(--accent); padding: 0.5rem 1rem; margin: 0.75rem 0; color: var(--muted); background: rgba(139, 92, 246, 0.05); border-radius: 0 6px 6px 0; }
    .cell-code { background: var(--code-bg); border: 1px solid var(--border); border-radius: 8px; padding: 1rem 1.25rem; margin: 0.75rem 0 0.25rem; overflow-x: auto; }
    .cell-code pre { margin: 0; font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: 0.875rem; line-height: 1.6; white-space: pre; }
    .cell-figure { margin: 0.25rem 0 1rem; padding: 0.5rem; background: #fff; border-radius: 6px; overflow-x: auto; }
    .cell-figure svg { max-width: 100%; height: auto; display: block; }
    .cell-figure-controls { display: flex; justify-content: flex-end; margin-bottom: 0.35rem; }
    .cell-figure-toggle { border: 1px solid #cbd5e1; border-radius: 4px; background: #f8fafc; color: #334155; padding: 0.2rem 0.5rem; font: 11px system-ui, sans-serif; cursor: pointer; }
    .cell-figure-toggle:disabled { display: none; }
    .cell-figure-canvas { display: block; }
    .cell-figure-canvas[hidden], .cell-figure svg[hidden] { display: none; }
    .cell-output { background: var(--output-bg); border-left: 3px solid #f59e0b; padding: 0.75rem 1rem; margin: 0.25rem 0 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.85rem; white-space: pre-wrap; border-radius: 0 6px 6px 0; color: #fbbf24; }
    .kw { color: #c084fc; font-weight: 600; }
    .str { color: #34d399; }
    .num { color: #60a5fa; }
    .cmt { color: #64748b; font-style: italic; }
    .fn { color: #fbbf24; }
    .op { color: #818cf8; font-weight: 700; }
    .meta { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.8rem; text-align: center; }
  </style>
</head>
<body>
{body}
  <div class="meta">Generated by BioLang Notebook</div>
<script>
{figure_runtime}
</script>
</body>
</html>"#;

/// Template for `--export html-wasm`.
///
/// Kept separate from the documentation playground: an exported notebook is
/// an editor with a persistent, incremental session, while documentation code
/// blocks are immutable examples that may be replayed independently.
const HTML_WASM_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="bl-wasm-base" content="{wasm_base}">
  <title>{title} — BioLang Notebook</title>
  <style>
    :root { --bg: #0f172a; --fg: #e2e8f0; --muted: #94a3b8; --accent: #8b5cf6; --code-bg: #1e293b; --output-bg: #1a1a2e; --border: #334155; }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--fg); max-width: 960px; margin: 0 auto; padding: 2rem 1.5rem; line-height: 1.7; }
    h1, h2, h3, h4, h5, h6 { color: #f8fafc; margin: 1.5rem 0 0.75rem; font-weight: 700; }
    h1 { font-size: 2rem; border-bottom: 2px solid var(--accent); padding-bottom: 0.5rem; }
    p { margin: 0.75rem 0; }
    .markdown-table-wrap { overflow-x: auto; margin: 0.9rem 0; }
    table { width: 100%; border-collapse: collapse; background: var(--output-bg); }
    th, td { padding: 0.5rem 0.65rem; border: 1px solid var(--border); text-align: left; vertical-align: top; }
    th { background: var(--code-bg); color: #f8fafc; }
    ul, ol { margin: 0.75rem 0 0.75rem 1.5rem; }
    a { color: #a78bfa; }
    code { font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; }
    p code, li code { background: var(--code-bg); padding: 0.1rem 0.35rem; border-radius: 4px; font-size: 0.9em; }
    button { font: inherit; }
    .bl-notebook-bar { position: sticky; top: 0.5rem; z-index: 20; display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin: 0 0 1.25rem; padding: 0.65rem 0.8rem; border: 1px solid var(--border); border-radius: 8px; background: rgba(15,23,42,0.96); color: var(--muted); font-size: 0.8rem; backdrop-filter: blur(8px); }
    .bl-notebook-bar button, .bl-cell-run { border: 0; border-radius: 5px; padding: 0.35rem 0.75rem; background: var(--accent); color: #fff; cursor: pointer; font-weight: 650; }
    .bl-notebook-bar button:disabled, .bl-cell-run:disabled { opacity: 0.55; cursor: wait; }
    .bl-runtime-note { margin: -0.75rem 0 1.25rem; color: var(--muted); font-size: 0.78rem; }
    .bl-notebook-cell { margin: 1rem 0 1.4rem; }
    .bl-cell-toolbar { min-height: 2rem; display: grid; grid-template-columns: 1fr auto auto; align-items: center; gap: 0.7rem; color: var(--muted); font: 0.75rem system-ui, sans-serif; }
    .bl-cell-count { color: #c4b5fd; font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; }
    .bl-cell-editor { display: block; width: 100%; min-height: 76px; resize: vertical; overflow: hidden; border: 1px solid var(--border); border-radius: 8px; padding: 0.9rem 1.1rem; background: var(--code-bg); color: var(--fg); font: 0.875rem/1.6 'JetBrains Mono', 'Fira Code', Consolas, monospace; tab-size: 2; white-space: pre; }
    .bl-cell-editor:focus { outline: 2px solid #8b5cf6; outline-offset: 1px; border-color: transparent; }
    .bl-notebook-cell.is-running .bl-cell-editor { border-color: #8b5cf6; }
    .bl-live-output, .bl-saved-output { margin-top: 0.35rem; }
    .bl-live-output[hidden], .bl-saved-output[hidden] { display: none; }
    .bl-saved-label { margin: 0 0 0.2rem 0.7rem; color: var(--muted); font: 0.7rem system-ui, sans-serif; }
    .bl-output-text, .bl-output-result, .bl-output-error, .bl-output-empty { margin: 0.2rem 0; padding: 0.7rem 0.9rem; border-left: 3px solid #64748b; border-radius: 0 6px 6px 0; background: var(--output-bg); color: var(--fg); font: 0.85rem/1.5 'JetBrains Mono', Consolas, monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
    .bl-output-result { border-left-color: #22c55e; color: #86efac; }
    .bl-output-error { border-left-color: #ef4444; color: #fca5a5; }
    .bl-output-empty { color: var(--muted); }
    .cell-figure { margin: 0.25rem 0 1rem; padding: 0.5rem; background: #fff; border-radius: 6px; overflow-x: auto; }
    .cell-figure svg { max-width: 100%; height: auto; display: block; }
    .cell-figure-controls { display: flex; justify-content: flex-end; margin-bottom: 0.35rem; }
    .cell-figure-toggle { border: 1px solid #cbd5e1; border-radius: 4px; background: #f8fafc; color: #334155; padding: 0.2rem 0.5rem; font: 11px system-ui, sans-serif; cursor: pointer; }
    .cell-figure-toggle:disabled { display: none; }
    .cell-figure-canvas { display: block; }
    .cell-figure-canvas[hidden], .cell-figure svg[hidden] { display: none; }
    .cell-output { background: var(--output-bg); border-left: 3px solid #f59e0b; padding: 0.75rem 1rem; margin: 0.25rem 0 1rem; font-family: 'JetBrains Mono', monospace; font-size: 0.85rem; white-space: pre-wrap; border-radius: 0 6px 6px 0; color: #fbbf24; }
    .kw { color: #c084fc; font-weight: 600; }
    .str { color: #34d399; }
    .num { color: #60a5fa; }
    .cmt { color: #64748b; font-style: italic; }
    .fn { color: #fbbf24; }
    .op { color: #818cf8; font-weight: 700; }
    .note { color: var(--muted); font-size: 0.8rem; margin: 0.25rem 0 1.25rem; }
    .meta { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.8rem; text-align: center; }
  </style>
</head>
<body class="components-loaded">
{body}
  <div class="meta">Generated by BioLang Notebook — outputs below each block are from the run that produced this page; press Run to execute it again in your browser.</div>
<script>
{data}
</script>
<script>
{figure_runtime}
</script>
<script>
{runtime}
</script>
</body>
</html>"##;

// ── Jupyter import/export ────────────────────────────────────────────────────

pub fn ipynb_to_bln(path: &str) {
    let source = read_file(path);
    let nb: serde_json::Value = match serde_json::from_str(&source) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing Jupyter notebook: {e}");
            std::process::exit(1);
        }
    };

    let cells = match nb.get("cells").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => {
            eprintln!("Error: no 'cells' array in notebook");
            std::process::exit(1);
        }
    };

    let mut first = true;
    for cell in cells {
        let cell_type = cell
            .get("cell_type")
            .and_then(|t| t.as_str())
            .unwrap_or("raw");
        let source_lines = cell.get("source").and_then(|s| s.as_array());

        let text = match source_lines {
            Some(lines) => lines.iter().filter_map(|l| l.as_str()).collect::<String>(),
            None => continue,
        };

        if text.trim().is_empty() {
            continue;
        }

        if !first {
            println!();
        }
        first = false;

        match cell_type {
            "markdown" | "raw" => {
                print!("{text}");
                if !text.ends_with('\n') {
                    println!();
                }
            }
            "code" => {
                println!("```biolang");
                print!("{text}");
                if !text.ends_with('\n') {
                    println!();
                }
                println!("```");
            }
            _ => {
                print!("{text}");
                if !text.ends_with('\n') {
                    println!();
                }
            }
        }
    }
}

pub fn bln_to_ipynb(path: &str) {
    let source = read_file(path);
    let blocks = parse_notebook(&source);

    let cells: Vec<serde_json::Value> = blocks
        .iter()
        .map(|block| match block {
            Block::Prose(text) => {
                serde_json::json!({
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": split_source_lines(text)
                })
            }
            Block::Code(cb) => {
                serde_json::json!({
                    "cell_type": "code",
                    "metadata": {},
                    "source": split_source_lines(&cb.code),
                    "execution_count": null,
                    "outputs": []
                })
            }
        })
        .collect();

    let notebook = serde_json::json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": {
            "kernelspec": {
                "display_name": "BioLang",
                "language": "biolang",
                "name": "biolang"
            },
            "language_info": {
                "name": "biolang",
                "file_extension": ".bl",
                "version": env!("CARGO_PKG_VERSION")
            }
        },
        "cells": cells
    });

    println!("{}", serde_json::to_string_pretty(&notebook).unwrap());
}

/// Split text into Jupyter source line format: each line gets a trailing \n except possibly the last.
fn split_source_lines(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return vec![];
    }
    let mut result: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i < lines.len() - 1 {
            result.push(format!("{line}\n"));
        } else {
            result.push(line.to_string());
        }
    }
    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // ── HTML export: figures ────────────────────────────────────────
    //
    // A notebook that plots used to export the markup of its figures as
    // escaped text, which is the opposite of what exporting one is for.

    #[test]
    fn svg_output_is_rendered_as_a_figure() {
        let mut body = String::new();
        emit_output(
            &mut body,
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>",
        );
        assert!(
            body.contains("cell-figure"),
            "not wrapped as a figure: {body}"
        );
        assert!(
            body.contains("<svg"),
            "the SVG was not emitted as markup: {body}"
        );
        assert!(!body.contains("&lt;svg"), "the SVG was escaped: {body}");
    }

    #[test]
    fn text_and_svg_output_are_rendered_separately() {
        let mut body = String::new();
        emit_output(
            &mut body,
            "cells: 3\n<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>\ndone\n",
        );
        assert_eq!(body.matches("cell-figure").count(), 1, "{body}");
        assert_eq!(body.matches("cell-output").count(), 2, "{body}");
        assert!(body.contains("cells: 3"), "{body}");
        assert!(body.contains("done"), "{body}");
        assert!(!body.contains("&lt;svg"), "{body}");
    }

    #[test]
    fn exported_pages_embed_the_canvas_fallback() {
        assert!(HTML_TEMPLATE.contains("{figure_runtime}"));
        assert!(HTML_WASM_TEMPLATE.contains("{figure_runtime}"));
        assert!(FIGURE_FALLBACK_RUNTIME.contains("getContext('2d')"));
    }

    #[test]
    fn live_notebook_runtime_is_editable_and_incremental() {
        let runtime = include_str!("../../../website/js/notebook-runtime.js");
        assert!(HTML_WASM_TEMPLATE.contains("bl-cell-editor"));
        assert!(runtime.contains("module.evaluate(source)"));
        assert!(runtime.contains("executeThrough"));
        assert!(runtime.contains("module.reset()"));
        assert!(runtime.contains("event.shiftKey"));
    }

    #[test]
    fn ordinary_output_is_still_escaped() {
        // The whole point of escaping: text that merely looks like markup must
        // not become markup.
        let mut body = String::new();
        emit_output(&mut body, "cells: 15049 <not a tag>");
        assert!(body.contains("cell-output"), "{body}");
        assert!(
            body.contains("&lt;not a tag&gt;"),
            "output was not escaped: {body}"
        );
    }

    #[test]
    fn text_that_merely_mentions_svg_is_not_treated_as_one() {
        // A partial or quoted SVG is not a document and must not be injected.
        let mut body = String::new();
        emit_output(&mut body, "wrote <svg> to disk");
        assert!(body.contains("cell-output"), "{body}");
        assert!(
            body.contains("&lt;svg&gt;"),
            "a fragment was emitted as markup: {body}"
        );
    }

    use super::*;

    #[test]
    fn test_parse_empty() {
        let blocks = parse_notebook("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_front_matter_parsed() {
        let src = "---\ntitle: My Paper\nauthors: Ada, Alan\nbibliography: refs.bib\n---\n# Body\n";
        let (fm, body) = split_front_matter(src);
        let fm = fm.expect("front matter should parse");
        assert_eq!(fm.title.as_deref(), Some("My Paper"));
        assert_eq!(fm.authors, vec!["Ada".to_string(), "Alan".to_string()]);
        assert_eq!(fm.bibliography.as_deref(), Some("refs.bib"));
        assert!(body.trim_start().starts_with("# Body"));
    }

    #[test]
    fn test_front_matter_not_confused_with_dash_code() {
        // A leading `---` dash code block is not metadata: leave it for the parser.
        let src = "---\nlet x = 1\nprintln(x)\n---\n";
        let (fm, body) = split_front_matter(src);
        assert!(fm.is_none());
        assert_eq!(body, src);
    }

    #[test]
    fn test_inline_typst_emphasis() {
        assert_eq!(inline_to_typst("**bold** and *em*"), "*bold* and _em_");
    }

    #[test]
    fn test_inline_typst_citation_vs_email() {
        // @key at a word boundary is a citation; an email's @ is escaped.
        assert_eq!(inline_to_typst("see @smith2020"), "see @smith2020");
        assert_eq!(inline_to_typst("mail a@b now"), "mail a\\@b now");
    }

    #[test]
    fn test_parse_prose_only() {
        let blocks = parse_notebook("## Hello\nSome text here.");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Prose(t) if t.contains("Hello")));
    }

    #[test]
    fn test_parse_code_block_dashes() {
        let src = "## Intro\n---\nlet x = 1\n---\n## End";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Prose(_)));
        assert!(matches!(&blocks[1], Block::Code(cb) if cb.code.contains("let x = 1")));
        assert!(matches!(&blocks[2], Block::Prose(_)));
    }

    #[test]
    fn test_parse_multiple_code_blocks() {
        let src = "---\nlet a = 1\n---\nMiddle\n---\nlet b = 2\n---";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Code(cb) if cb.code.contains("let a")));
        assert!(matches!(&blocks[1], Block::Prose(t) if t.contains("Middle")));
        assert!(matches!(&blocks[2], Block::Code(cb) if cb.code.contains("let b")));
    }

    // Fenced code blocks

    #[test]
    fn test_parse_fenced_biolang() {
        let src = "# Title\n```biolang\nlet x = 42\n```\nDone.";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Prose(_)));
        assert!(matches!(&blocks[1], Block::Code(cb) if cb.code.contains("let x = 42")));
        assert!(matches!(&blocks[2], Block::Prose(t) if t.contains("Done")));
    }

    #[test]
    fn test_parse_fenced_bl() {
        let src = "```bl\nprint(1)\n```";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Code(cb) if cb.code.contains("print(1)")));
    }

    #[test]
    fn test_parse_fenced_bare() {
        let src = "```\nlet y = 2\n```";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Code(cb) if cb.code.contains("let y = 2")));
    }

    #[test]
    fn test_parse_other_fence_is_prose() {
        let src = "Text\n```python\nprint('hi')\n```\nMore text";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 1);
        assert!(
            matches!(&blocks[0], Block::Prose(t) if t.contains("python") && t.contains("More text"))
        );
    }

    #[test]
    fn test_parse_mixed_fenced_and_dashes() {
        let src = "# Header\n---\nlet a = 1\n---\n```biolang\nlet b = 2\n```";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Prose(_)));
        assert!(matches!(&blocks[1], Block::Code(cb) if cb.code.contains("let a")));
        assert!(matches!(&blocks[2], Block::Code(cb) if cb.code.contains("let b")));
    }

    // Directives

    #[test]
    fn test_parse_directive_hide() {
        let src = "```\n# @hide\nlet x = 1\n```";
        let blocks = parse_notebook(src);
        assert_eq!(blocks.len(), 1);
        if let Block::Code(cb) = &blocks[0] {
            assert!(cb.directives.contains(&CellDirective::Hide));
            assert!(!cb.code.contains("@hide"));
            assert!(cb.code.contains("let x = 1"));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_directive_skip() {
        let src = "```\n# @skip\nlet x = 1\n```";
        let blocks = parse_notebook(src);
        if let Block::Code(cb) = &blocks[0] {
            assert!(cb.directives.contains(&CellDirective::Skip));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_directive_echo() {
        let src = "---\n# @echo\nlet x = 1\n---";
        let blocks = parse_notebook(src);
        if let Block::Code(cb) = &blocks[0] {
            assert!(cb.directives.contains(&CellDirective::Echo));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_directive_hide_output() {
        let src = "```\n# @hide-output\nprint(42)\n```";
        let blocks = parse_notebook(src);
        if let Block::Code(cb) = &blocks[0] {
            assert!(cb.directives.contains(&CellDirective::HideOutput));
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_parse_multiple_directives() {
        let src = "```\n# @echo\n# @hide-output\nlet x = 1\n```";
        let blocks = parse_notebook(src);
        if let Block::Code(cb) = &blocks[0] {
            assert!(cb.directives.contains(&CellDirective::Echo));
            assert!(cb.directives.contains(&CellDirective::HideOutput));
            assert_eq!(cb.directives.len(), 2);
        } else {
            panic!("expected code block");
        }
    }

    // ANSI rendering

    #[test]
    fn test_render_heading() {
        let result = render_prose_ansi("# Hello World");
        assert!(result.contains("Hello World"));
        assert!(result.contains("\x1b[1;4m"));
    }

    #[test]
    fn test_render_inline_code() {
        let result = render_inline_ansi("Use `dna` type");
        assert!(result.contains("\x1b[36m"));
        assert!(result.contains("dna"));
    }

    #[test]
    fn test_render_bold() {
        let result = render_inline_ansi("This is **bold** text");
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("bold"));
    }

    // HTML export helpers

    #[test]
    fn test_markdown_to_html_heading() {
        let html = markdown_to_html("# Title\n\nSome paragraph.");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Some paragraph.</p>"));
    }

    #[test]
    fn test_markdown_to_html_list() {
        let html = markdown_to_html("- Item A\n- Item B");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Item A</li>"));
        assert!(html.contains("<li>Item B</li>"));
    }

    #[test]
    fn markdown_tables_render_as_accessible_html() {
        let html = markdown_to_html(
            "| Question | BioLang function | What it returns |\n\
             |---|---|---|\n\
             | Quick overview | `summary(values)` | count, min, max |\n\
             | Equal-share centre | `mean(values)` | arithmetic mean |",
        );
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("<th>Question</th>"), "{html}");
        assert!(html.contains("<code>summary(values)</code>"), "{html}");
        assert!(html.contains("<td>arithmetic mean</td>"), "{html}");
        assert!(!html.contains("|---|"), "{html}");
    }

    #[test]
    fn test_highlight_keywords() {
        let html = highlight_biolang("let x = 1");
        assert!(html.contains("<span class=\"kw\">let</span>"));
        assert!(html.contains("<span class=\"num\">1</span>"));
    }

    #[test]
    fn test_highlight_comment() {
        let html = highlight_biolang("# comment");
        assert!(html.contains("<span class=\"cmt\">"));
    }

    #[test]
    fn test_highlight_string() {
        let html = highlight_biolang("let s = \"hello\"");
        assert!(html.contains("<span class=\"str\">"));
    }

    #[test]
    fn test_highlight_pipe() {
        let html = highlight_biolang("x |> print()");
        assert!(html.contains("<span class=\"op\">|&gt;</span>"));
    }

    // Jupyter helpers

    #[test]
    fn test_split_source_lines() {
        let lines = split_source_lines("line1\nline2\nline3");
        assert_eq!(lines, vec!["line1\n", "line2\n", "line3"]);
    }

    #[test]
    fn test_split_source_lines_single() {
        let lines = split_source_lines("single");
        assert_eq!(lines, vec!["single"]);
    }

    #[test]
    fn test_split_source_lines_empty() {
        let lines = split_source_lines("");
        assert!(lines.is_empty());
    }
}
