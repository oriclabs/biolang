use bl_core::value::{Table, Value};
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::builtins::flush_trailing_newline;
use bl_runtime::Interpreter;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::history::{History, SearchDirection};
use rustyline::{Config, Context, Editor, Helper};
use std::borrow::Cow;
use std::cell::RefCell;
use std::time::Instant;

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
    ":builtins", ":clear", ":cls", ":env", ":exit", ":fns", ":h", ":help", ":history", ":load",
    ":plugins", ":profile", ":q", ":quit", ":reset", ":save", ":time", ":type",
];

/// (command, description) — used for auto-hints when typing `:` commands
const REPL_COMMAND_HINTS: &[(&str, &str)] = &[
    (":help", "Show help"),
    (":h", "Show help"),
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
    (":reset", "Clear all user-defined state"),
    (":plugins", "List installed plugins"),
    (":profile", "Profile expression <expr>"),
    (":clear", "Clear the screen"),
    (":cls", "Clear the screen"),
    (":history", "Show command history [n]"),
];

const KEYWORDS: &[&str] = &[
    "and", "else", "enum", "false", "fn", "for", "if", "import", "in", "let", "match", "nil",
    "not", "or", "return", "true", "while", "yield",
];

// ── Tab Completion Helper ────────────────────────────────────────

struct BioHelper {
    words: Vec<String>,
    /// Display-only suffix appended by highlight_hint (not inserted on accept).
    hint_desc: RefCell<String>,
}

impl BioHelper {
    fn new() -> Self {
        Self {
            words: KEYWORDS.iter().map(|s| s.to_string()).collect(),
            hint_desc: RefCell::new(String::new()),
        }
    }

    /// Rebuild the completion word list from the interpreter's environment.
    fn refresh_from(&mut self, interp: &Interpreter) {
        let mut words: Vec<String> = KEYWORDS.iter().map(|s| s.to_string()).collect();
        for (name, _) in interp.env().list_global_vars() {
            if !words.contains(&name.to_string()) {
                words.push(name.to_string());
            }
        }
        words.sort();
        self.words = words;
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
        // Look up signature hint
        *self.hint_desc.borrow_mut() = String::new();
        fn_signature(word).map(|sig| sig[word.len()..].to_string())
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
}

impl Validator for BioHelper {}
impl Helper for BioHelper {}

// ── REPL ─────────────────────────────────────────────────────────

pub struct Repl {
    interpreter: Interpreter,
}

impl Repl {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::builder().bracketed_paste(true).build();
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

        let mut session_inputs: Vec<String> = Vec::new();

        loop {
            let readline = rl.readline(PROMPT);
            match readline {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let _ = rl.add_history_entry(&line);

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
                            ":help" | ":h" => self.cmd_help(),
                            ":env" => self.cmd_env(),
                            ":builtins" | ":fns" => cmd_builtins(arg),
                            ":reset" => {
                                self.cmd_reset();
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
                                    eprintln!("{RED}Usage: :save <file.bl>{RESET}");
                                } else {
                                    cmd_save(arg, &session_inputs);
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
                                let n: usize = arg.parse().unwrap_or(20);
                                let len = rl.history().len();
                                let start = len.saturating_sub(n);
                                for i in start..len {
                                    if let Ok(Some(result)) =
                                        rl.history().get(i, SearchDirection::Forward)
                                    {
                                        println!("{DIM}{:>4}{RESET}  {}", i + 1, result.entry);
                                    }
                                }
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

                    // Leading |> with no pending context → pipe from `_`
                    let mut input = if trimmed.starts_with("|>") {
                        format!("_\n{line}")
                    } else {
                        line
                    };

                    // Gather continuation lines while input is incomplete
                    // (unclosed delimiters, trailing |>, trailing operators,
                    //  block keywords without { )
                    while needs_continuation(&input) {
                        match rl.readline(CONTINUATION) {
                            Ok(cont) => {
                                input.push('\n');
                                input.push_str(&cont);
                            }
                            Err(_) => break,
                        }
                    }

                    // Input is syntactically complete → execute immediately
                    session_inputs.push(input.clone());
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

        Ok(())
    }

    fn eval_and_print(&mut self, input: &str) {
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
                    // Store last result as `_` for pipe continuation
                    self.interpreter
                        .env_mut()
                        .define("_".to_string(), value.clone());
                    print_colored_value(&value);
                }
            }
            Err(e) => {
                flush_trailing_newline();
                eprintln!("{RED}{}{RESET}", e.format_with_source(input));
            }
        }
    }

    // ── Command handlers ─────────────────────────────────────────

    fn cmd_help(&self) {
        println!("{BOLD}Commands:{RESET}");
        println!("  {CYAN}:help{RESET}  :h            Show this help");
        println!("  {CYAN}:quit{RESET}  :q            Exit the REPL (or Ctrl+D)");
        println!("  {CYAN}:env{RESET}                 Show user-defined variables");
        println!("  {CYAN}:type{RESET}  <expr>        Show the type of an expression");
        println!("  {CYAN}:builtins{RESET} [category]  List built-in functions (:fns alias)");
        println!("  {CYAN}:load{RESET}  <file>        Load and execute a .bl script");
        println!("  {CYAN}:save{RESET}  <file>        Save session inputs to a .bl file");
        println!("  {CYAN}:time{RESET}  <expr>        Evaluate and show elapsed time");
        println!("  {CYAN}:reset{RESET}               Clear all user-defined state");
        println!("  {CYAN}:plugins{RESET}             List installed plugins");
        println!("  {CYAN}:profile{RESET} <expr>       Profile function calls in expression");
        println!("  {CYAN}:clear{RESET}  :cls          Clear the screen");
        println!("  {CYAN}:history{RESET} [n]          Show last n commands (default 20)");
        println!("  {CYAN}?{RESET}name                Show function signature (e.g. ?mean)");
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
        println!("  dna\"ATCG\" |> rev_comp()      {DIM}# → CGAT{RESET}");
        println!("  :builtins stats              {DIM}# list stats functions{RESET}");
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
        use std::collections::HashMap;

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

fn print_banner() {
    println!(
        r#"
{BOLD}{CYAN}  ____  _       _
 | __ )(_) ___ | |    __ _ _ __   __ _
 |  _ \| |/ _ \| |   / _` | '_ \ / _` |
 | |_) | | (_) | |__| (_| | | | | (_| |
 |____/|_|\___/|_____\__,_|_| |_|\__, |
                                  |___/{RESET}
 {DIM}BioLang — pipe-first bioinformatics DSL{RESET}
 {DIM}v{VERSION}{RESET}

 {BOLD}Commands:{RESET}  {CYAN}:help{RESET}  {CYAN}:builtins{RESET}  {CYAN}:quit{RESET}  {CYAN}?{RESET}name  {CYAN}:load{RESET} <file>  {DIM}Tab for completion{RESET}
"#
    );
}

fn cmd_save(path: &str, inputs: &[String]) {
    if inputs.is_empty() {
        eprintln!("{DIM}No inputs to save.{RESET}");
        return;
    }
    let content = inputs.join("\n");
    match std::fs::write(path, content) {
        Ok(()) => println!("{DIM}Saved {} inputs to '{path}'.{RESET}", inputs.len()),
        Err(e) => eprintln!("{RED}Cannot write '{path}': {e}{RESET}"),
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
        Value::Range { start, end, inclusive } => {
            if *inclusive {
                format!("{start}..={end}")
            } else {
                format!("{start}..{end}")
            }
        }
        Value::EnumValue { enum_name, variant, .. } => format!("{enum_name}::{variant}"),
        Value::Set(items) => format!("#{{{} items}}", items.len()),
        Value::Regex { pattern, flags } => format!("/{pattern}/{flags}"),
        Value::Future(_) => "<future>".into(),
        Value::Kmer(km) => format!("Kmer({})", km.decode()),
        Value::SparseMatrix(sm) => format!("Sparse({}x{}, {} nnz)", sm.nrow, sm.ncol, sm.nnz()),
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(|v| value_preview(v)).collect();
            format!("({}{})", parts.join(", "), if items.len() == 1 { "," } else { "" })
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
    ("assert", "assert(condition, message)", "core"),
    ("debug", "debug(value)", "core"),
    ("env", "env(name) → Str|Nil", "core"),
    ("sleep", "sleep(ms)", "core"),
    ("doctor", "doctor() → Table (check env, containers, LLM, APIs)", "core"),
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
    ("find_index", "find_index(list, fn) → Int (-1 if not found)", "hof"),
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
    ("substr", "substr(str, start, len) → Str", "string"),
    ("char_at", "char_at(str, index) → Str|Nil", "string"),
    ("index_of", "index_of(str, sub) → Int (-1 if not found)", "string"),
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
    ("summary", "summary(list) → Record{min,q1,median,mean,q3,max}", "stats"),
    ("ttest", "ttest(list1, list2) → Record{t,p,df}", "stats"),
    ("ttest_one", "ttest_one(list, mu) → Record{t,p,df}", "stats"),
    ("ttest_paired", "ttest_paired(list1, list2) → Record{t,p}", "stats"),
    ("anova", "anova(groups) → Record{f,p,df}", "stats"),
    ("chi_square", "chi_square(obs, exp) → Record{chi2,p,df}", "stats"),
    ("fisher_exact", "fisher_exact(a,b,c,d) → Record{p,odds}", "stats"),
    ("wilcoxon", "wilcoxon(list1, list2) → Record{u,p}", "stats"),
    ("p_adjust", "p_adjust(pvals, method) → List", "stats"),
    ("normalize", "normalize(list, method) → List", "stats"),
    ("lm", "lm(x, y) → Record{slope,intercept,r2,p}", "stats"),
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
    ("http_get", "http_get(url, headers?) → {status, body, headers}", "fs"),
    ("http_post", "http_post(url, body, headers?) → {status, body, headers}", "fs"),
    ("download", "download(url, path?) → {path, size, url}", "fs"),
    ("upload", "upload(path, url, headers?) → {status, size}", "fs"),
    ("ref_genome", "ref_genome(name|\"list\", path?) → {path, name, description}", "fs"),
    ("bio_fetch", "bio_fetch(name, path?) → {path, name, description, cached}", "fs"),
    ("bio_sources", "bio_sources(category?) → Table of available data shortcuts", "fs"),
    // Plot
    ("plot", "plot(table, opts?) → Str (SVG)", "plot"),
    ("heatmap", "heatmap(table, opts?) → Str (SVG)", "plot"),
    ("histogram", "histogram(list, opts?) → Str (SVG)", "plot"),
    ("volcano", "volcano(table, opts?) → Str (SVG)", "plot"),
    ("ma_plot", "ma_plot(table, opts?) → Str (SVG)", "plot"),
    ("save_svg", "save_svg(svg, path)", "plot"),
    ("genome_track", "genome_track(table, opts?) → Str (SVG)", "plot"),
    ("hist", "hist(list, bins?) → Str (ASCII)", "plot"),
    ("scatter", "scatter(list1, list2) → Str (ASCII)", "plot"),
    // Matrix
    ("matrix", "matrix(nested_lists) → Matrix", "matrix"),
    ("zeros", "zeros(nrow, ncol) → Matrix", "matrix"),
    ("eye", "eye(n) → Matrix (identity)", "matrix"),
    ("dim", "dim(matrix) → [nrow, ncol]", "matrix"),
    ("transpose", "transpose(matrix) → Matrix", "matrix"),
    ("dot", "dot(a, b) → Matrix (matmul)", "matrix"),
    ("pca", "pca(matrix, n) → Record{scores,loadings,...}", "matrix"),
    ("cor_matrix", "cor_matrix(matrix) → Matrix", "matrix"),
    // Enrichment
    ("read_gmt", "read_gmt(path) → Map{set→genes}", "enrich"),
    ("enrich", "enrich(genes, sets, bg) → Table", "enrich"),
    ("gsea", "gsea(ranked, sets) → Table", "enrich"),
    // Bio
    ("dna", "dna\"ATCG\" → DNA sequence", "bio"),
    ("rev_comp", "rev_comp(seq) → DNA/RNA", "bio"),
    ("transcribe", "transcribe(dna) → RNA", "bio"),
    ("translate", "translate(rna|dna) → Protein", "bio"),
    ("gc_content", "gc_content(seq) → Float", "bio"),
    ("read_fasta", "read_fasta(path) → List[Record]", "bio"),
    ("read_fastq", "read_fastq(path) → List[Record]", "bio"),
    ("read_bed", "read_bed(path) → Table", "bio"),
    ("read_gff", "read_gff(path) → Table", "bio"),
    ("read_vcf", "read_vcf(path) → Table", "bio"),
    ("write_fasta", "write_fasta(records, path) → Int (count)", "bio"),
    ("write_fastq", "write_fastq(records, path) → Int (count)", "bio"),
    ("write_bed", "write_bed(records|table, path) → Int (count)", "bio"),
    ("write_vcf", "write_vcf(records, path) → Int (count)", "bio"),
    ("write_gff", "write_gff(records, path) → Int (count)", "bio"),
    ("validate", "validate(path) → {valid, format, errors, lines_checked}", "bio"),
    ("vcf_filter", "vcf_filter(path, expr) → Table (e.g. \"QUAL > 30 && DP > 10\")", "bio"),
    ("align", "align(seq1, seq2, opts?) → Record", "bio"),
    ("edit_distance", "edit_distance(s1, s2) → Int", "bio"),
    ("hamming_distance", "hamming_distance(s1, s2) → Int", "bio"),
    ("interval", "interval(chrom, start, end, strand?) → Interval", "bio"),
    // API
    ("ncbi_search", "ncbi_search(db, query) → Record", "api"),
    ("ncbi_gene", "ncbi_gene(query) → Record", "api"),
    ("ensembl_gene", "ensembl_gene(id) → Record", "api"),
    ("uniprot_search", "uniprot_search(query) → Record", "api"),
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
    ("sketch", "sketch(seq, k?, n?) → List (MinHash sketch)", "hash"),
    ("sketch_dist", "sketch_dist(a, b) → Float (Jaccard distance 0–1)", "hash"),
    // DateTime
    ("now", "now() → Str (ISO 8601 UTC)", "datetime"),
    ("timestamp", "timestamp() → Int (Unix epoch seconds)", "datetime"),
    ("timestamp_ms", "timestamp_ms() → Int (Unix epoch ms)", "datetime"),
    ("date_format", "date_format(date_str, fmt) → Str", "datetime"),
    ("date_parse", "date_parse(str, fmt) → Str (ISO 8601)", "datetime"),
    ("date_add", "date_add(date_str, amount, unit) → Str", "datetime"),
    ("date_diff", "date_diff(date1, date2, unit) → Int", "datetime"),
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
    ("uniq_count", "uniq_count(list) → List[{value, count}]", "text"),
    ("wc", "wc(input) → {lines, words, chars, bytes}", "text"),
    ("tee", "tee(value, path) → value (writes to file)", "text"),
    ("shell", "shell(cmd, stdin?) → {stdout, stderr, exit_code}", "text"),
    ("count_lines", "count_lines(path) → Int", "text"),
    ("stream_lines", "stream_lines(path) → Stream (lazy file reader)", "text"),
    ("stream_concat", "stream_concat(a, b) → Stream (lazy concat)", "text"),
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
    ("container_available", "container_available() → {runtime, version, image_dir}", "container"),
    ("container_run", "container_run(image, cmd, opts?) → {stdout, stderr, exit_code}", "container"),
    ("container_pull", "container_pull(image) → {image, storage, hint}", "container"),
    ("tool", "tool(name, cmd, opts?) → {stdout, stderr, exit_code}", "container"),
    ("tool_search", "tool_search(query, opts?) → List[{name, pulls, license, ...}]", "container"),
    ("tool_popular", "tool_popular(limit?) → List (sorted by downloads)", "container"),
    ("tool_info", "tool_info(name) → {name, pulls, license, versions: [...]}", "container"),
    ("tool_pull", "tool_pull(name, version?) → {image, storage, hint}", "container"),
    ("tool_list", "tool_list() → List[Str]", "container"),
    ("tool_available", "tool_available() → {runtime, version, image_dir}", "container"),
    // LLM
    ("chat", "chat(message, context?) → Str (LLM response)", "llm"),
    ("chat_code", "chat_code(description, context?) → Str (BioLang code)", "llm"),
    ("llm_models", "llm_models() → {provider, model, env_vars}", "llm"),
    // Units
    ("bp", "bp(n) → Record{value, unit} (base pairs)", "bio"),
    ("kb", "kb(n) → Record{value, unit} (kilobases)", "bio"),
    ("mb", "mb(n) → Record{value, unit} (megabases)", "bio"),
    ("gb", "gb(n) → Record{value, unit} (gigabases)", "bio"),
    // Generators
    ("help", "help(fn) → Nil (print function docs)", "core"),
    ("gen_int", "gen_int(lo, hi) → Int (seeded PRNG)", "core"),
    ("gen_float", "gen_float(lo, hi) → Float (seeded PRNG)", "core"),
    ("gen_str", "gen_str(len) → Str (random alphanumeric)", "core"),
    // Parallel + property testing
    ("par_map", "par_map(list, fn) → List (parallel map)", "hof"),
    ("par_filter", "par_filter(list, fn) → List (parallel filter)", "hof"),
    ("prop_test", "prop_test(property_fn, generator_fn, iters?) → Record", "hof"),
    // Transfer protocols
    ("ftp_download", "ftp_download(url, path?) → {path, size}", "transfer"),
    ("ftp_list", "ftp_list(url) → List[{name, path}]", "transfer"),
    ("ftp_upload", "ftp_upload(path, url) → {size}", "transfer"),
    ("sftp_download", "sftp_download(url, path?) → {path, size}", "transfer"),
    ("sftp_upload", "sftp_upload(path, url) → {size}", "transfer"),
    ("scp", "scp(source, dest) → {source, dest}", "transfer"),
    ("s3_download", "s3_download(s3_url, path?) → {path, size}", "transfer"),
    ("s3_upload", "s3_upload(path, s3_url) → {size}", "transfer"),
    ("s3_list", "s3_list(s3_url, recursive?) → List[{name, size}]", "transfer"),
    ("gcs_download", "gcs_download(gs_url, path?) → {path, size}", "transfer"),
    ("gcs_upload", "gcs_upload(path, gs_url) → {size}", "transfer"),
    ("rsync", "rsync(source, dest, opts?) → {source, dest}", "transfer"),
    ("aspera_download", "aspera_download(url, path?) → {path}", "transfer"),
    ("sra_prefetch", "sra_prefetch(accession, path?) → {path, accession}", "transfer"),
    ("sra_fastq", "sra_fastq(accession, path?) → {files, accession}", "transfer"),
    // Set operations
    ("set", "set(list) → Set (deduped)", "list"),
    ("union", "union(set1, set2) → Set", "list"),
    ("intersection", "intersection(set1, set2) → Set", "list"),
    ("difference", "difference(set1, set2) → Set", "list"),
    ("symmetric_difference", "symmetric_difference(set1, set2) → Set", "list"),
    ("is_subset", "is_subset(a, b) → Bool", "list"),
    ("is_superset", "is_superset(a, b) → Bool", "list"),
    // Async
    ("await_all", "await_all(futures) → List (resolve all)", "hof"),
    // Decorators
    ("memoize", "memoize(fn) → fn (cached results)", "hof"),
    ("time_it", "time_it(fn) → fn (prints elapsed time)", "hof"),
    ("once", "once(fn) → fn (execute only first call)", "hof"),
    // Genomic range queries
    ("interval_tree", "interval_tree(table) → Record (sorted intervals per chrom)", "bio"),
    ("query_overlaps", "query_overlaps(tree, chrom, start, end) → Table", "bio"),
    ("query_nearest", "query_nearest(tree, chrom, pos, k?) → Table", "bio"),
    ("coverage", "coverage(tree) → Table{chrom, start, end, depth}", "bio"),
    // Sequence pattern matching
    ("motif_find", "motif_find(seq, iupac_pattern) → List[{start, end, match}]", "bio"),
    ("motif_count", "motif_count(seq, iupac_pattern) → Int", "bio"),
    ("consensus", "consensus(sequences) → Str", "bio"),
    ("pwm", "pwm(sequences) → List[{A, C, G, T}] (position weight matrix)", "bio"),
    ("pwm_scan", "pwm_scan(seq, pwm, threshold?) → List[{pos, score}]", "bio"),
    // Pipeline
    ("pipeline_steps", "pipeline_steps(pipeline) → Table{step, name, plugin, params, depends_on}", "bio"),
    // GAP 1: Coordinate systems
    ("coord_bed", "coord_bed(val) → Record with __coord_system: 'bed'", "coord"),
    ("coord_vcf", "coord_vcf(val) → Record with __coord_system: 'vcf'", "coord"),
    ("coord_gff", "coord_gff(val) → Record with __coord_system: 'gff'", "coord"),
    ("coord_sam", "coord_sam(val) → Record with __coord_system: 'sam'", "coord"),
    ("coord_convert", "coord_convert(val, to_system) → Record (converted coordinates)", "coord"),
    ("coord_system", "coord_system(val) → Str (current coord system)", "coord"),
    ("coord_check", "coord_check(a, b) → Bool (are coord systems compatible?)", "coord"),
    // GAP 2: K-mers
    ("kmer_encode", "kmer_encode(seq, k) → Kmer or List[Kmer]", "kmer"),
    ("kmer_decode", "kmer_decode(kmer) → Str", "kmer"),
    ("kmer_rc", "kmer_rc(kmer) → Kmer (reverse complement)", "kmer"),
    ("kmer_canonical", "kmer_canonical(kmer) → Kmer (canonical form)", "kmer"),
    ("kmer_count", "kmer_count(seq, k) → Table{kmer, count}", "kmer"),
    ("kmer_spectrum", "kmer_spectrum(counts) → Table{frequency, count}", "kmer"),
    ("minimizers", "minimizers(seq, k, w) → List[{kmer, pos}]", "kmer"),
    // GAP 3: Streaming
    ("stream_chunks", "stream_chunks(stream, n) → Stream of List (chunks of n)", "stream"),
    ("stream_take", "stream_take(stream, n) → List (first n items)", "stream"),
    ("stream_skip", "stream_skip(stream, n) → Stream (skip first n)", "stream"),
    ("stream_batch", "stream_batch(stream, n, fn) → List (process in batches)", "stream"),
    ("memory_usage", "memory_usage() → Record{heap_bytes, ...}", "stream"),
    // GAP 4: Parallel
    ("scatter_by", "scatter_by(list, key_fn) → Map{key → List}", "hof"),
    ("bench", "bench(fn, args, n) → Record{mean_ns, min_ns, max_ns, iterations}", "hof"),
    // GAP 5: Sparse matrix
    ("sparse_matrix", "sparse_matrix(data) → SparseMatrix from triplets or nested lists", "sparse"),
    ("to_dense", "to_dense(sparse) → Matrix", "sparse"),
    ("to_sparse", "to_sparse(matrix) → SparseMatrix", "sparse"),
    ("sparse_get", "sparse_get(m, i, j) → Float", "sparse"),
    ("nnz", "nnz(m) → Int (non-zero count)", "sparse"),
    ("sparse_row_sums", "sparse_row_sums(m) → List[Float]", "sparse"),
    ("sparse_col_sums", "sparse_col_sums(m) → List[Float]", "sparse"),
    ("normalize_sparse", "normalize_sparse(m, method) → SparseMatrix ('log1p_cpm'|'scale')", "sparse"),
    // GAP 6: Typed table columns
    ("table_col_types", "table_col_types(table) → Record{col → type_str}", "table"),
    ("table_set_col_type", "table_set_col_type(table, col, type) → Record{table, schema}", "table"),
    ("table_validate", "table_validate(schema_record) → Record{valid, errors}", "table"),
    ("table_schema", "table_schema(table) → Record{columns, types, nrow, ncol}", "table"),
    ("table_cast", "table_cast(table, col, type) → Table (coerce column)", "table"),
    // GAP 7: Pipe fusion
    ("pipe_fuse", "pipe_fuse(list, ops...) → List (explicit fused pipeline)", "hof"),
    // GAP 8: Provenance
    ("with_provenance", "with_provenance(value, meta) → Record{__value, __provenance}", "provenance"),
    ("provenance", "provenance(wrapped) → Record or Nil (extract provenance)", "provenance"),
    ("provenance_chain", "provenance_chain(wrapped) → List (walk parent chain)", "provenance"),
    ("checkpoint", "checkpoint(name, value) → value (save to disk)", "provenance"),
    ("resume_checkpoint", "resume_checkpoint(name) → value or Nil", "provenance"),
    // GAP 10: Bio operations
    ("de_bruijn_graph", "de_bruijn_graph(sequences, k) → Record{nodes, edges}", "bio"),
    ("neighbor_joining", "neighbor_joining(distance_matrix) → List[{name, distance, children}]", "bio"),
    ("umap", "umap(matrix, n_components, opts?) → Matrix (embeddings)", "bio"),
    ("tsne", "tsne(matrix, n_components, opts?) → Matrix (embeddings)", "bio"),
    ("leiden", "leiden(adjacency, resolution?) → List[Int] (cluster assignments)", "bio"),
    ("diff_expr", "diff_expr(counts, groups) → Table{gene, log2fc, pvalue, padj, mean_a, mean_b}", "bio"),
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
        println!();
        println!("{DIM}Use :builtins <category> to list functions, e.g. :builtins stats{RESET}");
        println!("{DIM}Use ?name to show signature, e.g. ?mean{RESET}");
        return;
    }
    // Find matching category or search by name
    let matches: Vec<_> = BUILTIN_CATALOG
        .iter()
        .filter(|(name, _, cat)| cat.contains(filter.as_str()) || name.contains(filter.as_str()))
        .collect();
    if matches.is_empty() {
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
    println!("{DIM}Unknown function: {name}. Try :builtins to browse.{RESET}");
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
fn needs_continuation(input: &str) -> bool {
    let mut parens = 0i32;
    let mut braces = 0i32;
    let mut brackets = 0i32;
    let mut in_string = false;
    let mut prev_char = '\0';

    for ch in input.chars() {
        if in_string {
            if ch == '"' && prev_char != '\\' {
                in_string = false;
            }
        } else {
            match ch {
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
        Value::Protein(seq) => format!("{BOLD}{MAGENTA}Protein({}{RESET}{BOLD}{MAGENTA}){RESET}", &seq.data),
        Value::Interval(iv) => format!("{BLUE}{iv}{RESET}"),
        Value::List(items) => {
            if items.is_empty() {
                return format!("{DIM}[]{RESET}");
            }
            if items.len() <= 10 {
                let parts: Vec<String> = items.iter().map(|v| colorize_value(v)).collect();
                format!("[{}]", parts.join(", "))
            } else {
                let parts: Vec<String> = items[..10].iter().map(|v| colorize_value(v)).collect();
                format!("[{}, {DIM}... {} more{RESET}]", parts.join(", "), items.len() - 10)
            }
        }
        Value::Record(fields) => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{BOLD}{k}{RESET}: {}", colorize_value(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
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
            format!("{DIM}<fn {}>{RESET}", name.as_deref().unwrap_or("anonymous"))
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
        Value::Range { start, end, inclusive } => {
            if *inclusive {
                format!("{CYAN}{start}..={end}{RESET}")
            } else {
                format!("{CYAN}{start}..{end}{RESET}")
            }
        }
        Value::EnumValue { enum_name, variant, fields } => {
            if fields.is_empty() {
                format!("{MAGENTA}{enum_name}::{variant}{RESET}")
            } else {
                let args: Vec<String> = fields.iter().map(|v| colorize_value(v)).collect();
                format!("{MAGENTA}{enum_name}::{variant}{RESET}({})", args.join(", "))
            }
        }
        Value::Set(items) => {
            let parts: Vec<String> = items.iter().take(10).map(|v| colorize_value(v)).collect();
            if items.len() > 10 {
                format!("#{{{}, {DIM}... {} more{RESET}}}", parts.join(", "), items.len() - 10)
            } else {
                format!("#{{{}}}", parts.join(", "))
            }
        }
        Value::Regex { pattern, flags } => format!("{GREEN}/{pattern}/{flags}{RESET}"),
        Value::Future(_) => format!("{DIM}<future>{RESET}"),
        Value::Kmer(km) => format!("{MAGENTA}Kmer({}{RESET}{MAGENTA}){RESET}", km.decode()),
        Value::SparseMatrix(sm) => format!("{CYAN}{sm}{RESET}"),
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(|v| colorize_value(v)).collect();
            format!("({}{})", parts.join(", "), if items.len() == 1 { "," } else { "" })
        }
        Value::Gene { symbol, .. } => format!("{MAGENTA}Gene({symbol}){RESET}"),
        Value::Variant { chrom, pos, .. } => format!("{MAGENTA}Variant({chrom}:{pos}){RESET}"),
        Value::Genome { name, .. } => format!("{MAGENTA}Genome({name}){RESET}"),
        Value::Quality(scores) => format!("{BLUE}Quality({}bp){RESET}", scores.len()),
        Value::AlignedRead(r) => format!("{MAGENTA}AlignedRead({} {}:{}){RESET}", r.qname, r.rname, r.pos),
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
        natural_widths.iter().map(|w| (*w).min(MAX_COL_WIDTH)).collect()
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
    println!(
        "{DIM}Table: {} rows × {} cols{RESET}",
        t.rows.len(),
        ncols
    );

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
            'A' => { out.push_str(A_COLOR); out.push(ch); out.push_str(RESET); }
            'T' if !is_rna => { out.push_str(T_COLOR); out.push(ch); out.push_str(RESET); }
            'U' if is_rna => { out.push_str(T_COLOR); out.push(ch); out.push_str(RESET); }
            'G' => { out.push_str(G_COLOR); out.push(ch); out.push_str(RESET); }
            'C' => { out.push_str(C_COLOR); out.push(ch); out.push_str(RESET); }
            _ => { out.push_str(DIM); out.push(ch); out.push_str(RESET); }
        }
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
