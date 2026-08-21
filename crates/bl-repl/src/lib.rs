use bl_core::value::{Table, Value};
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::builtins::{all_builtin_names, flush_trailing_newline};
use bl_runtime::Interpreter;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::{History, SearchDirection};
use rustyline::validate::Validator;
use rustyline::{Config, Context, Editor, Helper};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

const PROMPT: &str = "bl> ";
const CONTINUATION: &str = "+   ";

// ANSI color codes
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const UNDERLINE: &str = "\x1b[4m";
const RESET: &str = "\x1b[0m";

const REPL_COMMANDS: &[&str] = &[
    ":builtins",
    ":clear",
    ":cls",
    ":env",
    ":exit",
    ":paste",
    ":fns",
    ":h",
    ":help",
    ":history",
    ":load",
    ":plot",
    ":plugins",
    ":profile",
    ":q",
    ":quit",
    ":reset",
    ":restore",
    ":save",
    ":workspace",
    ":time",
    ":type",
];

/// (command, description) — used for auto-hints when typing `:` commands
const REPL_COMMAND_HINTS: &[(&str, &str)] = &[
    (":help", "Show help"),
    (":h", "Show help"),
    (":paste", "Paste a multi-line block; end it with a lone '.'"),
    (":quit", "Exit the REPL"),
    (":q", "Exit the REPL"),
    (":exit", "Exit the REPL"),
    (":env", "Show user-defined variables"),
    (":builtins", "List built-in functions [category]"),
    (":fns", "List built-in functions [category]"),
    (":type", "Show expression type <expr>"),
    (":time", "Evaluate with timing <expr>"),
    (":load", "Load a .bl script <file>"),
    (":save", "Save session to file <file>"),
    (":workspace", "Save variable values to a file [file]"),
    (":restore", "Restore variable values from a file [file]"),
    (":reset", "Clear all user-defined state"),
    (":plugins", "List installed plugins"),
    (":profile", "Profile expression <expr>"),
    (":clear", "Clear the screen"),
    (":cls", "Clear the screen"),
    (":history", "Show command history [n] or search [text]"),
    (":plot", "ASCII plot of last result [bins]"),
];

const KEYWORDS: &[&str] = &[
    "and", "else", "enum", "false", "fn", "for", "if", "import", "in", "into", "let", "match",
    "nil", "not", "or", "return", "true", "while", "yield",
];

// ── Tab Completion Helper ────────────────────────────────────────

struct BioHelper {
    words: Vec<String>,
    /// User-defined names, so hints can say where a suggestion came from.
    user_vars: Vec<String>,
    /// Display-only suffix appended by highlight_hint (not inserted on accept).
    hint_desc: RefCell<String>,
}

impl BioHelper {
    fn new() -> Self {
        let mut words: Vec<String> = KEYWORDS.iter().map(|s| s.to_string()).collect();
        // Runtime registration is authoritative; the curated catalog only adds metadata.
        for name in all_builtin_names() {
            if !words.contains(&name.to_string()) {
                words.push(name.to_string());
            }
        }
        words.sort();
        Self {
            words,
            user_vars: Vec::new(),
            hint_desc: RefCell::new(String::new()),
        }
    }

    /// Rebuild the completion word list from the interpreter's environment.
    fn refresh_from(&mut self, interp: &Interpreter) {
        let mut words: Vec<String> = KEYWORDS.iter().map(|s| s.to_string()).collect();
        // Include all builtin names
        for name in all_builtin_names() {
            words.push(name.to_string());
        }
        let mut user: Vec<String> = Vec::new();
        for (name, value) in interp.env().list_global_vars() {
            if name != "_" && !matches!(value, Value::NativeFunction { .. }) {
                user.push(name.to_string());
            }
            if !words.contains(&name.to_string()) {
                words.push(name.to_string());
            }
        }
        words.sort();
        words.dedup();
        user.sort();
        self.words = words;
        self.user_vars = user;
    }

    /// Longest common prefix of every word starting with `prefix`, plus how
    /// many matched. Used to show ghost text for an ambiguous prefix.
    fn common_completion(&self, prefix: &str) -> Option<(String, usize)> {
        let matches: Vec<&String> = self
            .words
            .iter()
            .filter(|w| w.starts_with(prefix))
            .collect();
        let (first, rest) = matches.split_first()?;
        let mut common = (*first).clone();
        for w in rest {
            let keep = common
                .char_indices()
                .zip(w.char_indices())
                .take_while(|((_, a), (_, b))| a == b)
                .count();
            common.truncate(
                common
                    .char_indices()
                    .nth(keep)
                    .map(|(i, _)| i)
                    .unwrap_or(common.len()),
            );
            if common.len() <= prefix.len() {
                break; // nothing further is shared; the count is still exact
            }
        }
        let completion = if common.len() <= prefix.len() {
            String::new()
        } else {
            common[prefix.len()..].to_string()
        };
        Some((completion, matches.len()))
    }
}

impl Completer for BioHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let text = &line[..pos];

        // Complete REPL commands
        if text.trim_start().starts_with(':') {
            if let Some(colon_pos) = text.find(':') {
                let prefix = &text[colon_pos..];
                let matches: Vec<Pair> = REPL_COMMANDS
                    .iter()
                    .filter(|c| c.starts_with(prefix) && **c != prefix)
                    .map(|c| Pair {
                        display: c.to_string(),
                        replacement: c.to_string(),
                    })
                    .collect();
                return Ok((colon_pos, matches));
            }
        }

        // Find current word boundary
        let start = text
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + text[i..].chars().next().map_or(1, |c| c.len_utf8()))
            .unwrap_or(0);
        let prefix = &text[start..];
        if prefix.is_empty() {
            return Ok((pos, vec![]));
        }

        let matches: Vec<Pair> = self
            .words
            .iter()
            .filter(|w| w.starts_with(prefix) && w.as_str() != prefix)
            .map(|w| Pair {
                display: w.clone(),
                replacement: w.clone(),
            })
            .collect();
        Ok((start, matches))
    }
}

impl Hinter for BioHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        let text = &line[..pos];
        if pos != line.len() || text.is_empty() {
            *self.hint_desc.borrow_mut() = String::new();
            return None;
        }

        // Hint for : commands — only return completion text, store description separately
        let trimmed = text.trim_start();
        if trimmed.starts_with(':') {
            for (cmd, desc) in REPL_COMMAND_HINTS {
                if cmd.starts_with(trimmed) && *cmd != trimmed {
                    *self.hint_desc.borrow_mut() = format!(" — {desc}");
                    return Some(cmd[trimmed.len()..].to_string());
                }
            }
            *self.hint_desc.borrow_mut() = String::new();
            return None;
        }

        // Find last word
        let start = text
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + text[i..].chars().next().map_or(1, |c| c.len_utf8()))
            .unwrap_or(0);
        let word = &text[start..];
        if word.len() < 2 || text.ends_with('(') {
            *self.hint_desc.borrow_mut() = String::new();
            return None;
        }

        // A known function completes to its full signature.
        if let Some(sig) = fn_signature(word) {
            *self.hint_desc.borrow_mut() = String::new();
            return Some(sig[word.len()..].to_string());
        }

        // Otherwise fall back to the completion word list, so user-defined
        // variables and builtins without a catalogued signature still get
        // ghost text.
        let (completion, n) = self.common_completion(word)?;
        let full = format!("{word}{completion}");
        *self.hint_desc.borrow_mut() = if n > 1 {
            format!("  ({n} matches — Tab to list)")
        } else if self.user_vars.contains(&full) {
            "  (your variable)".to_string()
        } else {
            String::new()
        };
        if completion.is_empty() && n <= 1 {
            return None;
        }
        Some(completion)
    }
}

impl Highlighter for BioHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        let desc = self.hint_desc.borrow();
        if desc.is_empty() {
            Cow::Owned(format!("{DIM}{hint}{RESET}"))
        } else {
            Cow::Owned(format!("{DIM}{hint}{desc}{RESET}"))
        }
    }

    fn highlight_char(
        &self,
        _line: &str,
        _pos: usize,
        _kind: rustyline::highlight::CmdKind,
    ) -> bool {
        // Force redraw so ANSI codes from hints don't leak into history recall
        true
    }
}

impl Validator for BioHelper {}
impl Helper for BioHelper {}

// ── REPL ─────────────────────────────────────────────────────────

pub struct Repl {
    interpreter: Interpreter,
    api_cache: ApiCache,
}

const CONSOLE_PROTOCOL: &str = "biolang.console/v1";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleRequest {
    id: u64,
    command: String,
    source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleVariable {
    name: String,
    type_name: String,
    preview: String,
    size_bytes: usize,
    members: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleEnvironment {
    variables: Vec<ConsoleVariable>,
    total_bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleValue {
    kind: &'static str,
    type_name: String,
    text: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    sequence: Option<String>,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleResponse {
    protocol: &'static str,
    id: u64,
    status: &'static str,
    output: String,
    value: Option<ConsoleValue>,
    error: Option<String>,
    duration_ms: u128,
    environment: ConsoleEnvironment,
}

/// Run a newline-delimited JSON session for editor integrations.
///
/// This protocol intentionally lives beside the interactive REPL so all clients
/// share the same parser, interpreter, last-result binding, and environment rules.
pub fn run_console_protocol() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, BufRead, Write};

    let mut interpreter = Interpreter::new();
    let current_file = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".biolang-console.bl");
    interpreter.set_current_file(Some(current_file));

    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<ConsoleRequest>(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = ConsoleResponse {
                    protocol: CONSOLE_PROTOCOL,
                    id: 0,
                    status: "error",
                    output: String::new(),
                    value: None,
                    error: Some(format!("Invalid console request: {error}")),
                    duration_ms: 0,
                    environment: console_environment(&interpreter),
                };
                serde_json::to_writer(&mut stdout, &response)?;
                writeln!(stdout)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = match request.command.as_str() {
            "evaluate" => evaluate_console_request(
                request.id,
                request.source.as_deref().unwrap_or_default(),
                &mut interpreter,
            ),
            "reset" => {
                interpreter.reset();
                ConsoleResponse {
                    protocol: CONSOLE_PROTOCOL,
                    id: request.id,
                    status: "ok",
                    output: String::new(),
                    value: None,
                    error: None,
                    duration_ms: 0,
                    environment: console_environment(&interpreter),
                }
            }
            "inspect" | "ping" => ConsoleResponse {
                protocol: CONSOLE_PROTOCOL,
                id: request.id,
                status: "ok",
                output: String::new(),
                value: None,
                error: None,
                duration_ms: 0,
                environment: console_environment(&interpreter),
            },
            command => ConsoleResponse {
                protocol: CONSOLE_PROTOCOL,
                id: request.id,
                status: "error",
                output: String::new(),
                value: None,
                error: Some(format!("Unknown console command '{command}'")),
                duration_ms: 0,
                environment: console_environment(&interpreter),
            },
        };
        serde_json::to_writer(&mut stdout, &response)?;
        writeln!(stdout)?;
        stdout.flush()?;
    }
    Ok(())
}

fn evaluate_console_request(
    id: u64,
    source: &str,
    interpreter: &mut Interpreter,
) -> ConsoleResponse {
    let started = Instant::now();
    let buffer = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    bl_runtime::builtins::set_output_buffer(Some(buffer.clone()));

    let result = Lexer::new(source)
        .tokenize()
        .map_err(|error| error.format_with_source(source))
        .and_then(|tokens| {
            Parser::new(tokens)
                .parse()
                .map_err(|error| error.format_with_source(source))
        })
        .and_then(|parsed| {
            if parsed.has_errors() {
                Err(parsed
                    .errors
                    .iter()
                    .map(|error| error.format_with_source(source))
                    .collect::<Vec<_>>()
                    .join("\n"))
            } else {
                interpreter
                    .run(&parsed.program)
                    .map_err(|error| error.format_with_source(source))
            }
        });

    bl_runtime::builtins::flush_trailing_newline();
    bl_runtime::builtins::set_output_buffer(None);
    let output = buffer.lock().map(|value| value.clone()).unwrap_or_default();
    let duration_ms = started.elapsed().as_millis();

    match result {
        Ok(value) => {
            let console_value = if matches!(value, Value::Nil) {
                None
            } else {
                interpreter.env_mut().define("_".to_string(), value.clone());
                Some(console_value(&value))
            };
            ConsoleResponse {
                protocol: CONSOLE_PROTOCOL,
                id,
                status: "ok",
                output,
                value: console_value,
                error: None,
                duration_ms,
                environment: console_environment(interpreter),
            }
        }
        Err(error) => ConsoleResponse {
            protocol: CONSOLE_PROTOCOL,
            id,
            status: "error",
            output,
            value: None,
            error: Some(error),
            duration_ms,
            environment: console_environment(interpreter),
        },
    }
}

fn console_environment(interpreter: &Interpreter) -> ConsoleEnvironment {
    let mut variables = interpreter
        .env()
        .list_global_vars()
        .into_iter()
        .filter(|(name, value)| {
            *name != "_"
                && !name.starts_with("__const_")
                && !matches!(value, Value::NativeFunction { .. })
        })
        .map(|(name, value)| ConsoleVariable {
            name: name.to_string(),
            type_name: value.type_of().to_string(),
            preview: value_preview(value),
            size_bytes: estimate_value_bytes(value),
            members: value_members(value),
        })
        .collect::<Vec<_>>();
    variables.sort_by(|left, right| left.name.cmp(&right.name));
    ConsoleEnvironment {
        total_bytes: variables.iter().map(|variable| variable.size_bytes).sum(),
        variables,
    }
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

fn console_value(value: &Value) -> ConsoleValue {
    const MAX_ROWS: usize = 200;
    const MAX_SEQUENCE: usize = 100_000;
    match value {
        Value::Table(table) => ConsoleValue {
            kind: "table",
            type_name: value.type_of().to_string(),
            text: value.to_string(),
            columns: table.columns.clone(),
            rows: table
                .rows
                .iter()
                .take(MAX_ROWS)
                .map(|row| row.iter().map(ToString::to_string).collect())
                .collect(),
            sequence: None,
            truncated: table.rows.len() > MAX_ROWS,
        },
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => ConsoleValue {
            kind: "sequence",
            type_name: value.type_of().to_string(),
            text: value_preview(value),
            columns: Vec::new(),
            rows: Vec::new(),
            sequence: Some(sequence.data.chars().take(MAX_SEQUENCE).collect()),
            truncated: sequence.data.chars().count() > MAX_SEQUENCE,
        },
        _ => ConsoleValue {
            kind: "text",
            type_name: value.type_of().to_string(),
            text: value.to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
            sequence: None,
            truncated: false,
        },
    }
}

fn estimate_value_bytes(value: &Value) -> usize {
    let base = std::mem::size_of::<Value>();
    match value {
        Value::Str(value) => base + value.len(),
        Value::List(values) => base + values.iter().map(estimate_value_bytes).sum::<usize>(),
        Value::Map(values) | Value::Record(values) => {
            base + values
                .iter()
                .map(|(key, value)| key.len() + estimate_value_bytes(value))
                .sum::<usize>()
        }
        Value::Table(table) => {
            base + table.columns.iter().map(String::len).sum::<usize>()
                + table
                    .rows
                    .iter()
                    .flatten()
                    .map(estimate_value_bytes)
                    .sum::<usize>()
        }
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            base + sequence.data.len()
        }
        Value::Matrix(matrix) => base + matrix.data.len() * std::mem::size_of::<f64>(),
        Value::Set(values) | Value::Tuple(values) => {
            base + values.iter().map(estimate_value_bytes).sum::<usize>()
        }
        Value::Quality(values) => base + values.len(),
        _ => base,
    }
}

impl Repl {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
            api_cache: ApiCache::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // CompletionType::List makes Tab print the candidates as a selectable
        // menu instead of silently cycling through them one at a time, so an
        // ambiguous prefix shows what the options actually are.
        let config = Config::builder()
            .bracketed_paste(true)
            .completion_type(rustyline::CompletionType::List)
            .completion_prompt_limit(60)
            .build();
        let mut rl = Editor::with_config(config)?;
        // Bind Esc to clear the current input line
        use rustyline::{Cmd, EventHandler, KeyCode, KeyEvent, Modifiers};
        rl.bind_sequence(
            KeyEvent(KeyCode::Esc, Modifiers::NONE),
            EventHandler::Simple(Cmd::Kill(rustyline::Movement::WholeBuffer)),
        );
        rl.set_helper(Some(BioHelper::new()));

        let history_path = dirs_history_path();
        if let Some(ref path) = history_path {
            let _ = rl.load_history(path);
        }

        // Initialize completion list from builtins
        if let Some(helper) = rl.helper_mut() {
            helper.refresh_from(&self.interpreter);
        }

        print_banner();

        // Clean up stale temp files from previous crashed sessions
        bl_runtime::tempfiles::cleanup_stale();

        let mut session_inputs: Vec<String> = Vec::new();
        let mut pending_line: Option<String> = None;
        // Track the last `let` binding name so |> can reference it
        // when `_` is not set (let statements produce Nil, not a value)
        let mut last_let_var: Option<String> = None;

        loop {
            let readline = if let Some(pl) = pending_line.take() {
                Ok(pl)
            } else {
                rl.readline(PROMPT)
            };
            match readline {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Add single-line commands to history immediately;
                    // multi-line inputs are added after continuation is gathered.
                    if line.starts_with(':') || line.starts_with('?') {
                        let _ = rl.add_history_entry(&line);
                    }

                    // ?function — quick help lookup
                    if trimmed.starts_with('?') {
                        let name = trimmed.strip_prefix('?').unwrap_or("").trim();
                        if !name.is_empty() {
                            cmd_fn_help(name);
                        } else {
                            println!("{DIM}Usage: ?function_name — show function signature{RESET}");
                        }
                        continue;
                    }

                    // REPL commands
                    if trimmed.starts_with(':') {
                        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                        let cmd = parts[0];
                        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

                        match cmd {
                            ":quit" | ":q" | ":exit" => break,
                            ":help" | ":h" => {
                                if arg.is_empty() {
                                    self.cmd_help();
                                } else {
                                    cmd_fn_help_extended(arg);
                                }
                            }
                            ":paste" => {
                                // Take lines verbatim until a lone "." and run
                                // the block as one unit.
                                //
                                // The bracket-counting continuation above is a
                                // heuristic, and heuristics on pasted code have
                                // to guess: a comment mentioning an unmatched
                                // brace, a terminal that does not support
                                // bracketed paste, a block whose first line
                                // errors and whose remaining lines should then
                                // not run. Here the reader says where the block
                                // ends, so there is nothing to infer.
                                println!(
                                    "  {}",
                                    "Paste your code, then a single '.' on its own line to run it (Ctrl-C cancels)."
                                );
                                let mut block = String::new();
                                loop {
                                    match rl.readline("  | ") {
                                        Ok(l) => {
                                            if l.trim() == "." {
                                                break;
                                            }
                                            block.push_str(&l);
                                            block.push('\n');
                                        }
                                        // Ctrl-C or Ctrl-D abandons the block
                                        // rather than running a partial paste.
                                        Err(_) => {
                                            block.clear();
                                            println!("  (paste cancelled)");
                                            break;
                                        }
                                    }
                                }
                                let block = block.trim_end().to_string();
                                if !block.is_empty() {
                                    let _ = rl.add_history_entry(block.as_str());
                                    self.eval_and_print(&block);
                                    if let Some(helper) = rl.helper_mut() {
                                        helper.refresh_from(&self.interpreter);
                                    }
                                }
                            }
                            ":env" => self.cmd_env(),
                            ":builtins" | ":fns" => cmd_builtins(arg),
                            ":reset" => {
                                self.cmd_reset();
                                if let Some(helper) = rl.helper_mut() {
                                    helper.refresh_from(&self.interpreter);
                                }
                            }
                            ":workspace" => {
                                self.cmd_workspace_save(arg);
                            }
                            ":restore" => {
                                self.cmd_workspace_load(arg);
                                if let Some(helper) = rl.helper_mut() {
                                    helper.refresh_from(&self.interpreter);
                                }
                            }
                            ":load" => {
                                if arg.is_empty() {
                                    eprintln!("{RED}Usage: :load <file.bl>{RESET}");
                                } else {
                                    self.cmd_load(arg);
                                    if let Some(helper) = rl.helper_mut() {
                                        helper.refresh_from(&self.interpreter);
                                    }
                                }
                            }
                            ":save" => {
                                if arg.is_empty() {
                                    eprintln!("{RED}Usage: :save <file>{RESET}");
                                } else {
                                    self.cmd_save(arg, &session_inputs);
                                }
                            }
                            ":time" => {
                                if arg.is_empty() {
                                    eprintln!("{RED}Usage: :time <expression>{RESET}");
                                } else {
                                    self.cmd_time(arg);
                                }
                            }
                            ":type" => {
                                if arg.is_empty() {
                                    eprintln!("{RED}Usage: :type <expression>{RESET}");
                                } else {
                                    self.cmd_type(arg);
                                }
                            }
                            ":plugins" => self.cmd_plugins(),
                            ":clear" | ":cls" => {
                                // CSI escape: clear screen + move cursor to top-left
                                print!("\x1b[2J\x1b[H");
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                            }
                            ":history" => {
                                cmd_history(arg, rl.history());
                            }
                            ":plot" => {
                                let bins = arg.parse::<usize>().ok();
                                let expr = if let Some(b) = bins {
                                    format!("_ |> hist({b})")
                                } else {
                                    "_ |> hist()".to_string()
                                };
                                self.eval_and_print(&expr);
                            }
                            ":profile" => {
                                if arg.is_empty() {
                                    eprintln!("{RED}Usage: :profile <expression>{RESET}");
                                } else {
                                    self.cmd_profile(arg);
                                }
                            }
                            _ => {
                                eprintln!(
                                    "{RED}Unknown command: {cmd}. Type :help for help.{RESET}"
                                );
                            }
                        }
                        continue;
                    }

                    // ── Build input (R-style: syntactic completeness) ──

                    // Leading pipe operator with no pending context → pipe from last result
                    let piped_from_last = trimmed.starts_with("|>") || trimmed.starts_with('~');
                    let mut input = if piped_from_last {
                        // Use `_` if defined, else fall back to the last `let` binding
                        let has_underscore = self
                            .interpreter
                            .env()
                            .get("_", None)
                            .map(|v| !matches!(v, Value::Nil))
                            .unwrap_or(false);
                        let pipe_source = if has_underscore {
                            "_".to_string()
                        } else if let Some(ref var) = last_let_var {
                            var.clone()
                        } else {
                            "_".to_string() // fallback — will give a clear error
                        };
                        format!("{pipe_source}\n{line}")
                    } else {
                        line
                    };

                    // Gather continuation lines while input is incomplete
                    // (unclosed delimiters, trailing |>, trailing operators,
                    //  block keywords without { )
                    loop {
                        if !needs_continuation(&input) {
                            // Input looks complete — but peek for a |> continuation
                            // so that multi-line pipe chains work:
                            //   let x = expr
                            //     |> map(...)
                            //     |> filter(...)
                            if !could_continue_with_pipe(&input) {
                                break;
                            }
                            match rl.readline(CONTINUATION) {
                                Ok(cont) => {
                                    let ct = cont.trim();
                                    if ct.starts_with("|>") || ct.starts_with('~') {
                                        input.push('\n');
                                        input.push_str(&cont);
                                        // keep looping — might be more pipe stages
                                    } else if ct.is_empty() {
                                        // blank line ends the expression
                                        break;
                                    } else {
                                        // Not a pipe continuation — save for next iteration
                                        pending_line = Some(cont);
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        } else {
                            match rl.readline(CONTINUATION) {
                                Ok(cont) => {
                                    input.push('\n');
                                    input.push_str(&cont);
                                }
                                Err(_) => break,
                            }
                        }
                    }

                    // Add the user-typed input to history (without the pipe-source prefix)
                    let history_entry = if piped_from_last {
                        // Strip the "varname\n" prefix we added
                        if let Some(idx) = input.find('\n') {
                            input[idx + 1..].to_string()
                        } else {
                            input.clone()
                        }
                    } else {
                        input.clone()
                    };
                    let _ = rl.add_history_entry(&history_entry);
                    session_inputs.push(input.clone());

                    // Track let binding names for pipe-from-last fallback
                    if let Some(var) = extract_let_var(&input) {
                        last_let_var = Some(var);
                    }

                    // ── Auto-detection (Phase B/C) ──
                    let input = detect_and_rewrite(input, &self.api_cache);

                    self.eval_and_print(&input);
                    if let Some(helper) = rl.helper_mut() {
                        helper.refresh_from(&self.interpreter);
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("{RED}Error: {err}{RESET}");
                    break;
                }
            }
        }

        if let Some(ref path) = history_path {
            let _ = rl.save_history(path);
        }

        // Clean up any temp files from disk-backed operations (kmer_count, etc.)
        bl_runtime::tempfiles::cleanup_all();

        Ok(())
    }

    fn eval_and_print(&mut self, input: &str) {
        let is_api_call = is_api_shorthand(input);
        let cache_key = if is_api_call {
            Some(self.api_cache.hash_key(input))
        } else {
            None
        };

        let tokens = match Lexer::new(input).tokenize() {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(input));
                return;
            }
        };

        let program = match Parser::new(tokens).parse() {
            Ok(r) => {
                if r.has_errors() {
                    for e in &r.errors {
                        eprintln!("{RED}{}{RESET}", e.format_with_source(input));
                    }
                    return;
                }
                r.program
            }
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(input));
                return;
            }
        };

        match self.interpreter.run(&program) {
            Ok(value) => {
                flush_trailing_newline();
                if !matches!(value, Value::Nil) {
                    // Cache API results
                    if let Some(ref key) = cache_key {
                        self.api_cache.store(key, &value);
                    }
                    // Store last result as `_` for pipe continuation
                    self.interpreter
                        .env_mut()
                        .define("_".to_string(), value.clone());
                    print_colored_value(&value);
                }
            }
            Err(e) => {
                flush_trailing_newline();
                // Offline fallback: try cache (ignoring TTL)
                if let Some(ref key) = cache_key {
                    if let Some((val, date)) = self.api_cache.load_any(key) {
                        eprintln!("{YELLOW}(offline: using cached result from {date}){RESET}");
                        self.interpreter
                            .env_mut()
                            .define("_".to_string(), val.clone());
                        print_colored_value(&val);
                        return;
                    }
                }
                eprintln!("{RED}{}{RESET}", e.format_with_source(input));
            }
        }
    }

    // ── Command handlers ─────────────────────────────────────────

    fn cmd_help(&self) {
        println!("{BOLD}Commands:{RESET}");
        println!("  {CYAN}:help{RESET}  :h [fn]       Show this help, or details for a function");
        println!("  {CYAN}:quit{RESET}  :q            Exit the REPL (or Ctrl+D)");
        println!("  {CYAN}:env{RESET}                 Show user-defined variables");
        println!("  {CYAN}:type{RESET}  <expr>        Show the type of an expression");
        println!("  {CYAN}:builtins{RESET} [category]  List built-in functions (:fns alias)");
        println!("  {CYAN}:load{RESET}  <file>        Load and execute a .bl script");
        println!(
            "  {CYAN}:workspace{RESET} [file]     Save variable VALUES (default ~/.biolang/workspace.json.gz)"
        );
        println!("  {CYAN}:restore{RESET} [file]      Restore variable values saved by :workspace");
        println!(
            "  {CYAN}:save{RESET}  <file>        Export last result (.csv/.tsv/.fasta/.json/.bl)"
        );
        println!("  {CYAN}:time{RESET}  <expr>        Evaluate and show elapsed time");
        println!("  {CYAN}:plot{RESET}  [bins]        ASCII histogram of last result");
        println!("  {CYAN}:reset{RESET}               Clear all user-defined state");
        println!("  {CYAN}:plugins{RESET}             List installed plugins");
        println!("  {CYAN}:profile{RESET} <expr>       Profile function calls in expression");
        println!("  {CYAN}:clear{RESET}  :cls          Clear the screen");
        println!("  {CYAN}:history{RESET} [n|text]     Show last n commands or fuzzy search");
        println!("  {CYAN}?{RESET}name                Show function signature (e.g. ?mean)");
        println!();
        println!("{BOLD}Auto-detection:{RESET}");
        println!(
            "  Paste raw DNA ({CYAN}ATCGATCG{RESET})  → auto-wraps as {CYAN}dna\"...\"{RESET}"
        );
        println!("  Paste FASTA ({CYAN}>header{RESET})     → parses into record");
        println!("  Type {CYAN}P53_HUMAN{RESET}            → calls {CYAN}uniprot_entry(){RESET}");
        println!("  Type {CYAN}6LU7{RESET}                 → calls {CYAN}pdb_entry(){RESET}");
        println!("  Type {CYAN}ncbi \"BRCA1\"{RESET}         → calls {CYAN}ncbi_gene(){RESET}");
        println!();
        println!("{BOLD}Syntax:{RESET}");
        println!("  {CYAN}let{RESET} x = 42           Bind a variable");
        println!("  {CYAN}fn{RESET} f(x) {{ x * 2 }}    Define a function");
        println!("  x {CYAN}|>{RESET} f()              Pipe (passes x as first arg)");
        println!("  {CYAN}|>{RESET} f()                  Pipe from last result ({CYAN}_{RESET})");
        println!("  {CYAN}import{RESET} \"mod\" as m     Load a .bl module");
        println!();
        println!("{BOLD}Quick start:{RESET}");
        println!("  [1, 2, 3] |> mean()          {DIM}# → 2.0{RESET}");
        println!("  |> to_string()               {DIM}# pipe from last result{RESET}");
        println!("  dna\"ATCG\" |> reverse_complement()  {DIM}# → CGAT{RESET}");
        println!("  :builtins stats              {DIM}# list stats functions{RESET}");
    }

    /// :save with format-aware export
    fn cmd_save(&mut self, path: &str, session_inputs: &[String]) {
        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "bl" => {
                // Save session inputs
                if session_inputs.is_empty() {
                    eprintln!("{DIM}No inputs to save.{RESET}");
                    return;
                }
                let content = session_inputs.join("\n");
                match std::fs::write(path, content) {
                    Ok(()) => println!(
                        "{DIM}Saved {} inputs to '{path}'.{RESET}",
                        session_inputs.len()
                    ),
                    Err(e) => eprintln!("{RED}Cannot write '{path}': {e}{RESET}"),
                }
            }
            "csv" => {
                let expr = format!("_ |> write_csv(\"{path}\")");
                self.eval_and_print(&expr);
            }
            "tsv" => {
                let expr = format!("_ |> write_tsv(\"{path}\")");
                self.eval_and_print(&expr);
            }
            "fasta" | "fa" => {
                let expr = format!("_ |> write_fasta(\"{path}\")");
                self.eval_and_print(&expr);
            }
            "json" => {
                // Direct serialize via serde_json
                let last = self.interpreter.env().get("_", None);
                match last {
                    Ok(val) => {
                        if matches!(val, Value::Nil) {
                            eprintln!("{DIM}No result to save.{RESET}");
                        } else {
                            let json = value_to_json(val);
                            match std::fs::write(
                                path,
                                serde_json::to_string_pretty(&json).unwrap_or_default(),
                            ) {
                                Ok(()) => println!("{DIM}Saved to '{path}'.{RESET}"),
                                Err(e) => eprintln!("{RED}Cannot write '{path}': {e}{RESET}"),
                            }
                        }
                    }
                    Err(_) => eprintln!("{DIM}No result to save.{RESET}"),
                }
            }
            _ => {
                // Default: save session inputs
                if session_inputs.is_empty() {
                    eprintln!("{DIM}No inputs to save.{RESET}");
                    return;
                }
                let content = session_inputs.join("\n");
                match std::fs::write(path, content) {
                    Ok(()) => println!(
                        "{DIM}Saved {} inputs to '{path}'.{RESET}",
                        session_inputs.len()
                    ),
                    Err(e) => eprintln!("{RED}Cannot write '{path}': {e}{RESET}"),
                }
            }
        }
    }

    fn human_bytes(n: u64) -> String {
        const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
        let mut v = n as f64;
        let mut u = 0;
        while v >= 1024.0 && u < UNITS.len() - 1 {
            v /= 1024.0;
            u += 1;
        }
        if u == 0 {
            format!("{n} B")
        } else {
            format!("{v:.1} {}", UNITS[u])
        }
    }

    /// Default workspace location — the values counterpart to the history file.
    fn default_workspace_path() -> String {
        match dirs_history_path() {
            Some(h) => h.replace("/history", "/workspace.json.gz"),
            None => "workspace.json.gz".to_string(),
        }
    }

    /// `:workspace [file]` — save variable VALUES (not the commands that made
    /// them; that is what `:save` does).
    fn cmd_workspace_save(&mut self, arg: &str) {
        let path = if arg.is_empty() {
            Self::default_workspace_path()
        } else {
            arg.to_string()
        };
        let vars: Vec<(&str, &Value)> = self
            .interpreter
            .env()
            .list_global_vars()
            .into_iter()
            .filter(|(k, v)| *k != "_" && !matches!(v, Value::NativeFunction { .. }))
            .collect();

        if vars.is_empty() {
            eprintln!("{DIM}No user-defined variables to save.{RESET}");
            return;
        }

        match bl_runtime::workspace::save(&path, vars) {
            Ok(report) => {
                println!(
                    "{GREEN}Saved {} variable(s){RESET} to {CYAN}{path}{RESET} ({})",
                    report.saved.len(),
                    Self::human_bytes(report.bytes)
                );
                if !report.skipped.is_empty() {
                    let names: Vec<String> = report
                        .skipped
                        .iter()
                        .map(|(n, t)| format!("{n} ({t})"))
                        .collect();
                    println!(
                        "{DIM}Not saved — functions and handles cannot be serialized: {}{RESET}",
                        names.join(", ")
                    );
                    println!("{DIM}Re-run their definitions, or :load the script that defines them.{RESET}");
                }
            }
            Err(e) => eprintln!("{RED}{e}{RESET}"),
        }
    }

    /// `:restore [file]` — bring saved values back into the session.
    fn cmd_workspace_load(&mut self, arg: &str) {
        let path = if arg.is_empty() {
            Self::default_workspace_path()
        } else {
            arg.to_string()
        };
        match bl_runtime::workspace::load(&path) {
            Ok(vars) => {
                let n = vars.len();
                let mut replaced = Vec::new();
                for (name, value) in vars {
                    if self.interpreter.env().get(&name, None).is_ok() {
                        replaced.push(name.clone());
                    }
                    self.interpreter.env_mut().define(name, value);
                }
                println!("{GREEN}Restored {n} variable(s){RESET} from {CYAN}{path}{RESET}");
                if !replaced.is_empty() {
                    println!("{DIM}Overwrote existing: {}{RESET}", replaced.join(", "));
                }
            }
            Err(e) => eprintln!("{RED}{e}{RESET}"),
        }
    }

    fn cmd_env(&self) {
        let vars = self.interpreter.env().list_global_vars();
        let mut user_vars: Vec<(&str, &Value)> = vars
            .into_iter()
            .filter(|(k, v)| *k != "_" && !matches!(v, Value::NativeFunction { .. }))
            .collect();

        if user_vars.is_empty() {
            println!("{DIM}No user-defined variables.{RESET}");
            return;
        }

        user_vars.sort_by_key(|(name, _)| *name);

        println!("{BOLD}User-defined variables:{RESET}");
        for (name, val) in &user_vars {
            let type_str = format!("{}", val.type_of());
            let preview = value_preview(val);
            println!("  {CYAN}{name:<16}{RESET} {type_str:<12} {preview}");
        }
    }

    fn cmd_reset(&mut self) {
        self.interpreter.reset();
        println!("{DIM}Environment reset.{RESET}");
    }

    fn cmd_load(&mut self, path: &str) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{RED}Cannot read '{path}': {e}{RESET}");
                return;
            }
        };

        let tokens = match Lexer::new(&content).tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(&content));
                return;
            }
        };

        let program = match Parser::new(tokens).parse() {
            Ok(r) => {
                if r.has_errors() {
                    for e in &r.errors {
                        eprintln!("{RED}{}{RESET}", e.format_with_source(&content));
                    }
                    return;
                }
                r.program
            }
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(&content));
                return;
            }
        };

        // Set current_file so imports in the loaded file resolve relative to it
        let prev_file = self.interpreter.current_file().cloned();
        let file_path = std::path::PathBuf::from(path);
        if let Ok(canonical) = std::fs::canonicalize(&file_path) {
            self.interpreter.set_current_file(Some(canonical));
        } else {
            self.interpreter.set_current_file(Some(file_path));
        }

        match self.interpreter.run(&program) {
            Ok(_) => println!("{DIM}Loaded '{path}'.{RESET}"),
            Err(e) => eprintln!("{RED}{}{RESET}", e.format_with_source(&content)),
        }

        self.interpreter.set_current_file(prev_file);
    }

    fn cmd_time(&mut self, expr: &str) {
        let tokens = match Lexer::new(expr).tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        let program = match Parser::new(tokens).parse() {
            Ok(r) => {
                if r.has_errors() {
                    for e in &r.errors {
                        eprintln!("{RED}{e}{RESET}");
                    }
                    return;
                }
                r.program
            }
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        let start = Instant::now();
        match self.interpreter.run(&program) {
            Ok(value) => {
                let elapsed = start.elapsed();
                if !matches!(value, Value::Nil) {
                    print_colored_value(&value);
                }
                println!("{DIM}(elapsed: {elapsed:.3?}){RESET}");
            }
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(expr));
            }
        }
    }

    fn cmd_plugins(&self) {
        let plugins = bl_runtime::plugins::list_installed_plugins();
        if plugins.is_empty() {
            println!("{DIM}No plugins installed.{RESET}");
            println!("{DIM}Use 'bl add <name> --path <dir>' to install a plugin.{RESET}");
            return;
        }
        println!("{BOLD}Installed plugins:{RESET}");
        for p in &plugins {
            println!(
                "  {CYAN}{:<20}{RESET} v{:<8} {DIM}({}){RESET}  {}",
                p.name, p.version, p.kind, p.description
            );
        }
    }

    fn cmd_type(&mut self, expr: &str) {
        let tokens = match Lexer::new(expr).tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        let program = match Parser::new(tokens).parse() {
            Ok(r) => {
                if r.has_errors() {
                    for e in &r.errors {
                        eprintln!("{RED}{e}{RESET}");
                    }
                    return;
                }
                r.program
            }
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        // Evaluate in a cloned env to avoid side effects
        let mut temp = Interpreter::with_env(self.interpreter.env().clone());
        match temp.run(&program) {
            Ok(val) => println!("{CYAN}{}{RESET}", val.type_of()),
            Err(e) => eprintln!("{RED}{e}{RESET}"),
        }
    }

    fn cmd_profile(&mut self, expr: &str) {
        let tokens = match Lexer::new(expr).tokenize() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        let program = match Parser::new(tokens).parse() {
            Ok(r) => {
                if r.has_errors() {
                    for e in &r.errors {
                        eprintln!("{RED}{e}{RESET}");
                    }
                    return;
                }
                r.program
            }
            Err(e) => {
                eprintln!("{RED}{e}{RESET}");
                return;
            }
        };

        // Enable profiling
        self.interpreter.profiling = Some(HashMap::new());

        let start = Instant::now();
        let result = self.interpreter.run(&program);
        let elapsed = start.elapsed();

        // Grab and disable profiling
        let profile_data = self.interpreter.profiling.take().unwrap_or_default();

        match result {
            Ok(value) => {
                if !matches!(value, Value::Nil) {
                    print_colored_value(&value);
                }
            }
            Err(e) => {
                eprintln!("{RED}{}{RESET}", e.format_with_source(expr));
            }
        }

        // Print profiling results
        println!();
        println!("{BOLD}Profile ({elapsed:.3?} total):{RESET}");

        if profile_data.is_empty() {
            println!("{DIM}  No function calls recorded.{RESET}");
            return;
        }

        // Sort by total time descending
        let mut entries: Vec<(&String, &(u64, u128))> = profile_data.iter().collect();
        entries.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

        println!(
            "  {BOLD}{:<30} {:>8} {:>12} {:>12}{RESET}",
            "Function", "Calls", "Total", "Avg"
        );
        println!("  {}", "-".repeat(66));

        for (name, (calls, total_ns)) in &entries {
            let total_us = *total_ns as f64 / 1000.0;
            let avg_us = if *calls > 0 {
                total_us / *calls as f64
            } else {
                0.0
            };
            let total_str = if total_us >= 1_000_000.0 {
                format!("{:.2}s", total_us / 1_000_000.0)
            } else if total_us >= 1_000.0 {
                format!("{:.2}ms", total_us / 1_000.0)
            } else {
                format!("{:.1}us", total_us)
            };
            let avg_str = if avg_us >= 1_000.0 {
                format!("{:.2}ms", avg_us / 1_000.0)
            } else {
                format!("{:.1}us", avg_us)
            };
            println!(
                "  {CYAN}{:<30}{RESET} {:>8} {:>12} {:>12}",
                name, calls, total_str, avg_str
            );
        }
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ──────────────────────────────────────────────────────

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinArityMetadata {
    pub kind: &'static str,
    pub minimum: usize,
    pub maximum: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinMetadata {
    pub name: String,
    pub signature: String,
    pub category: String,
    pub summary: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub example: Option<String>,
    pub arity: BuiltinArityMetadata,
    pub metadata_quality: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BioLangMetadata {
    pub schema_version: u32,
    pub language: &'static str,
    pub language_version: &'static str,
    pub builtins: Vec<BuiltinMetadata>,
}

fn metadata_arity(arity: &bl_core::value::Arity) -> BuiltinArityMetadata {
    match arity {
        bl_core::value::Arity::Exact(count) => BuiltinArityMetadata {
            kind: "exact",
            minimum: *count,
            maximum: Some(*count),
        },
        bl_core::value::Arity::AtLeast(minimum) => BuiltinArityMetadata {
            kind: "atLeast",
            minimum: *minimum,
            maximum: None,
        },
        bl_core::value::Arity::Range(minimum, maximum) => BuiltinArityMetadata {
            kind: "range",
            minimum: *minimum,
            maximum: Some(*maximum),
        },
    }
}

fn fallback_signature(name: &str, arity: &BuiltinArityMetadata) -> String {
    let parameters = match (arity.minimum, arity.maximum) {
        (0, Some(0)) => String::new(),
        (minimum, Some(maximum)) if minimum == maximum => (1..=minimum)
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", "),
        (minimum, Some(maximum)) => {
            let mut values = (1..=minimum)
                .map(|index| format!("arg{index}"))
                .collect::<Vec<_>>();
            values.extend((minimum + 1..=maximum).map(|index| format!("arg{index}?")));
            values.join(", ")
        }
        (minimum, None) => {
            let mut values = (1..=minimum)
                .map(|index| format!("arg{index}"))
                .collect::<Vec<_>>();
            values.push("...".into());
            values.join(", ")
        }
    };
    format!("{name}({parameters})")
}

fn signature_parts(signature: &str) -> (Vec<String>, Option<String>) {
    let parameters = signature
        .find('(')
        .and_then(|open| {
            let mut depth = 0usize;
            signature[open + 1..]
                .char_indices()
                .find_map(|(index, character)| match character {
                    '(' | '[' | '{' | '<' => {
                        depth += 1;
                        None
                    }
                    ')' if depth == 0 => Some(&signature[open + 1..open + 1 + index]),
                    ')' | ']' | '}' | '>' => {
                        depth = depth.saturating_sub(1);
                        None
                    }
                    _ => None,
                })
        })
        .filter(|parameters| !parameters.trim().is_empty())
        .map(|parameters| {
            parameters
                .split(',')
                .map(|parameter| parameter.trim().to_string())
                .collect()
        })
        .unwrap_or_default();
    let return_type = signature
        .split_once("->")
        .or_else(|| signature.split_once('→'))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty());
    (parameters, return_type)
}

/// Versioned metadata used by the CLI, LSP, Desktop, and documentation tooling.
pub fn biolang_metadata() -> BioLangMetadata {
    let arities = bl_runtime::builtins::all_builtin_arities()
        .into_iter()
        .collect::<HashMap<_, _>>();
    let catalog = BUILTIN_CATALOG
        .iter()
        .map(|(name, signature, category)| (*name, (*signature, *category)))
        .collect::<HashMap<_, _>>();
    let examples = BUILTIN_EXAMPLES
        .iter()
        .map(|(name, example, return_type)| (*name, (*example, *return_type)))
        .collect::<HashMap<_, _>>();
    let summaries = BUILTIN_SUMMARIES.iter().copied().collect::<HashMap<_, _>>();

    let mut builtins = all_builtin_names()
        .into_iter()
        .map(|name| {
            let runtime_arity = arities
                .get(name)
                .cloned()
                .unwrap_or(bl_core::value::Arity::AtLeast(0));
            let arity = metadata_arity(&runtime_arity);
            let catalog_entry = catalog.get(name);
            let signature = catalog_entry
                .map(|(signature, _)| (*signature).to_string())
                .unwrap_or_else(|| fallback_signature(name, &arity));
            let (parameters, signature_return) = signature_parts(&signature);
            let example = examples.get(name);
            let category = catalog_entry
                .map(|(_, category)| (*category).to_string())
                .unwrap_or_else(|| "runtime".into());
            let return_type = example
                .map(|(_, return_type)| (*return_type).to_string())
                .or(signature_return);
            BuiltinMetadata {
                name: name.to_string(),
                signature,
                summary: summaries
                    .get(name)
                    .map(|summary| (*summary).to_string())
                    .or_else(|| catalog_entry.map(|_| format!("BioLang {category} builtin.")))
                    .unwrap_or_else(|| {
                        format!(
                            "Registered BioLang builtin accepting {} argument(s).",
                            arity.minimum
                        )
                    }),
                category,
                parameters,
                return_type,
                example: example.map(|(example, _)| (*example).to_string()),
                arity,
                metadata_quality: if catalog_entry.is_some() {
                    "curated"
                } else {
                    "runtime"
                },
            }
        })
        .collect::<Vec<_>>();
    builtins.sort_by(|left, right| left.name.cmp(&right.name));

    BioLangMetadata {
        schema_version: 1,
        language: "BioLang",
        language_version: VERSION,
        builtins,
    }
}

fn print_banner() {
    let builtin_count = all_builtin_names().len();
    let ncbi_status = if std::env::var("NCBI_API_KEY").is_ok() {
        "+"
    } else {
        "-"
    };
    let llm_status = if std::env::var("ANTHROPIC_API_KEY").is_ok()
        || std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("OLLAMA_MODEL").is_ok()
    {
        "+"
    } else {
        "-"
    };
    let cache_info = cache_dir_path()
        .filter(|p| p.exists())
        .and_then(|p| dir_size(&p).ok())
        .map(|sz| format!("  Cache: {}", format_cache_size(sz)))
        .unwrap_or_default();

    println!(
        r#"
{BOLD}{CYAN}  ____  _       _
 | __ )(_) ___ | |    __ _ _ __   __ _
 |  _ \| |/ _ \| |   / _` | '_ \ / _` |
 | |_) | | (_) | |__| (_| | | | | (_| |
 |____/|_|\___/|_____\__,_|_| |_|\__, |
                                  |___/{RESET}
 {DIM}BioLang — pipe-first bioinformatics DSL{RESET}
 {DIM}v{VERSION}  •  {builtin_count} builtins  •  NCBI[{ncbi_status}] LLM[{llm_status}]{cache_info}{RESET}

 {BOLD}Commands:{RESET}  {CYAN}:help{RESET}  {CYAN}:builtins{RESET}  {CYAN}:quit{RESET}  {CYAN}?{RESET}name  {DIM}Tab for completion  •  Paste DNA/FASTA auto-detected{RESET}
"#
    );
}

fn cache_dir_path() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(|h| PathBuf::from(h).join(".biolang").join("cache"))
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_file() {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

fn format_cache_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1}MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0}KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes}B")
    }
}

// ── History search ───────────────────────────────────────────────

fn cmd_history(arg: &str, history: &dyn History) {
    let len = history.len();
    if arg.is_empty() || arg.parse::<usize>().is_ok() {
        // Numeric: show last N entries
        let n: usize = arg.parse().unwrap_or(20);
        let start = len.saturating_sub(n);
        for i in start..len {
            if let Ok(Some(result)) = history.get(i, SearchDirection::Forward) {
                println!("{DIM}{:>4}{RESET}  {}", i + 1, result.entry);
            }
        }
    } else {
        // Fuzzy search: case-insensitive substring match
        let query = arg.to_lowercase();
        let mut count = 0;
        for i in 0..len {
            if let Ok(Some(result)) = history.get(i, SearchDirection::Forward) {
                let entry = &result.entry;
                if let Some(pos) = entry.to_lowercase().find(&query) {
                    let before = &entry[..pos];
                    let matched = &entry[pos..pos + arg.len()];
                    let after = &entry[pos + arg.len()..];
                    println!(
                        "{DIM}{:>4}{RESET}  {before}{BOLD}{YELLOW}{matched}{RESET}{after}",
                        i + 1
                    );
                    count += 1;
                }
            }
        }
        if count == 0 {
            println!("{DIM}No history entries matching '{arg}'.{RESET}");
        }
    }
}

// ── Extended function help ──────────────────────────────────────

/// Curated examples for common builtins
/// One-line purposes for builtins whose category label is not enough on its own.
///
/// Catalogued builtins otherwise summarise as "BioLang <category> builtin.",
/// which is fine for `mean` and useless for `stats_cox_diagnostics`. These are
/// taken from packages/statistics/README.md, so the reference and the package
/// documentation say the same thing.
const BUILTIN_SUMMARIES: &[(&str, &str)] = &[
    ("stats_explore", "Centre, spread, shape, missingness, transformation candidates and review flags for one numeric variable."),
    ("stats_compare", "Per-group evidence and the analyses appropriate to it, without choosing one."),
    ("stats_relationship", "Complete-pair counts, Pearson and Spearman association, and a regression line."),
    ("stats_categories", "Counts, proportions, modes, missingness and rare-level clues for a categorical variable."),
    ("stats_guide", "Attach an explicit scientific question and experimental unit to a report."),
    ("stats_explain", "Render a report as quick, learning or audit text."),
    ("stats_shape", "Skewness, kurtosis, histogram-peak and normal-Q-Q evidence, with no diagnosis."),
    ("stats_uncertainty", "Seeded bootstrap interval for a centre, spread, group difference or correlation."),
    ("stats_means", "Every mean paired with a compatible spread, so neither is quoted alone."),
    ("stats_preprocess", "Observable data-quality issues and non-applied normalisation alternatives."),
    ("stats_transform_preview", "Before-and-after evidence for a transform, without applying it."),
    ("stats_distribution_clues", "Scale-sensitive normal, log-normal, Poisson and negative-binomial fit clues."),
    ("stats_profile", "Whole-table types, summaries, missingness, duplicates and design clues."),
    ("stats_missingness", "Missingness by row, column, pair and optional group."),
    ("stats_design_check", "Repeated units, imbalance, and batch or group confounding."),
    ("stats_associations", "Bounded Pearson, Spearman, Cramer's V and categorical-numeric effect sizes."),
    ("stats_scan", "One-command profile, association screen and prioritised next steps."),
    ("stats_report", "Self-contained HTML or Markdown data-health report with provenance."),
    ("stats_normalization_guide", "Dense or sparse matrix audit and domain-aware normalisation alternatives."),
    ("stats_omics_profile", "Modality-aware matrix profile that preserves sparse semantics."),
    ("stats_linear_diagnostics", "Residual form, spread, Q-Q, order and influence clues for a simple linear model."),
    ("stats_multiple_linear_diagnostics", "Categorical encoding, interactions, VIF, influence and deterministic held-out error."),
    ("stats_robust_linear_diagnostics", "Huber regression as an explicit sensitivity check against the ordinary fit."),
    ("stats_glm_diagnostics", "Binomial or Poisson fit with deviance, dispersion, influence and calibration clues. Check `converged` before reading anything else."),
    ("stats_random_intercept_model", "One random intercept by REML: fixed effects, variance components, ICC and partially pooled intercepts."),
    ("stats_cox_diagnostics", "Multivariable Cox fit with Breslow ties, hazard-ratio intervals, baseline hazard and martingale residuals. The Schoenfeld screen is descriptive, not cox.zph."),
    ("stats_weighted_summary", "Weighted moments with the weighting scheme and effective sample size disclosed."),
    ("stats_time_series_diagnostics", "Autocorrelation, Ljung-Box and trend evidence for ordered observations."),
    ("stats_cluster_diagnostics", "Quantify declared non-independence without fitting a model."),
    ("stats_decision_map", "The questions that narrow a method, with `automatic_choice` false."),
    ("stats_distribution_plot", "Annotated histogram: observations, mean, median, IQR, SD bands and outlier flags."),
    ("stats_distribution_ascii", "Terminal-safe histogram, with exclusions and review flags stated."),
    ("stats_normal_diagram", "Normal-curve teaching diagram with 1/2/3-SD regions, optional observed coverage and z-tail highlighting."),
    ("stats_visualize", "Render an exploration report's visual guide as SVG or terminal-safe ASCII."),
    ("stats_normal_qq_plot", "Normal-distribution Q-Q diagnostic, distinct from the genomic qq_plot()."),
    ("stats_group_plot", "Group observations and robust summaries, in SVG or ASCII."),
    ("stats_relationship_plot", "Scatterplot and fitted line, in SVG or ASCII."),
    ("stats_categorical_plot", "Frequency bars, in SVG or ASCII."),
    ("stats_missingness_plot", "Missingness map, in SVG or ASCII."),
    ("stats_linear_diagnostic_plot", "Residual-versus-fitted or residual Q-Q display."),
    ("stats_overview_ascii", "Compact terminal-safe whole-table summary."),
    // Where BioLang deliberately differs from R's default, the summary says so:
    // a reader comparing the two would otherwise conclude BioLang is wrong.
    ("ttest", "Two-sample t test. Two arguments preserve the pooled Student form; pass {variance: \"welch\"} for Welch/R-default inference. Returns method, interval, and effect size."),
    ("wilcoxon", "Independent-group Mann-Whitney rank-sum test. Default: normal approximation without continuity correction; options select method: \"exact\" or continuity: true."),
    ("wilcoxon_paired", "Paired Wilcoxon signed-rank test on a-b differences. Default: tie-corrected normal approximation; options select method: \"exact\" or continuity: true."),
    ("fisher_exact", "Two-sided Fisher exact test on a 2x2 table. Reports the sample cross-product odds ratio and a labelled Wald log-odds interval; R reports a conditional estimate."),
    ("anova", "One-way analysis of means. One argument preserves classical equal-variance ANOVA; pass {variance: \"welch\"} for unequal variances. Returns method, effect sizes, and sums of squares."),
    ("kruskal_wallis", "Independent-group Kruskal-Wallis rank-sum test with tie correction and epsilon-squared effect size."),
    ("tukey_hsd", "Tukey-Kramer all-pairs comparison using the studentized-range distribution and simultaneous family-wise confidence intervals."),
    ("pairwise_ttest", "Explicit pairwise t-tests. Defaults to Welch tests with Holm correction; options select variance and adjustment."),
    ("cor", "Pearson correlation. Undefined when either input is constant, and NaN is returned; stats_relationship reports the same case as absent."),
];

const BUILTIN_EXAMPLES: &[(&str, &str, &str)] = &[
    (
        "gc_content",
        "dna\"ATCGCG\" |> gc_content()  # → 0.667",
        "Float",
    ),
    (
        "reverse_complement",
        "dna\"ATCG\" |> reverse_complement()  # → DNA(CGAT)",
        "DNA",
    ),
    (
        "transcribe",
        "dna\"ATCG\" |> transcribe()  # → RNA(AUCG)",
        "RNA",
    ),
    (
        "translate",
        "dna\"ATGCCC\" |> translate()  # → Protein(MP)",
        "Protein",
    ),
    ("mean", "[1, 2, 3, 4] |> mean()  # → 2.5", "Float"),
    ("median", "[1, 2, 3, 4, 5] |> median()  # → 3.0", "Float"),
    (
        "stdev",
        "[2, 4, 4, 4, 5, 5, 7, 9] |> stdev()  # → 2.0",
        "Float",
    ),
    ("sum", "[1, 2, 3] |> sum()  # → 6", "Float"),
    (
        "summary",
        "[1,2,3,4,5] |> summary()  # → {min, q1, median, mean, q3, max}",
        "Record",
    ),
    ("map", "[1,2,3] |> map(|x| x * 2)  # → [2, 4, 6]", "List"),
    (
        "filter",
        "[1,2,3,4] |> filter(|x| x > 2)  # → [3, 4]",
        "List",
    ),
    ("reduce", "[1,2,3] |> reduce(|a,b| a + b)  # → 6", "Value"),
    ("sort", "[3,1,2] |> sort()  # → [1, 2, 3]", "List"),
    ("unique", "[1,1,2,2,3] |> unique()  # → [1, 2, 3]", "List"),
    ("len", "[1,2,3] |> len()  # → 3", "Int"),
    (
        "join",
        "[\"a\",\"b\",\"c\"] |> join(\",\")  # → \"a,b,c\"",
        "Str",
    ),
    (
        "split",
        "\"a,b,c\" |> split(\",\")  # → [\"a\", \"b\", \"c\"]",
        "List",
    ),
    (
        "read_fasta",
        "read_fasta(\"seqs.fa\")  # → Table[id, description, seq, length]",
        "Table",
    ),
    (
        "read_fastq",
        "read_fastq(\"reads.fq\")  # → Table[id, description, seq, length, quality]",
        "Table",
    ),
    (
        "read_vcf",
        "read_vcf(\"variants.vcf\")  # → List[Variant]",
        "List[Variant]",
    ),
    ("read_bed", "read_bed(\"regions.bed\")  # → Table", "Table"),
    (
        "table",
        "table([{a: 1, b: 2}, {a: 3, b: 4}])  # → Table",
        "Table",
    ),
    (
        "hist",
        "[1,2,2,3,3,3] |> hist()  # → ASCII histogram",
        "Str",
    ),
    (
        "scatter",
        "scatter([1,2,3], [4,5,6])  # → ASCII scatter plot",
        "Str",
    ),
    (
        "ncbi_gene",
        "ncbi_gene(\"BRCA1\")  # → gene info record",
        "Record",
    ),
    (
        "ncbi_search",
        "ncbi_search(\"gene\", \"TP53\")  # → search results",
        "Record",
    ),
    (
        "ensembl_gene",
        "ensembl_gene(\"ENSG00000141510\")  # → gene info",
        "Record",
    ),
    (
        "uniprot_entry",
        "uniprot_entry(\"P04637\")  # → protein entry",
        "Record",
    ),
    (
        "uniprot_search",
        "uniprot_search(\"kinase AND organism_id:9606\")  # → results",
        "Record",
    ),
    (
        "pdb_entry",
        "pdb_entry(\"6LU7\")  # → structure info",
        "Record",
    ),
    (
        "kegg_get",
        "kegg_get(\"hsa:7157\")  # → KEGG entry text",
        "Str",
    ),
    (
        "go_term",
        "go_term(\"GO:0006915\")  # → GO term info",
        "Record",
    ),
    (
        "align",
        "align(dna\"ATCG\", dna\"ATGG\")  # → alignment record",
        "Record",
    ),
    (
        "kmer_count",
        "dna\"ATCGATCG\" |> kmer_count(3)  # → k-mer frequency table",
        "Table",
    ),
    (
        "lm",
        "lm([1,2,3], [2,4,6])  # → {slope: 2.0, r2: 1.0, ...}",
        "Record",
    ),
    ("ttest", "ttest([1,2,3], [4,5,9], {variance: \"welch\"})  # → method, p, CI, effect", "Record"),
    ("cor", "cor([1,2,3], [1,2,3])  # → 1.0", "Float"),
    (
        "p_adjust",
        "p_adjust([0.01, 0.04, 0.5], \"bh\")  # → adjusted p-values",
        "List",
    ),
    // Stats builtins that were catalogued with a return type but no example.
    (
        "variance",
        "variance([2.0, 4.0, 4.0, 6.0])  # → 2.6667 (sample, n-1)",
        "Float",
    ),
    (
        "quantile",
        "quantile([1.0, 2.0, 3.0, 4.0], 0.5)  # → 2.5 (type-7, as R)",
        "Float",
    ),
    ("cumsum", "cumsum([1, 2, 3])  # → [1, 3, 6]", "List"),
    (
        "normalize",
        "normalize([1.0, 2.0, 3.0], \"zscore\")  # → [-1.0, 0.0, 1.0]",
        "List",
    ),
    (
        "anova",
        "anova(groups, {variance: \"welch\"})  # → method, F, p, df, eta², omega²",
        "Record",
    ),
    (
        "kruskal_wallis",
        "kruskal_wallis(groups)  # → H, p, tie correction, epsilon²",
        "Record",
    ),
    (
        "tukey_hsd",
        "tukey_hsd(groups)  # → simultaneous all-pairs comparisons",
        "Record",
    ),
    (
        "pairwise_ttest",
        "pairwise_ttest(groups, {variance: \"welch\", adjust: \"holm\"})",
        "Record",
    ),
    (
        "chi_square",
        "chi_square([10, 20, 30], [20, 20, 20])  # → Record{chi2,p_value,df}",
        "Record{chi2,p_value,df}",
    ),
    (
        "fisher_exact",
        "fisher_exact(8, 2, 1, 5)  # → Record{p_value,odds_ratio,confidence_interval}",
        "Record{p_value,odds_ratio,confidence_interval}",
    ),
    (
        "wilcoxon",
        "wilcoxon([1.0,2.0,3.0], [4.0,5.0,6.0])  # → Record{u_statistic,p_value,effect_size}",
        "Record{u_statistic,p_value,effect_size}",
    ),
    (
        "wilcoxon_paired",
        "wilcoxon_paired(before, after, {method: \"normal\"})",
        "Record{v_statistic,p_value,effect_size}",
    ),
    (
        "ttest_one",
        "ttest_one([5.1, 4.9, 5.0], 5.0)  # → Record{statistic,p_value,df}",
        "Record{statistic,p_value,df}",
    ),
    (
        "ttest_paired",
        "ttest_paired([5.1, 4.9], [4.8, 4.6])  # → Record{statistic,p_value,df,mean_diff}",
        "Record{statistic,p_value,df,mean_diff}",
    ),
    // Guided exploration. Shown as the builtin rather than the `statistics`
    // package wrapper that also exposes it: the builtin is compiled into `bl`
    // and runs anywhere, while `import "statistics" as stat` first needs the
    // package installed, so a wrapper example fails on a clean machine. It is
    // also the name this help entry is titled with.
    (
        "stats_explore",
        "stats_explore([12.1, 12.4, 13.0, 29.0])  # → Record{kind: \"numeric\", ...}",
        "Record",
    ),
    (
        "stats_glm_diagnostics",
        "stats_glm_diagnostics(predictors, outcome, {family: \"binomial\"})  # check .converged first",
        "Record",
    ),
    (
        "stats_cox_diagnostics",
        "stats_cox_diagnostics(time, event, predictors)  # Breslow ties, as survival::coxph(ties=\"breslow\")",
        "Record",
    ),
    (
        "stats_random_intercept_model",
        "stats_random_intercept_model(predictors, outcome, subject_ids, {method: \"reml\"})",
        "Record",
    ),
    (
        "stats_uncertainty",
        "stats_uncertainty(values, {statistic: \"median\", seed: 42})  # seeded, and the seed is returned",
        "Record",
    ),
    (
        "stats_scan",
        "stats_scan(trial)  # profile, association screen and prioritised next steps",
        "Record",
    ),
    (
        "stats_means",
        "stats_means([2.0, 4.0, 8.0])  # → every mean paired with a compatible spread",
        "Record",
    ),
    (
        "stats_shape",
        "stats_shape(values)  # → skewness, kurtosis and Q-Q evidence; no diagnosis",
        "Record",
    ),
    (
        "stats_compare",
        "stats_compare(values, groups)  # → per-group evidence; no test is chosen",
        "Record",
    ),
    (
        "stats_relationship",
        "stats_relationship(x, y)  # → Pearson, Spearman and a fitted line",
        "Record",
    ),
    (
        "stats_categories",
        "stats_categories([\"red\", \"blue\", \"red\"])  # → levels, modes, rare-level clues",
        "Record",
    ),
    (
        "stats_guide",
        "stats_guide(report, {question: \"Does dose shift the median?\", experimental_unit: \"patient\"})",
        "Record",
    ),
    (
        "stats_preprocess",
        "stats_preprocess(values)  # → issues and suggestions; nothing is applied",
        "Record",
    ),
    (
        "stats_transform_preview",
        "stats_transform_preview(values, \"log\")  # → before and after; input unchanged",
        "Record",
    ),
    (
        "stats_distribution_clues",
        "stats_distribution_clues(counts)  # → four candidate families, none selected",
        "Record",
    ),
    (
        "stats_profile",
        "stats_profile(trial, {subject_column: \"patient\"})  # → columns, missingness, design",
        "Record",
    ),
    (
        "stats_missingness",
        "stats_missingness(trial, {group_column: \"arm\"})  # → by column, row, pair and group",
        "Record",
    ),
    (
        "stats_design_check",
        "stats_design_check(trial, {group_column: \"arm\", batch_column: \"run\"})  # blocking if every batch is one arm",
        "Record",
    ),
    (
        "stats_associations",
        "stats_associations(trial)  # → bounded effect sizes; no hypothesis tests",
        "Record",
    ),
    (
        "stats_report",
        "stats_report(trial, {format: \"html\"})  # → .content, .mime_type, .provenance",
        "Record",
    ),
    (
        "stats_normalization_guide",
        "stats_normalization_guide(counts_matrix)  # → audit and alternatives; nothing applied",
        "Record",
    ),
    (
        "stats_omics_profile",
        "stats_omics_profile(counts_matrix, {modality: \"single_cell\"})  # sparse stays sparse",
        "Record",
    ),
    (
        "stats_linear_diagnostics",
        "stats_linear_diagnostics(x, y)  # → residual clues and Cook distances",
        "Record",
    ),
    (
        "stats_multiple_linear_diagnostics",
        "stats_multiple_linear_diagnostics(predictors, y, {validation_folds: 4})",
        "Record",
    ),
    (
        "stats_robust_linear_diagnostics",
        "stats_robust_linear_diagnostics(predictors, y)  # Huber, as an explicit sensitivity check",
        "Record",
    ),
    (
        "stats_weighted_summary",
        "stats_weighted_summary(values, weights)  # → weighted mean and effective sample size",
        "Record",
    ),
    (
        "stats_time_series_diagnostics",
        "stats_time_series_diagnostics(series)  # → autocorrelations, Ljung-Box, trend",
        "Record",
    ),
    (
        "stats_cluster_diagnostics",
        "stats_cluster_diagnostics(values, clusters)  # → ICC and design effect",
        "Record",
    ),
    (
        "stats_decision_map",
        "stats_decision_map()  # → centre/spread/scale/uncertainty paths; chooses nothing",
        "Record",
    ),
    (
        "stats_explain",
        "stats_explain(report, \"audit\")  # detail: \"quick\", \"learning\" or \"audit\"",
        "Str",
    ),
    (
        "stats_overview_ascii",
        "stats_overview_ascii(trial)  # → terminal-safe whole-table summary",
        "Str",
    ),
    (
        "stats_distribution_plot",
        "stats_distribution_plot(values)  # → SVG histogram with mean, median, IQR and SD bands",
        "Str",
    ),
    (
        "stats_distribution_ascii",
        "stats_distribution_ascii(values)  # → terminal-safe histogram",
        "Str",
    ),
    (
        "stats_normal_diagram",
        "stats_normal_diagram()  # → teaching curve with 1/2/3-SD regions",
        "Str",
    ),
    (
        "stats_normal_qq_plot",
        "stats_normal_qq_plot(values)  # distinct from the genomic qq_plot()",
        "Str",
    ),
    (
        "stats_group_plot",
        "stats_group_plot(values, groups, {format: \"ascii\"})",
        "Str",
    ),
    (
        "stats_relationship_plot",
        "stats_relationship_plot(x, y)  # → scatterplot and fitted line",
        "Str",
    ),
    (
        "stats_categorical_plot",
        "stats_categorical_plot(labels)  # → frequency bars",
        "Str",
    ),
    (
        "stats_missingness_plot",
        "stats_missingness_plot(trial, {format: \"ascii\"})",
        "Str",
    ),
    (
        "stats_linear_diagnostic_plot",
        "stats_linear_diagnostic_plot(x, y, {view: \"qq\"})  # view: \"residuals\" or \"qq\"",
        "Str",
    ),
    (
        "stats_visualize",
        "stats_visualize(report, {format: \"ascii\"})",
        "Str",
    ),
    ("matrix", "matrix([[1,2],[3,4]])  # → 2x2 Matrix", "Matrix"),
    (
        "dot",
        "dot(matrix([[1,0],[0,1]]), matrix([[5],[6]]))  # → matmul",
        "Matrix",
    ),
    (
        "pca",
        "pca(matrix([[1,2],[3,4],[5,6]]), 2)  # → PCA result",
        "Record",
    ),
    (
        "glob",
        "glob(\"*.fasta\")  # → list of matching files",
        "List[Str]",
    ),
    (
        "shell",
        "shell(\"echo hello\")  # → {stdout, stderr, exit_code}",
        "Record",
    ),
    ("md5", "md5(\"hello\")  # → hex digest", "Str"),
    ("now", "now()  # → \"2024-01-15T10:30:00Z\"", "Str"),
    (
        "chat",
        "chat(\"explain GC content\")  # → LLM response",
        "Str",
    ),
    ("doctor", "doctor()  # → environment check table", "Table"),
    (
        "enrich",
        "enrich(genes, gene_sets, background)  # → enrichment table",
        "Table",
    ),
    (
        "diff_expr",
        "diff_expr(counts, [\"A\",\"A\",\"B\",\"B\"])  # → DE results",
        "Table",
    ),
    (
        "read_10x_sparse",
        "let cells = read_10x_sparse(\"filtered_feature_bc_matrix\")",
        "Record",
    ),
    (
        "cell_qc",
        "cell_qc(cells.matrix, cells.genes)  # -> Table",
        "Table",
    ),
    (
        "normalize_total",
        "normalize_total(cells.matrix, 10000.0) |> log1p_transform()",
        "SparseMatrix",
    ),
    (
        "sc_pca",
        "let pcs = sc_pca(log_counts, 30)  # pcs.scores is cells x components",
        "Record",
    ),
    ("knn_graph", "let edges = knn_graph(pcs.scores, 15)", "List"),
    (
        "leiden_graph",
        "leiden_graph(edges, cells.n_cells, 0.5)",
        "List",
    ),
];

fn cmd_fn_help_extended(name: &str) {
    let name = name.trim();
    // Find in catalog
    let entry = BUILTIN_CATALOG.iter().find(|(n, _, _)| *n == name);
    if entry.is_none() {
        if all_builtin_names().contains(&name) {
            println!("{CYAN}{name}(...){RESET}  {DIM}[runtime builtin; detailed metadata unavailable]{RESET}");
        } else {
            println!("{DIM}Unknown function: {name}. Try :builtins to browse.{RESET}");
        }
        return;
    }
    let (_, sig, cat) = entry.unwrap();
    let label = CATEGORIES
        .iter()
        .find(|(k, _)| k == cat)
        .map(|(_, l)| *l)
        .unwrap_or(cat);

    println!("{BOLD}{CYAN}{sig}{RESET}");
    println!("  {DIM}Category:{RESET} {label}");

    // Show example if available
    if let Some((_, example, ret)) = BUILTIN_EXAMPLES.iter().find(|(n, _, _)| *n == name) {
        println!("  {DIM}Returns:{RESET}  {ret}");
        println!();
        println!("  {BOLD}Example:{RESET}");
        println!("    {GREEN}{example}{RESET}");
    }
}

// ── Auto-detection (Phase B) ────────────────────────────────────

/// DB shorthand mappings: word → builtin function name
const DB_SHORTHANDS: &[(&str, &str)] = &[
    ("ncbi", "ncbi_gene"),
    ("gene", "ncbi_gene"),
    ("pdb", "pdb_entry"),
    ("uniprot", "uniprot_entry"),
    ("ensembl", "ensembl_gene"),
    ("kegg", "kegg_get"),
    ("go", "go_term"),
];

fn looks_like_raw_dna(s: &str) -> bool {
    s.len() > 10
        && !s.contains(' ')
        && !s.contains('"')
        && s.chars()
            .all(|c| matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'C' | 'G' | 'N'))
}

fn looks_like_fasta(s: &str) -> bool {
    let first_line = s.lines().next().unwrap_or("");
    first_line.starts_with('>')
        && first_line.len() > 1
        // Exclude `>=` operator
        && !first_line.starts_with(">=")
        && s.lines().count() >= 2
}

fn parse_fasta_input(s: &str) -> Option<String> {
    let mut lines = s.lines();
    let header = lines.next()?.strip_prefix('>')?;
    let mut seq = String::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        seq.push_str(t);
    }
    if seq.is_empty() {
        return None;
    }
    // Detect if it's protein or DNA
    let is_dna = seq
        .chars()
        .all(|c| matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'C' | 'G' | 'N'));
    let seq_expr = if is_dna {
        format!("dna\"{seq}\"")
    } else {
        format!("protein\"{seq}\"")
    };
    let id = header.split_whitespace().next().unwrap_or(header);
    Some(format!("{{id: \"{id}\", seq: {seq_expr}}}"))
}

fn looks_like_uniprot_id(s: &str) -> bool {
    // UniProt accession: [OPQ][0-9][A-Z0-9]{3}[0-9] or [A-NR-Z][0-9][A-Z][A-Z0-9]{2}[0-9]
    // Or entry name with underscore: P53_HUMAN
    let bytes = s.as_bytes();
    if s.contains('_') && s.len() >= 3 && s.len() <= 20 {
        return s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    }
    if bytes.len() == 6 || bytes.len() == 10 {
        let first = bytes[0] as char;
        let second = (bytes[1] as char).is_ascii_digit();
        if matches!(first, 'O' | 'P' | 'Q' | 'A'..='N' | 'R'..='Z') && second {
            return s[2..].chars().all(|c| c.is_ascii_alphanumeric());
        }
    }
    false
}

fn looks_like_pdb_id(s: &str) -> bool {
    // PDB ID: 4 characters, first is digit, rest alphanumeric
    s.len() == 4
        && s.as_bytes().first().is_some_and(|c| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn detect_db_shorthand(input: &str) -> Option<String> {
    // Pattern: `word "string"` or `word 'string'`
    let trimmed = input.trim();
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let word = parts[0].to_lowercase();
    let arg = parts[1].trim();

    // Check if arg is a quoted string
    let is_quoted = (arg.starts_with('"') && arg.ends_with('"'))
        || (arg.starts_with('\'') && arg.ends_with('\''));
    if !is_quoted {
        return None;
    }

    for (shorthand, func) in DB_SHORTHANDS {
        if word == *shorthand {
            let inner = &arg[1..arg.len() - 1];
            return Some(format!("{func}(\"{inner}\")"));
        }
    }
    None
}

fn detect_and_rewrite(input: String, _cache: &ApiCache) -> String {
    let trimmed = input.trim();

    // Skip if it contains assignment, pipe, or looks like normal code
    if trimmed.contains('=') && !trimmed.contains("==")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("match ")
        || trimmed.contains("|>")
        || trimmed.contains('(')
        || trimmed.contains('[')
        || trimmed.contains('{')
    {
        return input;
    }

    // DB shorthand: `ncbi "BRCA1"`, `pdb "6LU7"`, etc.
    if let Some(rewritten) = detect_db_shorthand(trimmed) {
        println!("{DIM}→ {rewritten}{RESET}");
        return rewritten;
    }

    // Single word checks (no spaces, no quotes)
    if !trimmed.contains(' ') && !trimmed.contains('"') {
        // Raw DNA paste
        if looks_like_raw_dna(trimmed) {
            let rewritten = format!("dna\"{trimmed}\"");
            println!("{DIM}→ {rewritten}{RESET}");
            return rewritten;
        }

        // PDB ID: 4 chars starting with digit
        if looks_like_pdb_id(trimmed) {
            let upper = trimmed.to_uppercase();
            let rewritten = format!("pdb_entry(\"{upper}\")");
            println!("{DIM}→ {rewritten}{RESET}");
            return rewritten;
        }

        // UniProt ID
        if looks_like_uniprot_id(trimmed) {
            let rewritten = format!("uniprot_entry(\"{trimmed}\")");
            println!("{DIM}→ {rewritten}{RESET}");
            return rewritten;
        }
    }

    // FASTA paste
    if looks_like_fasta(trimmed) {
        if let Some(rewritten) = parse_fasta_input(trimmed) {
            println!("{DIM}→ (parsed FASTA){RESET}");
            return rewritten;
        }
    }

    input
}

/// Check if an expression is an API shorthand call (used for caching decisions)
fn is_api_shorthand(input: &str) -> bool {
    let trimmed = input.trim();
    let api_prefixes = [
        "ncbi_gene(",
        "ncbi_search(",
        "ncbi_sequence(",
        "ensembl_gene(",
        "ensembl_vep(",
        "uniprot_search(",
        "uniprot_entry(",
        "pdb_entry(",
        "kegg_get(",
        "kegg_find(",
        "go_term(",
        "go_annotations(",
        "string_network(",
        "reactome_pathways(",
        "cosmic_gene(",
        "datasets_gene(",
    ];
    api_prefixes.iter().any(|p| trimmed.starts_with(p))
}

// ── API Cache (Phase C) ─────────────────────────────────────────

struct ApiCache {
    cache_dir: Option<PathBuf>,
    ttl: Duration,
}

impl ApiCache {
    fn new() -> Self {
        let cache_dir = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(|h| {
                let dir = PathBuf::from(h).join(".biolang").join("cache");
                let _ = std::fs::create_dir_all(&dir);
                dir
            });

        let ttl_secs: u64 = std::env::var("BIOLANG_CACHE_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(86400); // 24h

        Self {
            cache_dir,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    fn hash_key(&self, expr: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(expr.trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn store(&self, key: &str, value: &Value) {
        let Some(ref dir) = self.cache_dir else {
            return;
        };
        let json = value_to_json(value);
        let data_path = dir.join(format!("{key}.json"));
        let meta_path = dir.join(format!("{key}.meta"));
        let _ = std::fs::write(&data_path, serde_json::to_string(&json).unwrap_or_default());
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = std::fs::write(&meta_path, now.to_string());
    }

    #[allow(dead_code)]
    fn load(&self, key: &str) -> Option<Value> {
        let dir = self.cache_dir.as_ref()?;
        let meta_path = dir.join(format!("{key}.meta"));
        let data_path = dir.join(format!("{key}.json"));

        let ts: u64 = std::fs::read_to_string(&meta_path)
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now.saturating_sub(ts) > self.ttl.as_secs() {
            return None; // expired
        }

        let data = std::fs::read_to_string(&data_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&data).ok()?;
        Some(json_to_value(&json))
    }

    /// Load from cache ignoring TTL (for offline fallback)
    fn load_any(&self, key: &str) -> Option<(Value, String)> {
        let dir = self.cache_dir.as_ref()?;
        let meta_path = dir.join(format!("{key}.meta"));
        let data_path = dir.join(format!("{key}.json"));

        let ts: u64 = std::fs::read_to_string(&meta_path)
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let data = std::fs::read_to_string(&data_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&data).ok()?;

        // Format date from timestamp
        let date = format_unix_ts(ts);
        Some((json_to_value(&json), date))
    }
}

fn format_unix_ts(ts: u64) -> String {
    // Simple date formatting without chrono
    let secs_per_day: u64 = 86400;
    let days = ts / secs_per_day;
    // Approximate: days since 1970-01-01
    let year = 1970 + (days as f64 / 365.25) as u64;
    let day_of_year = days - ((year - 1970) as f64 * 365.25) as u64;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{year}-{month:02}-{day:02}")
}

fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::json!(*n),
        Value::Float(f) => serde_json::json!(*f),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => serde_json::Value::Array(items.iter().map(value_to_json).collect()),
        Value::Record(fields) => {
            let map: serde_json::Map<String, serde_json::Value> = fields
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Map(m) => {
            let map: serde_json::Map<String, serde_json::Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Table(t) => {
            let rows: Vec<serde_json::Value> = t
                .rows
                .iter()
                .map(|row| {
                    let map: serde_json::Map<String, serde_json::Value> = t
                        .columns
                        .iter()
                        .enumerate()
                        .map(|(i, col)| {
                            let v = row.get(i).cloned().unwrap_or(Value::Nil);
                            (col.clone(), value_to_json(&v))
                        })
                        .collect();
                    serde_json::Value::Object(map)
                })
                .collect();
            serde_json::Value::Array(rows)
        }
        Value::DNA(seq) => serde_json::Value::String(seq.data.clone()),
        Value::RNA(seq) => serde_json::Value::String(seq.data.clone()),
        Value::Protein(seq) => serde_json::Value::String(seq.data.clone()),
        _ => serde_json::Value::String(format!("{val}")),
    }
}

fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_to_value).collect::<Vec<_>>().into())
        }
        serde_json::Value::Object(map) => {
            let fields: HashMap<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect();
            Value::Record(fields.into())
        }
    }
}

fn value_preview(val: &Value) -> String {
    match val {
        Value::Nil => "nil".into(),
        Value::Bool(b) => format!("{b}"),
        Value::Int(n) => format!("{n}"),
        Value::Float(f) => format!("{f}"),
        Value::Str(s) if s.len() <= 30 => format!("\"{s}\""),
        Value::Str(s) => format!("\"{}...\"", &s[..27]),
        Value::List(items) => format!("[{} items]", items.len()),
        Value::Map(m) => format!("{{{} entries}}", m.len()),
        Value::Record(r) => format!("{{{} fields}}", r.len()),
        Value::Table(t) => format!("[{} x {}]", t.num_rows(), t.num_cols()),
        Value::DNA(seq) => format!("{}bp", seq.data.len()),
        Value::RNA(seq) => format!("{}nt", seq.data.len()),
        Value::Protein(seq) => format!("{}aa", seq.data.len()),
        Value::Function { params, .. } => {
            let ps: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            format!("fn({})", ps.join(", "))
        }
        Value::Stream(_) => "Stream(...)".into(),
        Value::Formula(_) => "~expr".into(),
        Value::Interval(iv) => format!("{}:{}-{}", iv.chrom, iv.start, iv.end),
        Value::NativeFunction { name, .. } => format!("<builtin {name}>"),
        Value::PluginFunction {
            plugin_name,
            operation,
            ..
        } => format!("<plugin:{plugin_name}.{operation}>"),
        Value::CompiledClosure(_) => "<compiled closure>".into(),
        Value::Matrix(m) => format!("Matrix({}x{})", m.nrow, m.ncol),
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
            enum_name, variant, ..
        } => format!("{enum_name}::{variant}"),
        Value::Set(items) => format!("#{{{} items}}", items.len()),
        Value::Regex { pattern, flags } => format!("/{pattern}/{flags}"),
        Value::Future(_) => "<future>".into(),
        Value::Kmer(km) => format!("Kmer({})", km.decode()),
        Value::SparseMatrix(sm) => format!("Sparse({}x{}, {} nnz)", sm.nrow, sm.ncol, sm.nnz()),
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(value_preview).collect();
            format!(
                "({}{})",
                parts.join(", "),
                if items.len() == 1 { "," } else { "" }
            )
        }
        Value::Gene { symbol, .. } => format!("Gene({symbol})"),
        Value::Variant { chrom, pos, .. } => format!("Variant({chrom}:{pos})"),
        Value::Genome { name, .. } => format!("Genome({name})"),
        Value::Quality(scores) => format!("Quality({}bp)", scores.len()),
        Value::AlignedRead(r) => format!("AlignedRead({} {}:{})", r.qname, r.rname, r.pos),
    }
}

// ── Built-in function catalog ────────────────────────────────────

/// (name, signature, category)
const BUILTIN_CATALOG: &[(&str, &str, &str)] = &[
    // Core
    ("print", "print(values...)", "core"),
    ("println", "println(values...)", "core"),
    ("len", "len(collection) → Int", "core"),
    ("type", "type(value) → Str", "core"),
    ("range", "range(start?, end, step?) → List", "core"),
    ("int", "int(value) → Int", "core"),
    ("float", "float(value) → Float", "core"),
    ("str", "str(value) → Str", "core"),
    ("bool", "bool(value) → Bool", "core"),
    ("assert", "assert condition, message?", "core"),
    ("debug", "debug(value)", "core"),
    ("env", "env(name) → Str|Nil", "core"),
    ("sleep", "sleep(ms)", "core"),
    (
        "doctor",
        "doctor() → Table (check env, containers, LLM, APIs)",
        "core",
    ),
    ("error", "error(message) → raises error", "core"),
    ("try_call", "try_call(fn) → {ok, value, error}", "core"),
    // List
    ("push", "push(list, item) → List", "list"),
    ("pop", "pop(list) → List", "list"),
    ("head", "head(list, n?) → List", "list"),
    ("tail", "tail(list, n?) → List", "list"),
    ("reverse", "reverse(list) → List", "list"),
    ("contains", "contains(list|str, item) → Bool", "list"),
    ("join", "join(list, sep?) → Str", "list"),
    ("split", "split(str, sep) → List", "list"),
    ("zip", "zip(list1, list2) → List[[a,b],...]", "list"),
    ("enumerate", "enumerate(list) → List[[i,v],...]", "list"),
    ("flatten", "flatten(list) → List", "list"),
    ("chunk", "chunk(list, size) → List[List,...]", "list"),
    ("slice", "slice(list|str, start, end?) → List|Str", "list"),
    ("concat", "concat(a, b) → List|Str", "list"),
    ("unique", "unique(list) → List", "list"),
    ("sample", "sample(list, n) → List", "list"),
    ("sort", "sort(list, cmp_fn?) → List", "list"),
    // HOF
    ("map", "map(list|table, fn) → List", "hof"),
    ("filter", "filter(list|table, fn) → List|Table", "hof"),
    ("reduce", "reduce(list, fn, init?) → Value", "hof"),
    ("any", "any(list, fn) → Bool", "hof"),
    ("all", "all(list, fn) → Bool", "hof"),
    ("find", "find(list, fn) → Value|Nil", "hof"),
    (
        "find_index",
        "find_index(list, fn) → Int (-1 if not found)",
        "hof",
    ),
    ("mutate", "mutate(table, col, fn) → Table", "hof"),
    ("summarize", "summarize(grouped, fn) → Table", "hof"),
    // String
    ("upper", "upper(str) → Str", "string"),
    ("lower", "lower(str) → Str", "string"),
    ("trim", "trim(str) → Str", "string"),
    ("trim_left", "trim_left(str) → Str", "string"),
    ("trim_right", "trim_right(str) → Str", "string"),
    ("starts_with", "starts_with(str, prefix) → Bool", "string"),
    ("ends_with", "ends_with(str, suffix) → Bool", "string"),
    ("str_replace", "str_replace(str, from, to) → Str", "string"),
    ("substr", "substr(str, start, len?) → Str", "string"),
    ("char_at", "char_at(str, index) → Str|Nil", "string"),
    (
        "index_of",
        "index_of(str, sub) → Int (-1 if not found)",
        "string",
    ),
    ("str_repeat", "str_repeat(str, n) → Str", "string"),
    ("pad_left", "pad_left(str, width, char) → Str", "string"),
    ("pad_right", "pad_right(str, width, char) → Str", "string"),
    ("str_len", "str_len(str) → Int (char count)", "string"),
    ("format", "format(template, args...) → Str", "string"),
    // Math
    ("abs", "abs(n) → number", "math"),
    ("min", "min(list|args...) → number", "math"),
    ("max", "max(list|args...) → number", "math"),
    ("sqrt", "sqrt(n) → Float", "math"),
    ("pow", "pow(base, exp) → Float", "math"),
    ("log", "log(n) → Float (natural)", "math"),
    ("log2", "log2(n) → Float", "math"),
    ("log10", "log10(n) → Float", "math"),
    ("exp", "exp(n) → Float", "math"),
    ("ceil", "ceil(n) → Int", "math"),
    ("floor", "floor(n) → Int", "math"),
    ("round", "round(n, digits?) → Float", "math"),
    ("sign", "sign(n) → -1|0|1", "math"),
    ("clamp", "clamp(val, min, max) → Float", "math"),
    ("sin", "sin(radians) → Float", "math"),
    ("cos", "cos(radians) → Float", "math"),
    ("tan", "tan(radians) → Float", "math"),
    ("asin", "asin(x) → Float", "math"),
    ("acos", "acos(x) → Float", "math"),
    ("atan", "atan(x) → Float", "math"),
    ("atan2", "atan2(y, x) → Float", "math"),
    ("pi", "pi() → 3.14159...", "math"),
    ("euler", "euler() → 2.71828...", "math"),
    ("random", "random() → Float [0,1)", "math"),
    ("random_int", "random_int(lo, hi) → Int [lo,hi)", "math"),
    ("is_nan", "is_nan(n) → Bool", "math"),
    ("is_finite", "is_finite(n) → Bool", "math"),
    // Stats
    ("mean", "mean(list) → Float", "stats"),
    ("median", "median(list) → Float", "stats"),
    ("stdev", "stdev(list) → Float", "stats"),
    ("variance", "variance(list) → Float", "stats"),
    ("sum", "sum(list) → Float", "stats"),
    ("quantile", "quantile(list, q) → Float", "stats"),
    ("cor", "cor(list1, list2) → Float", "stats"),
    ("cumsum", "cumsum(list) → List", "stats"),
    (
        "summary",
        "summary(list) → Record{count,min,median,mean,max,sd}",
        "stats",
    ),
    (
        "ttest",
        "ttest(list1, list2) → Record{statistic,p_value,df,mean_diff}",
        "stats",
    ),
    (
        "ttest_one",
        "ttest_one(list, mu) → Record{statistic,p_value,df}",
        "stats",
    ),
    (
        "ttest_paired",
        "ttest_paired(list1, list2) → Record{statistic,p_value,df,mean_diff}",
        "stats",
    ),
    (
        "anova",
        "anova(groups, options?) → Record{method,f_statistic,p_value,df_between,df_within,eta_squared,omega_squared}",
        "stats",
    ),
    (
        "kruskal_wallis",
        "kruskal_wallis(groups) → Record{h_statistic,p_value,epsilon_squared}",
        "stats",
    ),
    (
        "tukey_hsd",
        "tukey_hsd(groups, options?) → Record{comparisons,critical_value}",
        "stats",
    ),
    (
        "pairwise_ttest",
        "pairwise_ttest(groups, options?) → Record{adjustment,comparisons}",
        "stats",
    ),
    (
        "chi_square",
        "chi_square(obs, exp) → Record{chi2,p_value,df}",
        "stats",
    ),
    (
        "fisher_exact",
        "fisher_exact(a,b,c,d) → Record{p_value,odds_ratio,confidence_interval}",
        "stats",
    ),
    ("wilcoxon", "wilcoxon(list1, list2) → Record{u_statistic,p_value,effect_size}", "stats"),
    ("wilcoxon_paired", "wilcoxon_paired(before, after, options?) → Record{v_statistic,p_value,effect_size}", "stats"),
    ("p_adjust", "p_adjust(pvals, method) → List", "stats"),
    ("normalize", "normalize(list, method) → List", "stats"),
    // Guided exploration. These are the builtins the `statistics` package wraps;
    // scripts normally reach them as `stats_explore(...)` after
    // `import "statistics" as stat`. Every analysis here returns a Record
    // carrying `kind` and `schema` alongside the evidence, and every plot
    // returns a Str holding SVG or ASCII depending on `options.format`. None of
    // them modify their input or choose a test: `input_modified` and
    // `automatic_choice` are fields you can assert on.
    (
        "stats_explore",
        "stats_explore(values, options?) → Record{kind,summary,shape,outliers,alternatives,limitations}",
        "stats",
    ),
    (
        "stats_compare",
        "stats_compare(values, groups, options?) → Record{kind,groups,alternatives,limitations}",
        "stats",
    ),
    (
        "stats_relationship",
        "stats_relationship(x, y, options?) → Record{kind,pearson,spearman,slope,intercept}",
        "stats",
    ),
    (
        "stats_categories",
        "stats_categories(values, options?) → Record{kind,levels,modes,rare_levels}",
        "stats",
    ),
    (
        "stats_guide",
        "stats_guide(report, context?) → Record",
        "stats",
    ),
    (
        "stats_explain",
        "stats_explain(report, detail?) → Str",
        "stats",
    ),
    (
        "stats_shape",
        "stats_shape(values, options?) → Record{kind,evidence,sensitivity}",
        "stats",
    ),
    (
        "stats_uncertainty",
        "stats_uncertainty(values, options?) → Record{estimate,lower,upper,seed,method}",
        "stats",
    ),
    (
        "stats_means",
        "stats_means(values, options?) → Record{arithmetic_mean,geometric_mean,harmonic_mean,median,centre_spread_pairs}",
        "stats",
    ),
    (
        "stats_preprocess",
        "stats_preprocess(values, options?) → Record{issues,suggestions,automatic_changes}",
        "stats",
    ),
    (
        "stats_transform_preview",
        "stats_transform_preview(values, method, options?) → Record{before,after,input_modified}",
        "stats",
    ),
    (
        "stats_distribution_clues",
        "stats_distribution_clues(values, options?) → Record{candidates,model_selected}",
        "stats",
    ),
    (
        "stats_profile",
        "stats_profile(table, options?) → Record{columns,missingness,duplicate_rows,design}",
        "stats",
    ),
    (
        "stats_missingness",
        "stats_missingness(table, options?) → Record{columns,missing_by_row,co_missing}",
        "stats",
    ),
    (
        "stats_design_check",
        "stats_design_check(table, options?) → Record{groups,repeated_subjects,design_clues,issues}",
        "stats",
    ),
    (
        "stats_associations",
        "stats_associations(table, options?) → Record{pairs,high_association_pairs,threshold}",
        "stats",
    ),
    (
        "stats_scan",
        "stats_scan(table, options?) → Record{profile,associations,recommendations}",
        "stats",
    ),
    (
        "stats_report",
        "stats_report(table, options?) → Record{format,content,provenance}",
        "stats",
    ),
    (
        "stats_normalization_guide",
        "stats_normalization_guide(matrix, options?) → Record{data_type,suggestions,automatic_changes}",
        "stats",
    ),
    (
        "stats_omics_profile",
        "stats_omics_profile(matrix, options?) → Record{modality,suggestions,automatic_changes}",
        "stats",
    ),
    (
        "stats_linear_diagnostics",
        "stats_linear_diagnostics(x, y, options?) → Record{residual_mse,normal_qq_correlation,cook_distances}",
        "stats",
    ),
    (
        "stats_multiple_linear_diagnostics",
        "stats_multiple_linear_diagnostics(predictors, outcome, options?) → Record{coefficients,maximum_vif,validation_rmse}",
        "stats",
    ),
    (
        "stats_robust_linear_diagnostics",
        "stats_robust_linear_diagnostics(predictors, outcome, options?) → Record{coefficients,weights,converged}",
        "stats",
    ),
    (
        "stats_glm_diagnostics",
        "stats_glm_diagnostics(predictors, outcome, options?) → Record{coefficients,residual_deviance,aic,converged,iterations}",
        "stats",
    ),
    (
        "stats_random_intercept_model",
        "stats_random_intercept_model(predictors, outcome, clusters, options?) → Record{fixed_effects,random_intercept_variance,residual_variance,intraclass_correlation}",
        "stats",
    ),
    (
        "stats_cox_diagnostics",
        "stats_cox_diagnostics(time, event, predictors, options?) → Record{coefficients,partial_log_likelihood,likelihood_ratio,baseline_hazard,converged}",
        "stats",
    ),
    (
        "stats_weighted_summary",
        "stats_weighted_summary(values, weights, options?) → Record{weighted_mean,effective_sample_size}",
        "stats",
    ),
    (
        "stats_time_series_diagnostics",
        "stats_time_series_diagnostics(values, options?) → Record{autocorrelations,ljung_box_p_value,trend_per_observation}",
        "stats",
    ),
    (
        "stats_cluster_diagnostics",
        "stats_cluster_diagnostics(values, clusters, options?) → Record{intraclass_correlation,approximate_unequal_independence_design_effect}",
        "stats",
    ),
    (
        "stats_decision_map",
        "stats_decision_map(options?) → Record{paths,automatic_choice}",
        "stats",
    ),
    (
        "stats_distribution_plot",
        "stats_distribution_plot(values, options?) → Str",
        "stats",
    ),
    (
        "stats_distribution_ascii",
        "stats_distribution_ascii(values, options?) → Str",
        "stats",
    ),
    (
        "stats_normal_diagram",
        "stats_normal_diagram(values?, options?) → Str",
        "stats",
    ),
    (
        "stats_visualize",
        "stats_visualize(report, options?) → Str",
        "stats",
    ),
    (
        "stats_normal_qq_plot",
        "stats_normal_qq_plot(values, options?) → Str",
        "stats",
    ),
    (
        "stats_group_plot",
        "stats_group_plot(values, groups, options?) → Str",
        "stats",
    ),
    (
        "stats_relationship_plot",
        "stats_relationship_plot(x, y, options?) → Str",
        "stats",
    ),
    (
        "stats_categorical_plot",
        "stats_categorical_plot(values, options?) → Str",
        "stats",
    ),
    (
        "stats_missingness_plot",
        "stats_missingness_plot(table, options?) → Str",
        "stats",
    ),
    (
        "stats_linear_diagnostic_plot",
        "stats_linear_diagnostic_plot(x, y, options?) → Str",
        "stats",
    ),
    (
        "stats_overview_ascii",
        "stats_overview_ascii(table, options?) → Str",
        "stats",
    ),
    (
        "lm",
        "lm(x, y) → Record{slope,intercept,r_squared,p_value,std_error}",
        "stats",
    ),
    // Map/Record
    ("keys", "keys(map|record) → List", "map"),
    ("values", "values(map|record) → List", "map"),
    ("merge", "merge(a, b) → Map|Record", "map"),
    ("has_key", "has_key(map, key) → Bool", "map"),
    ("remove_key", "remove_key(map, key) → Map|Record", "map"),
    // Table
    ("table", "table(records) → Table", "table"),
    ("collect", "collect(stream) → List", "table"),
    ("count", "count(collection) → Int", "table"),
    ("take", "take(stream|list, n) → List", "table"),
    ("next", "next(stream) → Value|Nil", "table"),
    // FS
    ("read_text", "read_text(path) → Str", "fs"),
    ("write_text", "write_text(path, text)", "fs"),
    ("read_lines", "read_lines(path) → List[Str]", "fs"),
    ("write_lines", "write_lines(path, lines)", "fs"),
    ("append_text", "append_text(path, text)", "fs"),
    ("file_exists", "file_exists(path) → Bool", "fs"),
    ("is_dir", "is_dir(path) → Bool", "fs"),
    ("is_file", "is_file(path) → Bool", "fs"),
    ("file_size", "file_size(path) → Int (bytes)", "fs"),
    ("list_dir", "list_dir(path) → List[Record]", "fs"),
    ("mkdir", "mkdir(path)", "fs"),
    ("remove", "remove(path)", "fs"),
    ("copy_file", "copy_file(src, dst) → Str", "fs"),
    ("rename_file", "rename_file(src, dst) → Str", "fs"),
    ("basename", "basename(path) → Str", "fs"),
    ("dirname", "dirname(path) → Str", "fs"),
    ("extension", "extension(path) → Str", "fs"),
    ("path_join", "path_join(base, child) → Str", "fs"),
    ("abs_path", "abs_path(path) → Str", "fs"),
    ("glob", "glob(pattern) → List[Str]", "fs"),
    ("temp_file", "temp_file() → Str (path)", "fs"),
    ("temp_dir", "temp_dir() → Str (path)", "fs"),
    (
        "http_get",
        "http_get(url, headers?) → {status, body, headers}",
        "fs",
    ),
    (
        "http_post",
        "http_post(url, body, headers?) → {status, body, headers}",
        "fs",
    ),
    ("download", "download(url, path?) → {path, size, url}", "fs"),
    (
        "upload",
        "upload(path, url, headers?) → {status, size}",
        "fs",
    ),
    (
        "ref_genome",
        "ref_genome(name|\"list\", path?) → {path, name, description}",
        "fs",
    ),
    (
        "bio_fetch",
        "bio_fetch(name, path?) → {path, name, description, cached}",
        "fs",
    ),
    (
        "bio_sources",
        "bio_sources(category?) → Table of available data shortcuts",
        "fs",
    ),
    // Plot
    ("plot", "plot(table, opts?) → Str (SVG)", "plot"),
    ("heatmap", "heatmap(table, opts?) → Str (SVG)", "plot"),
    ("histogram", "histogram(list, opts?) → Str (SVG)", "plot"),
    ("volcano", "volcano(table, opts?) → Str (SVG)", "plot"),
    ("ma_plot", "ma_plot(table, opts?) → Str (SVG)", "plot"),
    ("save_svg", "save_svg(svg, path)", "plot"),
    (
        "save_png",
        "save_png(svg, path, opts?) → Str (path)",
        "plot",
    ),
    (
        "genome_track",
        "genome_track(table, opts?) → Str (SVG)",
        "plot",
    ),
    ("hist", "hist(list, bins?) → Str (ASCII)", "plot"),
    // SVG, not ASCII: scatter builds the same document plot() does. The comment
    // below claims these were verified against the implementations; this one was
    // not, and the wrong return type is the sort of thing a reader only
    // discovers by printing the first forty characters.
    ("scatter", "scatter(list1, list2) → Str (SVG)", "plot"),
    // Signatures verified against each implementation's argument handling.
    // Without these the metadata falls back to `name(arg1, arg2?)`, which made
    // the whole plot family undocumentable and invisible to the arity checker.
    ("umap_plot", "umap_plot(points, opts?) → Str (SVG)", "plot"),
    ("pca_plot", "pca_plot(points, opts?) → Str (SVG)", "plot"),
    // The single-cell figures. Each was added without an entry here, so each
    // was documented as `name(arg1, arg2?)` - which is how the whole plot
    // family became undocumentable the first time.
    (
        "feature_plot",
        "feature_plot(points, opts?) → Str (SVG)",
        "plot",
    ),
    (
        "elbow_plot",
        "elbow_plot(variance_ratios, opts?) → Str (SVG)",
        "plot",
    ),
    (
        "violin_plot",
        "violin_plot(data, opts?) → Str (SVG)",
        "plot",
    ),
    (
        "variable_feature_plot",
        "variable_feature_plot(matrix, opts?) → Str (SVG)",
        "plot",
    ),
    (
        "dot_plot",
        "dot_plot(matrix, clusters, opts?) → Str (SVG)",
        "plot",
    ),
    ("violin", "violin(data, opts?) → Str (SVG)", "plot"),
    (
        "clustered_heatmap",
        "clustered_heatmap(data, opts?) → Str (SVG)",
        "plot",
    ),
    ("density", "density(data, opts?) → Str (SVG)", "plot"),
    ("boxplot", "boxplot(data, opts?) → Str (ASCII)", "plot"),
    ("sparkline", "sparkline(data) → Str (ASCII)", "plot"),
    (
        "heatmap_ascii",
        "heatmap_ascii(table, opts?) → Str (ASCII)",
        "plot",
    ),
    (
        "quality_plot",
        "quality_plot(quality, opts?) → Str (ASCII)",
        "plot",
    ),
    (
        "bar_chart",
        "bar_chart(labels_to_values) → Nil (prints ASCII)",
        "plot",
    ),
    // Sequence dot-matrix, NOT the expression dot plot of scanpy/Seurat.
    (
        "dotplot",
        "dotplot(seq1, seq2, opts?) → Str (sequence dot-matrix)",
        "plot",
    ),
    // Matrix
    ("matrix", "matrix(nested_lists) → Matrix", "matrix"),
    ("zeros", "zeros(nrow, ncol) → Matrix", "matrix"),
    ("eye", "eye(n) → Matrix (identity)", "matrix"),
    ("dim", "dim(matrix) → [nrow, ncol]", "matrix"),
    ("transpose", "transpose(matrix) → Matrix", "matrix"),
    ("dot", "dot(a, b) → Matrix (matmul)", "matrix"),
    (
        "pca",
        "pca(data, [n]) → Record{explained_variance,...}",
        "data",
    ),
    ("cor_matrix", "cor_matrix(matrix) → Matrix", "matrix"),
    // Enrichment
    ("read_gmt", "read_gmt(path) → Map{set→genes}", "enrich"),
    ("enrich", "enrich(genes, sets, bg) → Table", "enrich"),
    ("gsea", "gsea(ranked, sets) → Table", "enrich"),
    // Bio
    ("dna", "dna\"ATCG\" → DNA sequence", "bio"),
    (
        "reverse_complement",
        "reverse_complement(seq) → DNA/RNA",
        "bio",
    ),
    ("transcribe", "transcribe(dna) → RNA", "bio"),
    ("translate", "translate(rna|dna) → Protein", "bio"),
    ("gc_content", "gc_content(seq) → Float", "bio"),
    ("read_fasta", "read_fasta(path) → Table", "bio"),
    (
        "fasta_stats",
        "fasta_stats(path) → Record{count,total_bp,mean_length,n50,...}",
        "bio",
    ),
    ("read_fastq", "read_fastq(path) → Table", "bio"),
    ("read_bed", "read_bed(path) → Table", "bio"),
    ("read_gff", "read_gff(path) → Table", "bio"),
    ("read_vcf", "read_vcf(path) → List[Variant]", "bio"),
    (
        "write_fasta",
        "write_fasta(records, path) → Int (count)",
        "bio",
    ),
    (
        "write_fastq",
        "write_fastq(records, path) → Int (count)",
        "bio",
    ),
    (
        "write_bed",
        "write_bed(records|table, path) → Int (count)",
        "bio",
    ),
    ("write_vcf", "write_vcf(records, path) → Int (count)", "bio"),
    ("write_gff", "write_gff(records, path) → Int (count)", "bio"),
    (
        "validate",
        "validate(path) → {valid, format, errors, lines_checked}",
        "bio",
    ),
    (
        "vcf_filter",
        "vcf_filter(path, expr) → Table (e.g. \"QUAL > 30 && DP > 10\")",
        "bio",
    ),
    (
        "align",
        "align(seq1, seq2, mode?, match?, mismatch?, gap?) → Record",
        "bio",
    ),
    ("edit_distance", "edit_distance(s1, s2) → Int", "bio"),
    ("hamming_distance", "hamming_distance(s1, s2) → Int", "bio"),
    (
        "msa",
        "msa(sequences, opts?) → Record{sequences, names, n_seqs, length}",
        "bio",
    ),
    (
        "distance_matrix",
        "distance_matrix(alignment, opts?) → Matrix (pairwise distances)",
        "bio",
    ),
    (
        "conservation_scores",
        "conservation_scores(alignment) → List[Float] (per-column 0-1)",
        "bio",
    ),
    (
        "interval",
        "interval(chrom, start, end, strand?) → Interval",
        "bio",
    ),
    // API
    (
        "ncbi_search",
        "ncbi_search(db, query, max?) → List[Record]",
        "api",
    ),
    ("ncbi_gene", "ncbi_gene(query) → Record", "api"),
    ("ensembl_gene", "ensembl_gene(id) → Record", "api"),
    ("uniprot_entry", "uniprot_entry(accession) → Record", "api"),
    (
        "uniprot_search",
        "uniprot_search(query, max?) → List[Record]",
        "api",
    ),
    ("pdb_entry", "pdb_entry(id) → Record", "api"),
    ("kegg_get", "kegg_get(id) → Str", "api"),
    ("go_term", "go_term(id) → Record", "api"),
    // Hash
    ("md5", "md5(str) → Str (hex)", "hash"),
    ("sha256", "sha256(str) → Str (hex)", "hash"),
    ("sha512", "sha512(str) → Str (hex)", "hash"),
    ("crc32", "crc32(str) → Int", "hash"),
    ("hmac_sha256", "hmac_sha256(data, key) → Str (hex)", "hash"),
    ("base64_encode", "base64_encode(str) → Str", "hash"),
    ("base64_decode", "base64_decode(str) → Str", "hash"),
    (
        "sketch",
        "sketch(seq, k?, n?) → List (MinHash sketch)",
        "hash",
    ),
    (
        "sketch_dist",
        "sketch_dist(a, b) → Float (Jaccard distance 0–1)",
        "hash",
    ),
    // DateTime
    ("now", "now() → Str (ISO 8601 UTC)", "datetime"),
    (
        "timestamp",
        "timestamp() → Int (Unix epoch seconds)",
        "datetime",
    ),
    (
        "timestamp_ms",
        "timestamp_ms() → Int (Unix epoch ms)",
        "datetime",
    ),
    (
        "date_format",
        "date_format(date_str, fmt) → Str",
        "datetime",
    ),
    (
        "date_parse",
        "date_parse(str, fmt) → Str (ISO 8601)",
        "datetime",
    ),
    (
        "date_add",
        "date_add(date_str, amount, unit) → Str",
        "datetime",
    ),
    (
        "date_diff",
        "date_diff(date1, date2, unit) → Int",
        "datetime",
    ),
    ("year", "year(date_str) → Int", "datetime"),
    ("month", "month(date_str) → Int", "datetime"),
    ("day", "day(date_str) → Int", "datetime"),
    ("weekday", "weekday(date_str) → Str", "datetime"),
    // Text processing
    ("grep", "grep(input, pattern, flags?) → List", "text"),
    ("grep_count", "grep_count(input, pattern) → Int", "text"),
    ("lines", "lines(text) → List[Str]", "text"),
    ("cut", "cut(text, delimiter, fields) → List", "text"),
    ("paste", "paste(list1, list2, sep?) → List[Str]", "text"),
    (
        "uniq_count",
        "uniq_count(list) → List[{value, count}]",
        "text",
    ),
    ("wc", "wc(input) → {lines, words, chars, bytes}", "text"),
    ("tee", "tee(value, path) → value (writes to file)", "text"),
    (
        "shell",
        "shell(cmd, stdin?) → {stdout, stderr, exit_code}",
        "text",
    ),
    ("count_lines", "count_lines(path) → Int", "text"),
    (
        "stream_lines",
        "stream_lines(path) → Stream (lazy file reader)",
        "text",
    ),
    (
        "stream_concat",
        "stream_concat(a, b) → Stream (lazy concat)",
        "text",
    ),
    // Type predicates
    ("is_nil", "is_nil(value) → Bool", "type"),
    ("is_int", "is_int(value) → Bool", "type"),
    ("is_float", "is_float(value) → Bool", "type"),
    ("is_num", "is_num(value) → Bool", "type"),
    ("is_str", "is_str(value) → Bool", "type"),
    ("is_bool", "is_bool(value) → Bool", "type"),
    ("is_list", "is_list(value) → Bool", "type"),
    ("is_map", "is_map(value) → Bool", "type"),
    ("is_record", "is_record(value) → Bool", "type"),
    ("is_table", "is_table(value) → Bool", "type"),
    ("is_function", "is_function(value) → Bool", "type"),
    ("is_dna", "is_dna(value) → Bool", "type"),
    ("is_rna", "is_rna(value) → Bool", "type"),
    ("is_protein", "is_protein(value) → Bool", "type"),
    ("is_interval", "is_interval(value) → Bool", "type"),
    ("is_matrix", "is_matrix(value) → Bool", "type"),
    ("is_stream", "is_stream(value) → Bool", "type"),
    ("is_range", "is_range(value) → Bool", "type"),
    ("is_enum", "is_enum(value) → Bool", "type"),
    ("is_set", "is_set(value) → Bool", "type"),
    ("is_regex", "is_regex(value) → Bool", "type"),
    ("is_future", "is_future(value) → Bool", "type"),
    // Container
    (
        "container_available",
        "container_available() → {runtime, version, image_dir}",
        "container",
    ),
    (
        "container_run",
        "container_run(image, cmd, opts?) → {stdout, stderr, exit_code}",
        "container",
    ),
    (
        "container_pull",
        "container_pull(image) → {image, storage, hint}",
        "container",
    ),
    (
        "tool",
        "tool(name, cmd, opts?) → {stdout, stderr, exit_code}",
        "container",
    ),
    (
        "tool_search",
        "tool_search(query, opts?) → List[{name, pulls, versions, ...}]",
        "container",
    ),
    (
        "tool_popular",
        "tool_popular(limit?) → List (sorted by Quay popularity)",
        "container",
    ),
    (
        "tool_info",
        "tool_info(name) → {name, pulls, versions: [...]}",
        "container",
    ),
    (
        "tool_pull",
        "tool_pull(name, version?) → {image, storage, hint}",
        "container",
    ),
    ("tool_list", "tool_list() → List[Str]", "container"),
    (
        "tool_available",
        "tool_available() → {runtime, version, image_dir}",
        "container",
    ),
    // LLM
    (
        "chat",
        "chat(message, context?) → Str (LLM response)",
        "llm",
    ),
    (
        "chat_code",
        "chat_code(description, context?) → Str (BioLang code)",
        "llm",
    ),
    (
        "llm_models",
        "llm_models() → {provider, model, env_vars}",
        "llm",
    ),
    // Units
    ("bp", "bp(n) → Record{value, unit} (base pairs)", "bio"),
    ("kb", "kb(n) → Record{value, unit} (kilobases)", "bio"),
    ("mb", "mb(n) → Record{value, unit} (megabases)", "bio"),
    ("gb", "gb(n) → Record{value, unit} (gigabases)", "bio"),
    // Generators
    ("help", "help(fn) → Nil (print function docs)", "core"),
    ("gen_int", "gen_int(max | min, max?, seed?) → Int", "core"),
    ("gen_float", "gen_float() → Float in [0, 1)", "core"),
    ("gen_str", "gen_str(len?) → Str (random lowercase)", "core"),
    // Parallel + property testing
    ("par_map", "par_map(list, fn) → List (parallel map)", "hof"),
    (
        "par_filter",
        "par_filter(list, fn) → List (parallel filter)",
        "hof",
    ),
    (
        "prop_test",
        "prop_test(property_fn, generator_fn, iters) → Record",
        "hof",
    ),
    // Transfer protocols
    (
        "ftp_download",
        "ftp_download(url, path?) → {path, size}",
        "transfer",
    ),
    ("ftp_list", "ftp_list(url) → List[{name, path}]", "transfer"),
    ("ftp_upload", "ftp_upload(path, url) → {size}", "transfer"),
    (
        "sftp_download",
        "sftp_download(url, path?) → {path, size}",
        "transfer",
    ),
    ("sftp_upload", "sftp_upload(path, url) → {size}", "transfer"),
    ("scp", "scp(source, dest) → {source, dest}", "transfer"),
    (
        "s3_download",
        "s3_download(s3_url, path?) → {path, size}",
        "transfer",
    ),
    ("s3_upload", "s3_upload(path, s3_url) → {size}", "transfer"),
    (
        "s3_list",
        "s3_list(s3_url, recursive?) → List[{name, size}]",
        "transfer",
    ),
    (
        "gcs_download",
        "gcs_download(gs_url, path?) → {path, size}",
        "transfer",
    ),
    (
        "gcs_upload",
        "gcs_upload(path, gs_url) → {size}",
        "transfer",
    ),
    (
        "rsync",
        "rsync(source, dest, opts?) → {source, dest}",
        "transfer",
    ),
    (
        "aspera_download",
        "aspera_download(url, path?) → {path}",
        "transfer",
    ),
    (
        "sra_prefetch",
        "sra_prefetch(accession, path?) → {path, accession}",
        "transfer",
    ),
    (
        "sra_fastq",
        "sra_fastq(accession, path?) → {files, accession}",
        "transfer",
    ),
    // Set operations
    ("set", "set(list) → Set (deduped)", "list"),
    ("union", "union(set1, set2) → Set", "list"),
    ("intersection", "intersection(set1, set2) → Set", "list"),
    ("difference", "difference(set1, set2) → Set", "list"),
    (
        "symmetric_difference",
        "symmetric_difference(set1, set2) → Set",
        "list",
    ),
    ("is_subset", "is_subset(a, b) → Bool", "list"),
    ("is_superset", "is_superset(a, b) → Bool", "list"),
    // Async
    (
        "await_all",
        "await_all(futures) → List (resolve all)",
        "hof",
    ),
    // Decorators
    ("memoize", "memoize(fn) → fn (cached results)", "hof"),
    ("time_it", "time_it(fn) → fn (prints elapsed time)", "hof"),
    ("once", "once(fn) → fn (execute only first call)", "hof"),
    // Genomic range queries
    (
        "interval_tree",
        "interval_tree(table) → Record (sorted intervals per chrom)",
        "bio",
    ),
    (
        "query_overlaps",
        "query_overlaps(tree, chrom, start, end) → Table",
        "bio",
    ),
    (
        "query_nearest",
        "query_nearest(tree, chrom, pos, k?) → Table",
        "bio",
    ),
    (
        "coverage",
        "coverage(tree) → Table{chrom, start, end, depth}",
        "bio",
    ),
    // Sequence pattern matching
    (
        "motif_find",
        "motif_find(seq, iupac_pattern) → List[{start, end, match}]",
        "bio",
    ),
    (
        "motif_count",
        "motif_count(seq, iupac_pattern) → Int",
        "bio",
    ),
    ("consensus", "consensus(sequences) → Str", "bio"),
    (
        "pwm",
        "pwm(sequences) → List[{A, C, G, T}] (position weight matrix)",
        "bio",
    ),
    (
        "pwm_scan",
        "pwm_scan(seq, pwm, threshold?) → List[{pos, score}]",
        "bio",
    ),
    // Pipeline
    (
        "pipeline_steps",
        "pipeline_steps(pipeline) → Table{step, name, plugin, params, depends_on}",
        "bio",
    ),
    // GAP 1: Coordinate systems
    (
        "coord_bed",
        "coord_bed(val) → Record with __coord_system: 'bed'",
        "coord",
    ),
    (
        "coord_vcf",
        "coord_vcf(val) → Record with __coord_system: 'vcf'",
        "coord",
    ),
    (
        "coord_gff",
        "coord_gff(val) → Record with __coord_system: 'gff'",
        "coord",
    ),
    (
        "coord_sam",
        "coord_sam(val) → Record with __coord_system: 'sam'",
        "coord",
    ),
    (
        "coord_convert",
        "coord_convert(val, to_system) → Record (converted coordinates)",
        "coord",
    ),
    (
        "coord_system",
        "coord_system(val) → Str (current coord system)",
        "coord",
    ),
    (
        "coord_check",
        "coord_check(a, b) → Bool (are coord systems compatible?)",
        "coord",
    ),
    // GAP 2: K-mers
    (
        "kmer_encode",
        "kmer_encode(seq, k) → Kmer or List[Kmer]",
        "kmer",
    ),
    ("kmer_decode", "kmer_decode(kmer) → Str", "kmer"),
    (
        "kmer_rc",
        "kmer_rc(kmer) → Kmer (reverse complement)",
        "kmer",
    ),
    (
        "kmer_canonical",
        "kmer_canonical(kmer) → Kmer (canonical form)",
        "kmer",
    ),
    (
        "kmer_count",
        "kmer_count(seq, k, top_n?) → Table|Stream{kmer, count}",
        "kmer",
    ),
    (
        "kmer_distinct",
        "kmer_distinct(seq, k) → Int (distinct k-mer count)",
        "kmer",
    ),
    (
        "kmer_spectrum",
        "kmer_spectrum(counts) → Table{frequency, count}",
        "kmer",
    ),
    (
        "minimizers",
        "minimizers(seq, k, w) → List[{kmer, pos}]",
        "kmer",
    ),
    // GAP 3: Streaming
    (
        "stream_chunks",
        "stream_chunks(stream, n) → Stream of List (chunks of n)",
        "stream",
    ),
    (
        "stream_take",
        "stream_take(stream, n) → List (first n items)",
        "stream",
    ),
    (
        "stream_skip",
        "stream_skip(stream, n) → Stream (skip first n)",
        "stream",
    ),
    (
        "stream_batch",
        "stream_batch(stream, n, fn) → List (process in batches)",
        "stream",
    ),
    (
        "memory_usage",
        "memory_usage() → Record{heap_bytes, ...}",
        "stream",
    ),
    // GAP 4: Parallel
    (
        "scatter_by",
        "scatter_by(list, key_fn) → Map{key → List}",
        "hof",
    ),
    (
        "bench",
        "bench(fn, args, n) → Record{mean_ns, min_ns, max_ns, iterations}",
        "hof",
    ),
    // GAP 5: Sparse matrix
    (
        "sparse_matrix",
        "sparse_matrix(data | nrow, ncol, entries) → SparseMatrix",
        "sparse",
    ),
    ("to_dense", "to_dense(sparse) → Matrix", "sparse"),
    ("to_sparse", "to_sparse(matrix) → SparseMatrix", "sparse"),
    ("sparse_get", "sparse_get(m, i, j) → Float", "sparse"),
    ("nnz", "nnz(m) → Int (non-zero count)", "sparse"),
    (
        "sparse_row_sums",
        "sparse_row_sums(m) → List[Float]",
        "sparse",
    ),
    (
        "sparse_col_sums",
        "sparse_col_sums(m) → List[Float]",
        "sparse",
    ),
    (
        "normalize_sparse",
        "normalize_sparse(m, method) → SparseMatrix ('log1p_cpm'|'scale')",
        "sparse",
    ),
    // Single-cell RNA-seq
    (
        "read_10x",
        "read_10x(path, gene_column?) → Record (dense)",
        "singlecell",
    ),
    (
        "read_10x_sparse",
        "read_10x_sparse(path, gene_column?) → Record (CSR counts, obs, var, layers)",
        "singlecell",
    ),
    (
        "read_anndata",
        "read_anndata(zarr_path) → Record (native dense or CSR AnnData Zarr)",
        "singlecell",
    ),
    (
        "write_anndata",
        "write_anndata(zarr_path, object) → Nil (preserves CSR sparsity)",
        "singlecell",
    ),
    (
        "normalize_total",
        "normalize_total(matrix, target?) → matrix (row library-size normalization)",
        "singlecell",
    ),
    (
        "log1p_transform",
        "log1p_transform(matrix) → matrix (preserves CSR sparsity)",
        "singlecell",
    ),
    (
        "highly_variable_genes",
        "highly_variable_genes(matrix, n?) → List[Int] (dispersion-ranked columns)",
        "singlecell",
    ),
    (
        "cca",
        "cca(matrix1, matrix2, opts?) → Record{u, v, d} \
         (shared axes; cells x cells, so small inputs only)",
        "singlecell",
    ),
    (
        "harmony_integrate",
        "harmony_integrate(embedding, batches, opts?) → matrix \
         (batch-corrected, per-cluster)",
        "singlecell",
    ),
    (
        "find_all_markers",
        "find_all_markers(matrix, clusters, opts?) → List[Record] \
         (gene, cluster, p_value, p_adj, avg_log2fc, pct_1, pct_2)",
        "singlecell",
    ),
    (
        "cell_qc",
        "cell_qc(matrix, gene_names?, mito_prefix?) → Table",
        "singlecell",
    ),
    (
        "gene_qc",
        "gene_qc(matrix, gene_names?) → Table",
        "singlecell",
    ),
    (
        "select_rows",
        "select_rows(matrix|table, indices) → matrix|table",
        "singlecell",
    ),
    (
        "select_cols",
        "select_cols(matrix, indices) → matrix",
        "singlecell",
    ),
    (
        "matrix_at",
        "matrix_at(matrix, row, column) → Value",
        "singlecell",
    ),
    (
        "sc_subset_cells",
        "sc_subset_cells(object, indices) → Record (synchronized metadata/layers)",
        "singlecell",
    ),
    (
        "sc_subset_genes",
        "sc_subset_genes(object, indices) → Record (invalidates reductions)",
        "singlecell",
    ),
    (
        "sc_merge_objects",
        "sc_merge_objects(left, right, left_batch, right_batch) → Record",
        "singlecell",
    ),
    (
        "sc_pca",
        "sc_pca(matrix, n_components?) → {scores, loadings, explained_variance_ratio, ...}",
        "singlecell",
    ),
    (
        "knn_graph",
        "knn_graph(embedding, k?) → List[{source, target, distance}]",
        "singlecell",
    ),
    (
        "leiden_cluster",
        "leiden_cluster(embedding, k, resolution?) → List[Int]",
        "singlecell",
    ),
    (
        "leiden_graph",
        "leiden_graph(edges, n_nodes, resolution) → List[Int]",
        "singlecell",
    ),
    (
        "doublet_score",
        "doublet_score(matrix, n_simulated?) → List[Float]",
        "singlecell",
    ),
    (
        "cell_cycle_score",
        "cell_cycle_score(matrix, s_gene_indices, g2m_gene_indices) → List[Record]",
        "singlecell",
    ),
    (
        "module_score",
        "module_score(matrix, gene_indices) → List[Float]",
        "singlecell",
    ),
    (
        "sc_sctransform",
        "sc_sctransform(matrix, n_variable_features?) → matrix | {matrix, genes}",
        "singlecell",
    ),
    (
        "sc_integrate",
        "sc_integrate(embedding, batch_ids) → matrix",
        "singlecell",
    ),
    (
        "diffusion_pseudotime",
        "diffusion_pseudotime(embedding, edges, start_cell) → List[Float]",
        "singlecell",
    ),
    (
        "lr_score",
        "lr_score(matrix, labels, ligand_receptor_pairs) → List[Record]",
        "singlecell",
    ),
    (
        "lr_aggregate",
        "lr_aggregate(scores, pathway_map) → List[Record]",
        "singlecell",
    ),
    (
        "spatial_neighbors",
        "spatial_neighbors(coordinates, k?) → List[Record]",
        "singlecell",
    ),
    (
        "spatial_moransi",
        "spatial_moransi(expression, edges) → Float",
        "singlecell",
    ),
    (
        "reference_classify",
        "reference_classify(query, reference_profiles, labels) → List[Record]",
        "singlecell",
    ),
    (
        "pseudobulk_aggregate",
        "pseudobulk_aggregate(matrix, sample_ids, groups) → Record",
        "singlecell",
    ),
    (
        "wnn_graph",
        "wnn_graph(rna_edges, protein_edges, rna_weight) → List[Record]",
        "singlecell",
    ),
    (
        "velocity_estimate",
        "velocity_estimate(spliced, unspliced) → matrix",
        "singlecell",
    ),
    // GAP 6: Typed table columns
    (
        "table_col_types",
        "table_col_types(table) → Record{col → type_str}",
        "table",
    ),
    (
        "table_set_col_type",
        "table_set_col_type(table, col, type) → Record{table, schema}",
        "table",
    ),
    (
        "table_validate",
        "table_validate(schema_record) → Record{valid, errors}",
        "table",
    ),
    (
        "table_schema",
        "table_schema(table) → Record{columns, types, nrow, ncol}",
        "table",
    ),
    (
        "table_cast",
        "table_cast(table, col, type) → Table (coerce column)",
        "table",
    ),
    // GAP 7: Pipe fusion
    (
        "pipe_fuse",
        "pipe_fuse(list, ops...) → List (explicit fused pipeline)",
        "hof",
    ),
    // GAP 8: Provenance
    (
        "with_provenance",
        "with_provenance(value, meta) → Record{__value, __provenance}",
        "provenance",
    ),
    (
        "provenance",
        "provenance(wrapped) → Record or Nil (extract provenance)",
        "provenance",
    ),
    (
        "provenance_chain",
        "provenance_chain(wrapped) → List (walk parent chain)",
        "provenance",
    ),
    (
        "checkpoint",
        "checkpoint(name, value) → value (save to disk)",
        "provenance",
    ),
    (
        "resume_checkpoint",
        "resume_checkpoint(name) → value or Nil",
        "provenance",
    ),
    // GAP 10: Bio operations
    (
        "de_bruijn_graph",
        "de_bruijn_graph(sequences, k) → Record{nodes, edges}",
        "bio",
    ),
    (
        "neighbor_joining",
        "neighbor_joining(distance_matrix) → List[{name, distance, children}]",
        "bio",
    ),
    (
        "umap",
        "umap(matrix, n_components, opts?) → Matrix (embeddings)",
        "bio",
    ),
    (
        "tsne",
        "tsne(matrix, n_components, opts?) → Matrix (embeddings)",
        "bio",
    ),
    (
        "leiden",
        "leiden(adjacency, resolution?) → List[Int] (cluster assignments)",
        "bio",
    ),
    (
        "diff_expr",
        "diff_expr(counts, groups) → Table{gene, log2fc, pvalue, padj, mean_a, mean_b}",
        "bio",
    ),
    // Type predicates for new types
    ("is_kmer", "is_kmer(value) → Bool", "type"),
    ("is_sparse", "is_sparse(value) → Bool", "type"),
];

const CATEGORIES: &[(&str, &str)] = &[
    ("core", "Core"),
    ("list", "List"),
    ("hof", "Higher-Order (map/filter/...)"),
    ("string", "String"),
    ("math", "Math"),
    ("stats", "Statistics"),
    ("map", "Map/Record"),
    ("table", "Table/Stream"),
    ("fs", "Filesystem"),
    ("plot", "Plotting"),
    ("matrix", "Matrix"),
    ("enrich", "Enrichment"),
    ("bio", "Bio (sequences, I/O)"),
    ("api", "Bio APIs"),
    ("hash", "Hashing/Encoding"),
    ("datetime", "Date/Time"),
    ("text", "Text Processing"),
    ("type", "Type Predicates"),
    ("container", "Containers (Docker/Podman/BioContainers)"),
    ("llm", "LLM Chat (Anthropic/OpenAI/Ollama)"),
    ("transfer", "Transfer (FTP/SFTP/S3/GCS/rsync/Aspera/SRA)"),
    ("coord", "Coordinate Systems (BED/VCF/GFF/SAM)"),
    ("kmer", "K-mer Analysis"),
    ("sparse", "Sparse Matrix"),
    ("singlecell", "Single-Cell RNA-seq"),
    ("stream", "Streaming"),
    ("provenance", "Data Provenance"),
];

fn cmd_builtins(filter: &str) {
    let filter = filter.trim().to_lowercase();
    if filter.is_empty() {
        // Show category summary
        println!("{BOLD}Built-in function categories:{RESET}");
        for (key, label) in CATEGORIES {
            let count = BUILTIN_CATALOG.iter().filter(|(_, _, c)| c == key).count();
            println!("  {CYAN}{key:<12}{RESET} {label} ({count} functions)");
        }
        let runtime_count = all_builtin_names().len();
        let documented_count = BUILTIN_CATALOG.len();
        println!();
        println!(
            "{DIM}{documented_count} functions have detailed REPL metadata; {runtime_count} runtime built-ins are available.{RESET}"
        );
        println!("{DIM}Use :builtins <category> to list functions, e.g. :builtins stats{RESET}");
        println!("{DIM}Use ?name to show signature, e.g. ?mean{RESET}");
        return;
    }
    // Find matching category or search by name
    let matches: Vec<_> = BUILTIN_CATALOG
        .iter()
        .filter(|(name, _, cat)| cat.contains(filter.as_str()) || name.contains(filter.as_str()))
        .collect();
    let mut runtime_matches: Vec<_> = all_builtin_names()
        .into_iter()
        .filter(|name| name.contains(filter.as_str()))
        .filter(|name| !BUILTIN_CATALOG.iter().any(|(n, _, _)| n == name))
        .collect();
    runtime_matches.sort_unstable();
    runtime_matches.dedup();

    if matches.is_empty() && runtime_matches.is_empty() {
        println!("{DIM}No functions matching '{filter}'. Try :builtins for categories.{RESET}");
        return;
    }
    // Group by category
    let mut by_cat: std::collections::BTreeMap<&str, Vec<(&str, &str)>> =
        std::collections::BTreeMap::new();
    for (name, sig, cat) in &matches {
        by_cat.entry(cat).or_default().push((name, sig));
    }
    for (cat, fns) in &by_cat {
        let label = CATEGORIES
            .iter()
            .find(|(k, _)| k == cat)
            .map(|(_, l)| *l)
            .unwrap_or(cat);
        println!("{BOLD}{label}:{RESET}");
        for (_, sig) in fns {
            println!("  {CYAN}{sig}{RESET}");
        }
        println!();
    }
    if !runtime_matches.is_empty() {
        println!("{BOLD}Runtime built-ins without detailed REPL metadata:{RESET}");
        for name in runtime_matches {
            println!("  {CYAN}{name}(...){RESET}");
        }
        println!();
    }
}

fn cmd_fn_help(name: &str) {
    let name = name.trim();
    for (n, sig, cat) in BUILTIN_CATALOG {
        if *n == name {
            let label = CATEGORIES
                .iter()
                .find(|(k, _)| k == cat)
                .map(|(_, l)| *l)
                .unwrap_or(cat);
            println!("{CYAN}{sig}{RESET}  {DIM}[{label}]{RESET}");
            return;
        }
    }
    if all_builtin_names().contains(&name) {
        println!("{CYAN}{name}(...){RESET}  {DIM}[runtime builtin; detailed metadata unavailable]{RESET}");
    } else {
        println!("{DIM}Unknown function: {name}. Try :builtins to browse.{RESET}");
    }
}

fn fn_signature(name: &str) -> Option<&'static str> {
    for (n, sig, _) in BUILTIN_CATALOG {
        if *n == name {
            return Some(sig);
        }
    }
    None
}

// ── Continuation detection ──────────────────────────────────────

/// Check if the input needs continuation (unclosed delimiters or trailing pipe).
/// Returns true if the completed input could plausibly be extended with `|>`.
/// Only returns true when there's already a pipe in progress, so that
/// normal one-liners like `let x = 5` execute immediately.
///
/// Multi-line pipe chains are written with the pipe on the first line:
///   let passed = variants |>
///     filter(|v| v.filter == "PASS") |>
///     map(|v| v.chrom)
///
/// Or with a trailing backslash:
///   let passed = variants \
///     |> filter(|v| v.filter == "PASS")
fn could_continue_with_pipe(input: &str) -> bool {
    // If we already have a multi-line pipe chain, keep accepting more |> lines
    let line_count = input.lines().count();
    if line_count >= 2 && input.contains("|>") {
        // The input already has pipe(s) across lines — peek for more
        let last_line = input.trim_end().lines().last().unwrap_or("");
        let last = if let Some(idx) = last_line.find('#') {
            last_line[..idx].trim_end()
        } else {
            last_line.trim_end()
        };
        let last_ch = last.chars().last().unwrap_or(' ');
        return last_ch.is_alphanumeric() || last_ch == '_' || matches!(last_ch, ')' | ']' | '}');
    }
    false
}

/// Extract the variable name from a `let name = ...` statement.
fn extract_let_var(input: &str) -> Option<String> {
    let trimmed = input.trim();
    // Check for `let varname = ...`
    if trimmed.starts_with("let ") {
        let rest = trimmed[4..].trim_start();
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    // Check for `... |> into varname`
    if let Some(pos) = trimmed.rfind("|> into ") {
        let rest = trimmed[pos + 8..].trim();
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn needs_continuation(input: &str) -> bool {
    let mut parens = 0i32;
    let mut braces = 0i32;
    let mut brackets = 0i32;
    let mut in_string = false;
    // Brackets inside a `#` comment are prose, not syntax. Counting them meant a
    // line like
    //     let a = 1  # TODO: wrap this in a { block
    // left the REPL waiting for a closing brace that was never coming: it then
    // swallowed every following line as continuation, including `:quit`, and
    // eventually failed to parse the lot. Comments run to end of line, so the
    // flag resets on the newline. String literals were already handled.
    let mut in_comment = false;
    let mut prev_char = '\0';

    for ch in input.chars() {
        if ch == '\n' {
            in_comment = false;
            prev_char = ch;
            continue;
        }
        if in_comment {
            prev_char = ch;
            continue;
        }
        if in_string {
            if ch == '"' && prev_char != '\\' {
                in_string = false;
            }
        } else {
            match ch {
                '#' => in_comment = true,
                '"' => in_string = true,
                '(' => parens += 1,
                ')' => parens -= 1,
                '{' => braces += 1,
                '}' => braces -= 1,
                '[' => brackets += 1,
                ']' => brackets -= 1,
                _ => {}
            }
        }
        prev_char = ch;
    }

    if parens > 0 || braces > 0 || brackets > 0 {
        return true;
    }

    // Check for trailing pipe operator or trailing binary operators
    let trimmed = input.trim_end();
    if trimmed.ends_with("|>") || trimmed.ends_with("|>>") {
        return true;
    }

    // Check if the last non-comment line ends with a token that expects continuation
    // (e.g. `+`, `-`, `*`, `/`, `&&`, `||`, `==`, `,`, `=`, `and`, `or`, `|>`)
    let last_line = trimmed.lines().last().unwrap_or("").trim();
    let last_line_no_comment = if let Some(idx) = last_line.find('#') {
        last_line[..idx].trim_end()
    } else {
        last_line
    };
    if last_line_no_comment.ends_with("and")
        || last_line_no_comment.ends_with("or")
        || last_line_no_comment.ends_with('+')
        || last_line_no_comment.ends_with('\\')
    {
        return true;
    }

    // Block constructs without an opening brace are incomplete.
    // e.g. `for x in list` needs `{ ... }`, `fn foo(x)` needs `{ body }`.
    let first = trimmed.lines().next().unwrap_or("").trim();
    let has_brace = braces_seen(input);
    if (first.starts_with("for ")
        || first.starts_with("while ")
        || first.starts_with("fn ")
        || first.starts_with("match "))
        && !has_brace
    {
        return true;
    }
    // `if` can use either `{ }` blocks or `then` keyword
    if first.starts_with("if ") && !has_brace && !input.contains("then") {
        return true;
    }

    false
}

/// Returns true if the input contains at least one `{` outside of strings.
fn braces_seen(input: &str) -> bool {
    let mut in_string = false;
    let mut prev = '\0';
    for ch in input.chars() {
        if in_string {
            if ch == '"' && prev != '\\' {
                in_string = false;
            }
        } else {
            if ch == '"' {
                in_string = true;
            }
            if ch == '{' {
                return true;
            }
        }
        prev = ch;
    }
    false
}

/// Heuristic: returns true if the input looks like an expression whose result
/// could be piped into a `|>` on the next line.  Returns false for statements
/// (assignments, definitions, control flow, imports) that execute for side
/// effects and produce no meaningful pipeable value.
// ── Colored Value Display ────────────────────────────────────────

/// Maximum column width for table display.
const MAX_COL_WIDTH: usize = 40;
/// Maximum rows shown before truncation.
const MAX_TABLE_ROWS: usize = 20;

fn print_colored_value(value: &Value) {
    match value {
        Value::Table(t) => print_table(t),
        _ => println!("{}", colorize_value(value)),
    }
}

fn colorize_value(value: &Value) -> String {
    match value {
        Value::Nil => format!("{DIM}nil{RESET}"),
        Value::Bool(b) => format!("{YELLOW}{b}{RESET}"),
        Value::Int(n) => format!("{CYAN}{n}{RESET}"),
        Value::Float(f) => format!("{CYAN}{f}{RESET}"),
        Value::Str(s) => format!("{GREEN}\"{s}\"{RESET}"),
        Value::DNA(seq) => {
            let colored_bases = colorize_bases(&seq.data, false);
            format!("{BOLD}DNA({RESET}{colored_bases}{BOLD}){RESET}")
        }
        Value::RNA(seq) => {
            let colored_bases = colorize_bases(&seq.data, true);
            format!("{BOLD}RNA({RESET}{colored_bases}{BOLD}){RESET}")
        }
        Value::Protein(seq) => {
            let colored_aa = colorize_protein(&seq.data);
            format!("{BOLD}Protein({RESET}{colored_aa}{BOLD}){RESET}")
        }
        Value::Interval(iv) => format!("{BLUE}{iv}{RESET}"),
        Value::List(items) => {
            if items.is_empty() {
                return format!("{DIM}[]{RESET}");
            }
            if items.len() <= 10 {
                let parts: Vec<String> = items.iter().map(colorize_value).collect();
                format!("[{}]", parts.join(", "))
            } else {
                let parts: Vec<String> = items[..10].iter().map(colorize_value).collect();
                format!(
                    "[{}, {DIM}... {} more{RESET}]",
                    parts.join(", "),
                    items.len() - 10
                )
            }
        }
        Value::Record(fields) => {
            if fields.len() > 3 {
                // Pretty-print with indentation
                let mut out = String::from("{\n");
                for (k, v) in fields.iter() {
                    out.push_str(&format!("  {BOLD}{k}{RESET}: {},\n", colorize_value(v)));
                }
                out.push('}');
                out
            } else {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{BOLD}{k}{RESET}: {}", colorize_value(v)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
        }
        Value::Map(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{BOLD}{k}{RESET}: {}", colorize_value(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Matrix(m) => format!("{BLUE}{m}{RESET}"),
        Value::Stream(s) => format!("{DIM}<stream {}>{RESET}", s.label),
        Value::Function { name, .. } => {
            format!(
                "{DIM}<fn {}>{RESET}",
                name.as_deref().unwrap_or("anonymous")
            )
        }
        Value::NativeFunction { name, .. } => format!("{DIM}<builtin {name}>{RESET}"),
        Value::Formula(_) => format!("{DIM}<formula>{RESET}"),
        Value::PluginFunction {
            plugin_name,
            operation,
            ..
        } => format!("{DIM}<plugin:{plugin_name}.{operation}>{RESET}"),
        Value::CompiledClosure(_) => format!("{DIM}<compiled closure>{RESET}"),
        Value::Table(t) => format!("{DIM}Table: {} x {}{RESET}", t.num_rows(), t.num_cols()),
        Value::Range {
            start,
            end,
            inclusive,
        } => {
            if *inclusive {
                format!("{CYAN}{start}..={end}{RESET}")
            } else {
                format!("{CYAN}{start}..{end}{RESET}")
            }
        }
        Value::EnumValue {
            enum_name,
            variant,
            fields,
        } => {
            if fields.is_empty() {
                format!("{MAGENTA}{enum_name}::{variant}{RESET}")
            } else {
                let args: Vec<String> = fields.iter().map(colorize_value).collect();
                format!(
                    "{MAGENTA}{enum_name}::{variant}{RESET}({})",
                    args.join(", ")
                )
            }
        }
        Value::Set(items) => {
            let parts: Vec<String> = items.iter().take(10).map(colorize_value).collect();
            if items.len() > 10 {
                format!(
                    "#{{{}, {DIM}... {} more{RESET}}}",
                    parts.join(", "),
                    items.len() - 10
                )
            } else {
                format!("#{{{}}}", parts.join(", "))
            }
        }
        Value::Regex { pattern, flags } => format!("{GREEN}/{pattern}/{flags}{RESET}"),
        Value::Future(_) => format!("{DIM}<future>{RESET}"),
        Value::Kmer(km) => format!("{MAGENTA}Kmer({}{RESET}{MAGENTA}){RESET}", km.decode()),
        Value::SparseMatrix(sm) => format!("{CYAN}{sm}{RESET}"),
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(colorize_value).collect();
            format!(
                "({}{})",
                parts.join(", "),
                if items.len() == 1 { "," } else { "" }
            )
        }
        Value::Gene { symbol, .. } => format!("{MAGENTA}Gene({symbol}){RESET}"),
        Value::Variant { chrom, pos, .. } => format!("{MAGENTA}Variant({chrom}:{pos}){RESET}"),
        Value::Genome { name, .. } => format!("{MAGENTA}Genome({name}){RESET}"),
        Value::Quality(scores) => format!("{BLUE}Quality({}bp){RESET}", scores.len()),
        Value::AlignedRead(r) => format!(
            "{MAGENTA}AlignedRead({} {}:{}){RESET}",
            r.qname, r.rname, r.pos
        ),
    }
}

/// Get terminal width from env or fallback.
fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
}

fn print_table(t: &Table) {
    if t.columns.is_empty() || t.rows.is_empty() {
        println!("{DIM}(empty table){RESET}");
        return;
    }

    let ncols = t.columns.len();
    let show_rows = t.rows.len().min(MAX_TABLE_ROWS);
    let available_width = term_width();

    // Format all cell values as strings
    let mut col_cells: Vec<Vec<String>> = Vec::with_capacity(ncols);
    for ci in 0..ncols {
        let mut cells = Vec::with_capacity(show_rows + 1);
        cells.push(t.columns[ci].clone());
        for ri in 0..show_rows {
            let val = t.rows[ri].get(ci).cloned().unwrap_or(Value::Nil);
            cells.push(format!("{val}"));
        }
        col_cells.push(cells);
    }

    // Compute natural column widths
    let natural_widths: Vec<usize> = col_cells
        .iter()
        .map(|cells| {
            cells
                .iter()
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(3)
                .max(3)
        })
        .collect();

    // Adjust widths to fit terminal: borders take 1 + (3 * ncols) + 1 chars
    let border_overhead = 2 + 3 * ncols;
    let max_content = available_width.saturating_sub(border_overhead);
    let total_natural: usize = natural_widths.iter().sum();
    let widths: Vec<usize> = if total_natural <= max_content {
        natural_widths
            .iter()
            .map(|w| (*w).min(MAX_COL_WIDTH))
            .collect()
    } else {
        // Proportionally shrink columns to fit, min 3 chars each
        natural_widths
            .iter()
            .map(|w| {
                let scaled = (*w as f64 / total_natural as f64 * max_content as f64) as usize;
                scaled.max(3).min(MAX_COL_WIDTH)
            })
            .collect()
    };

    // Detect numeric columns (right-align them)
    let is_numeric: Vec<bool> = (0..ncols)
        .map(|ci| {
            t.rows[..show_rows].iter().all(|row| {
                matches!(
                    row.get(ci).unwrap_or(&Value::Nil),
                    Value::Int(_) | Value::Float(_) | Value::Nil
                )
            })
        })
        .collect();

    // Helper: truncate and pad a cell (returns plain text, no ANSI)
    let pad = |s: &str, width: usize, right_align: bool| -> String {
        let chars: Vec<char> = s.chars().collect();
        let display = if chars.len() > width {
            let truncated: String = chars[..width.saturating_sub(1)].iter().collect();
            format!("{truncated}…")
        } else {
            s.to_string()
        };
        let len = display.chars().count();
        if right_align && len < width {
            format!("{}{display}", " ".repeat(width - len))
        } else if len < width {
            format!("{display}{}", " ".repeat(width - len))
        } else {
            display
        }
    };

    // Build horizontal lines
    let line_parts: Vec<String> = widths.iter().map(|w| "─".repeat(*w + 2)).collect();
    let top_line = format!("┌{}┐", line_parts.join("┬"));
    let mid_line = format!("├{}┤", line_parts.join("┼"));
    let bot_line = format!("└{}┘", line_parts.join("┴"));

    // Print header info
    println!("{DIM}Table: {} rows × {} cols{RESET}", t.rows.len(), ncols);

    // Top border
    println!("{DIM}{top_line}{RESET}");

    // Header row — underline text only, pad with plain spaces
    let header_cells: Vec<String> = (0..ncols)
        .map(|ci| {
            let text = &t.columns[ci];
            let chars: Vec<char> = text.chars().collect();
            let w = widths[ci];
            let display = if chars.len() > w {
                let truncated: String = chars[..w.saturating_sub(1)].iter().collect();
                format!("{truncated}…")
            } else {
                text.to_string()
            };
            let len = display.chars().count();
            let padding = if len < w {
                " ".repeat(w - len)
            } else {
                String::new()
            };
            format!("{BOLD}{UNDERLINE}{display}{RESET}{padding}")
        })
        .collect();
    println!(
        "{DIM}│{RESET} {} {DIM}│{RESET}",
        header_cells.join(&format!(" {DIM}│{RESET} "))
    );

    // Separator
    println!("{DIM}{mid_line}{RESET}");

    // Data rows
    for ri in 0..show_rows {
        let row_cells: Vec<String> = (0..ncols)
            .map(|ci| {
                let val = t.rows[ri].get(ci).cloned().unwrap_or(Value::Nil);
                let raw = format!("{val}");
                let padded = pad(&raw, widths[ci], is_numeric[ci]);
                colorize_cell(&val, &padded)
            })
            .collect();
        println!(
            "{DIM}│{RESET} {} {DIM}│{RESET}",
            row_cells.join(&format!(" {DIM}│{RESET} "))
        );
    }

    // Bottom border
    println!("{DIM}{bot_line}{RESET}");

    // Truncation notice
    if t.rows.len() > MAX_TABLE_ROWS {
        println!(
            "{DIM}  … {} more rows{RESET}",
            t.rows.len() - MAX_TABLE_ROWS
        );
    }
}

/// Colorize a single table cell based on value type.
fn colorize_cell(val: &Value, text: &str) -> String {
    match val {
        Value::Nil => format!("{DIM}{text}{RESET}"),
        Value::Bool(_) => format!("{YELLOW}{text}{RESET}"),
        Value::Int(_) | Value::Float(_) => format!("{CYAN}{text}{RESET}"),
        Value::Str(_) => format!("{GREEN}{text}{RESET}"),
        Value::DNA(_) | Value::RNA(_) => format!("{BOLD}{CYAN}{text}{RESET}"),
        Value::Protein(_) => format!("{BOLD}{YELLOW}{text}{RESET}"),
        _ => text.to_string(),
    }
}

/// Colorize individual DNA/RNA bases: A=green, T/U=red, G=yellow, C=blue, N=dim.
fn colorize_bases(seq: &str, is_rna: bool) -> String {
    const A_COLOR: &str = "\x1b[32m"; // green
    const T_COLOR: &str = "\x1b[31m"; // red
    const G_COLOR: &str = "\x1b[33m"; // yellow
    const C_COLOR: &str = "\x1b[34m"; // blue
                                      // Truncate long sequences for display
    let max_display = 80;
    let truncated = seq.len() > max_display;
    let display_seq = if truncated { &seq[..max_display] } else { seq };

    let mut out = String::with_capacity(display_seq.len() * 10);
    for ch in display_seq.chars() {
        match ch.to_ascii_uppercase() {
            'A' => {
                out.push_str(A_COLOR);
                out.push(ch);
                out.push_str(RESET);
            }
            'T' if !is_rna => {
                out.push_str(T_COLOR);
                out.push(ch);
                out.push_str(RESET);
            }
            'U' if is_rna => {
                out.push_str(T_COLOR);
                out.push(ch);
                out.push_str(RESET);
            }
            'G' => {
                out.push_str(G_COLOR);
                out.push(ch);
                out.push_str(RESET);
            }
            'C' => {
                out.push_str(C_COLOR);
                out.push(ch);
                out.push_str(RESET);
            }
            _ => {
                out.push_str(DIM);
                out.push(ch);
                out.push_str(RESET);
            }
        }
    }
    if truncated {
        out.push_str(&format!("{DIM}…({} more){RESET}", seq.len() - max_display));
    }
    out
}

/// Colorize protein residues by biochemical property:
/// Hydrophobic (AILMFWV) = yellow, Positive charge (RHK) = red,
/// Negative charge (DE) = blue, Polar (STNQYC) = green, Special (GP) = magenta
fn colorize_protein(seq: &str) -> String {
    const HYDROPHOBIC: &str = "\x1b[33m"; // yellow
    const POS_CHARGE: &str = "\x1b[31m"; // red
    const NEG_CHARGE: &str = "\x1b[34m"; // blue
    const POLAR: &str = "\x1b[32m"; // green
    const SPECIAL: &str = "\x1b[35m"; // magenta

    let max_display = 80;
    let truncated = seq.len() > max_display;
    let display_seq = if truncated { &seq[..max_display] } else { seq };

    let mut out = String::with_capacity(display_seq.len() * 10);
    for ch in display_seq.chars() {
        let color = match ch.to_ascii_uppercase() {
            'A' | 'I' | 'L' | 'M' | 'F' | 'W' | 'V' => HYDROPHOBIC,
            'R' | 'H' | 'K' => POS_CHARGE,
            'D' | 'E' => NEG_CHARGE,
            'S' | 'T' | 'N' | 'Q' | 'Y' | 'C' => POLAR,
            'G' | 'P' => SPECIAL,
            _ => DIM,
        };
        out.push_str(color);
        out.push(ch);
        out.push_str(RESET);
    }
    if truncated {
        out.push_str(&format!("{DIM}…({} more){RESET}", seq.len() - max_display));
    }
    out
}

fn dirs_history_path() -> Option<String> {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let dir = format!("{home}/.biolang");
        let _ = std::fs::create_dir_all(&dir);
        Some(format!("{dir}/history"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn console_requests_retain_state_and_report_user_environment() {
        let mut interpreter = Interpreter::new();
        let binding = evaluate_console_request(1, "let x = 42", &mut interpreter);
        assert_eq!(binding.status, "ok");
        assert!(binding.value.is_none());
        assert_eq!(binding.environment.variables.len(), 1);
        assert_eq!(binding.environment.variables[0].name, "x");
        assert_eq!(binding.environment.variables[0].preview, "42");

        let expression = evaluate_console_request(2, "x * 2", &mut interpreter);
        assert_eq!(expression.status, "ok");
        let value = expression.value.expect("expression value");
        assert_eq!(value.type_name, "Int");
        assert_eq!(value.text, "84");
    }

    #[test]
    fn console_requests_capture_output_and_structured_errors() {
        let mut interpreter = Interpreter::new();
        let output = evaluate_console_request(1, "println(\"ready\")", &mut interpreter);
        assert_eq!(output.status, "ok");
        assert_eq!(output.output, "ready\n");

        let failure = evaluate_console_request(2, "unknown_name + 1", &mut interpreter);
        assert_eq!(failure.status, "error");
        assert!(failure
            .error
            .as_deref()
            .is_some_and(|error| error.contains("unknown_name")));
    }

    #[test]
    fn common_completion_finds_the_unambiguous_prefix() {
        let mut h = BioHelper::new();
        h.words = ["cluster", "cluster_leiden", "clusters", "normalize"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        h.words.sort();

        // one match -> completes fully
        let (c, n) = h.common_completion("norm").expect("should match");
        assert_eq!((c.as_str(), n), ("alize", 1));

        // several matches sharing a longer prefix -> completes to the shared part
        let (c, n) = h.common_completion("clu").expect("should match");
        assert_eq!(c, "ster");
        assert_eq!(n, 3);

        // ambiguous with nothing further in common -> empty completion, count kept
        let (c, n) = h.common_completion("cluster").expect("should match");
        assert_eq!(c, "");
        assert_eq!(n, 3);

        assert!(h.common_completion("zzz").is_none());
    }

    #[test]
    fn repl_command_list_and_hints_stay_in_sync() {
        // Every hinted command must be completable, or Tab and ghost text disagree.
        for (cmd, _) in REPL_COMMAND_HINTS {
            assert!(
                REPL_COMMANDS.contains(cmd),
                "{cmd} is hinted but missing from REPL_COMMANDS"
            );
        }
        for cmd in [":workspace", ":restore"] {
            assert!(REPL_COMMANDS.contains(&cmd), "{cmd} not registered");
            assert!(
                REPL_COMMAND_HINTS.iter().any(|(c, _)| *c == cmd),
                "{cmd} has no hint"
            );
        }
    }

    #[test]
    fn catalog_and_examples_only_reference_registered_builtins() {
        let runtime: HashSet<_> = all_builtin_names().into_iter().collect();

        let missing_catalog: Vec<_> = BUILTIN_CATALOG
            .iter()
            .map(|(name, _, _)| *name)
            .filter(|name| !runtime.contains(name))
            .collect();
        assert!(
            missing_catalog.is_empty(),
            "REPL catalog references unregistered builtins: {missing_catalog:?}"
        );

        let missing_examples: Vec<_> = BUILTIN_EXAMPLES
            .iter()
            .map(|(name, _, _)| *name)
            .filter(|name| !runtime.contains(name))
            .collect();
        assert!(
            missing_examples.is_empty(),
            "REPL examples reference unregistered builtins: {missing_examples:?}"
        );

        let uncatalogued_examples: Vec<_> = BUILTIN_EXAMPLES
            .iter()
            .map(|(name, _, _)| *name)
            .filter(|name| {
                !BUILTIN_CATALOG
                    .iter()
                    .any(|(catalog_name, _, _)| catalog_name == name)
            })
            .collect();
        assert!(
            uncatalogued_examples.is_empty(),
            "REPL examples missing from the catalog: {uncatalogued_examples:?}"
        );
    }

    #[test]
    fn completion_includes_every_registered_builtin() {
        let helper = BioHelper::new();
        for name in all_builtin_names() {
            assert!(
                helper.words.iter().any(|word| word == name),
                "registered builtin '{name}' is missing from REPL completion"
            );
        }
    }

    #[test]
    fn reverse_complement_metadata_uses_runtime_name() {
        assert_eq!(
            fn_signature("reverse_complement"),
            Some("reverse_complement(seq) → DNA/RNA")
        );
        assert_eq!(fn_signature("rev_comp"), None);
        assert!(BUILTIN_EXAMPLES
            .iter()
            .any(|(name, example, _)| *name == "reverse_complement"
                && example.contains("reverse_complement()")));
    }

    #[test]
    fn structured_metadata_covers_every_registered_builtin() {
        let document = biolang_metadata();
        let runtime = all_builtin_names();
        assert_eq!(document.schema_version, 1);
        assert_eq!(document.builtins.len(), runtime.len());
        assert!(document
            .builtins
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name));
        for name in runtime {
            assert!(
                document.builtins.iter().any(|builtin| builtin.name == name),
                "metadata is missing registered builtin '{name}'"
            );
        }
        let add_chr = document
            .builtins
            .iter()
            .find(|builtin| builtin.name == "add_chr")
            .expect("add_chr metadata");
        assert_eq!(add_chr.arity.minimum, 1);
        assert_eq!(add_chr.arity.maximum, Some(1));
        assert_eq!(add_chr.signature, "add_chr(arg1)");
    }

    // ── multi-line continuation ─────────────────────────────────────────────
    //
    // needs_continuation() decides whether the REPL should keep reading. It
    // counted brackets while skipping string literals but not comments, so a
    // line like `let a = 1  # wrap this in a { block` left the REPL waiting
    // forever - swallowing every later line, including `:quit`, as continuation.

    #[test]
    fn continuation_ignores_brackets_inside_comments() {
        assert!(!needs_continuation("let a = 1  # TODO: wrap in a { block"));
        assert!(!needs_continuation("let a = 1  # see fn(x"));
        assert!(!needs_continuation("let a = 1  # index [0"));
    }

    #[test]
    fn continuation_ignores_brackets_inside_strings() {
        assert!(!needs_continuation("let s = \"a } b\""));
        assert!(!needs_continuation("let s = \"unclosed ( paren\""));
    }

    #[test]
    fn continuation_still_tracks_real_brackets() {
        assert!(needs_continuation("fn f(n) {"));
        assert!(needs_continuation("let xs = ["));
        assert!(needs_continuation("let y = f("));
        assert!(!needs_continuation("fn f(n) { n * 2 }"));
        assert!(!needs_continuation("let a = 1"));
    }

    #[test]
    fn continuation_handles_code_and_comment_together() {
        // A real brace opened on a line whose comment also mentions one.
        assert!(needs_continuation("fn f(n) {  # opens a { here"));
        // Comment closes nothing: the brace count must stay balanced.
        assert!(!needs_continuation("fn f(n) { n }  # closes the } block"));
    }

    #[test]
    fn continuation_resets_comment_state_each_line() {
        // The comment on the first line must not swallow the second.
        assert!(needs_continuation(
            "let a = 1  # note {
fn g() {"
        ));
        assert!(!needs_continuation(
            "let a = 1  # note {
let b = 2"
        ));
    }
}
