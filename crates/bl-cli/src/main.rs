mod events;
mod notebook;
#[cfg(feature = "notebook-server")]
mod notebook_server;
mod testing;
mod update;

use bl_import as import;
use clap::{Parser, Subcommand, ValueEnum};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PlotModeArg {
    Auto,
    Unicode,
    Ascii,
    File,
    Open,
    Raw,
    None,
}

impl PlotModeArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Unicode => "unicode",
            Self::Ascii => "ascii",
            Self::File => "file",
            Self::Open => "open",
            Self::Raw => "raw",
            Self::None => "none",
        }
    }
}

#[derive(Parser)]
#[command(
    name = "bl",
    version = env!("CARGO_PKG_VERSION"),
    about = "BioLang — pipe-first bioinformatics DSL"
)]
struct Cli {
    /// Disable GPU acceleration and use the deterministic f64 CPU backend
    #[arg(long, global = true, conflicts_with = "gpu")]
    no_gpu: bool,
    /// Enable GPU auto-detection explicitly (this is the default)
    #[arg(long, global = true, conflicts_with = "no_gpu")]
    gpu: bool,
    /// Display SVG plots as terminal graphics, files, raw markup, or not at all
    #[arg(long, global = true, value_enum)]
    plot: Option<PlotModeArg>,
    /// Directory used by --plot file and --plot open
    #[arg(long, global = true, value_name = "DIR")]
    plot_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a BioLang script file
    Run {
        /// Path to the .bl script file
        file: String,
        /// Show each step as it executes
        #[arg(short, long)]
        verbose: bool,
        /// Emit versioned JSON Lines execution events
        #[arg(long)]
        events: bool,
        /// Print the script's final non-nil value
        #[arg(long)]
        print_result: bool,
    },
    /// Parse BioLang files without executing them
    Check {
        /// Paths to one or more .bl files
        #[arg(required = true)]
        files: Vec<String>,
    },
    /// Run `test_*` functions and report pass or fail
    Test {
        /// Files or directories to search; defaults to the working directory
        files: Vec<String>,
        /// Emit versioned JSON Lines test events
        #[arg(long)]
        events: bool,
    },
    /// Rewrite BioLang files in the canonical layout
    Fmt {
        /// Paths to .bl files, or `-` to format stdin to stdout
        #[arg(required = true)]
        files: Vec<String>,
        /// Report which files would change without writing them
        #[arg(long)]
        check: bool,
        /// Print the formatted source instead of rewriting the file
        #[arg(long)]
        stdout: bool,
        /// Spaces per indent level
        #[arg(long, default_value_t = 4)]
        indent: usize,
    },
    /// Start the interactive REPL
    Repl {
        /// Use the newline-delimited JSON protocol for editor integrations
        #[arg(long)]
        json: bool,
    },
    /// Start the LSP server (for editor integration)
    Lsp,
    /// Add a plugin (local path)
    Add {
        /// Plugin name (e.g. somer.align)
        name: String,
        /// Local path to plugin directory
        #[arg(long)]
        path: Option<String>,
    },
    /// Remove a plugin
    Remove {
        /// Plugin name (e.g. somer.align)
        name: String,
    },
    /// List installed plugins
    Plugins,
    /// Initialize a new BioLang package (creates biolang.toml)
    Init {
        /// Package name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Run a literate notebook (.bln or .bl.md file)
    Notebook {
        /// Path to a notebook, or `serve` followed by a notebook path
        file: String,
        /// Notebook path when using `bl notebook serve NOTEBOOK`
        #[arg(value_name = "NOTEBOOK")]
        serve_file: Option<String>,
        /// Export format: html, html-wasm, typst, pdf
        #[arg(long)]
        export: Option<String>,
        /// Where `--export html-wasm` should load the runtime from.
        ///
        /// The exported page is a loose file that can be served from anywhere,
        /// so it cannot assume the site layout the website itself uses.
        #[arg(long, default_value = "https://lang.bio/wasm")]
        wasm_base: String,
        /// Convert Jupyter .ipynb to .bln format (prints to stdout)
        #[arg(long)]
        from_ipynb: bool,
        /// Convert .bln to Jupyter .ipynb format (prints to stdout)
        #[arg(long)]
        to_ipynb: bool,
        /// Loopback address for `notebook serve`
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        /// Port for `notebook serve`; 0 asks the operating system for a free port
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Filesystem root disclosed to the notebook server (defaults to the notebook directory)
        #[arg(long)]
        root: Option<String>,
        /// Do not open the local notebook page in the default browser
        #[arg(long)]
        no_open: bool,
    },
    /// Install package dependencies
    Install {
        /// Package name or path
        source: Option<String>,
        /// Git URL
        #[arg(long)]
        git: Option<String>,
        /// Git branch
        #[arg(long)]
        branch: Option<String>,
    },
    /// List or copy examples bundled with an installed or local package
    Examples {
        /// Installed package name or local package directory
        package: String,
        /// Copy the complete example set into this new or empty directory
        #[arg(long, value_name = "DIR")]
        copy: Option<String>,
    },
    /// Convert Python, R, Jupyter, or R Markdown source to BioLang
    Import {
        /// Source file to convert (.py, .R, .ipynb, .Rmd), or `-` to read stdin
        file: String,
        /// Source format: python, r, ipynb, rmd (auto-detected from extension)
        #[arg(long)]
        from: Option<String>,
        /// Original file name used for format detection and naming (required with `-`)
        #[arg(long)]
        name: Option<String>,
        /// Write output to this file instead of stdout
        #[arg(short, long)]
        output: Option<String>,
        /// Validate generated BioLang syntax and exit non-zero when diagnostics remain
        #[arg(long)]
        validate: bool,
        /// Emit the full import result (content + validation) as JSON on stdout
        #[arg(long)]
        json: bool,
    },
    /// Convert biological data files with the optional bl-convert executable
    Convert {
        /// BL Convert arguments; INPUT OUTPUT implies the `convert` operation
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        arguments: Vec<OsString>,
    },
    /// Check the environment and per-capability readiness (native vs container)
    Doctor,
    /// Print a shell completion script
    ///
    /// Shells already complete file paths for native commands, so the value
    /// here is the part they cannot guess — subcommands and flags — and
    /// narrowing `bl run` to the files it can actually run.
    ///
    ///   bash:       bl completions bash       >> ~/.bashrc
    ///   zsh:        bl completions zsh        > ~/.zfunc/_bl
    ///   fish:       bl completions fish       > ~/.config/fish/completions/bl.fish
    ///   powershell: bl completions powershell >> $PROFILE
    Completions {
        /// Shell to generate for
        shell: clap_complete::Shell,
    },
    /// Show version and check for updates
    Version,
    /// Upgrade to the latest release
    Upgrade,
    /// Export structured language and builtin metadata
    Metadata {
        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },
}

fn main() {
    // Spawn on a thread with a large stack: the interpreter spends several Rust
    // frames per BioLang call, so recursion is far more stack-hungry than the
    // source suggests. At 64 MB a plain `fn f(n) { f(n-1) }` aborted the process
    // somewhere past a hundred levels, which ordinary tree recursion reaches.
    // 256 MB carries MAX_CALL_DEPTH comfortably; past that the interpreter
    // raises a BioLang error rather than letting the stack overflow.
    //
    // The comment here used to say 8 MB while the code asked for 64.
    let builder = std::thread::Builder::new()
        .name("bl-main".into())
        .stack_size(256 * 1024 * 1024);
    let handler = builder
        .spawn(|| {
            let cli = Cli::parse();

            // Set the policy before either `doctor` or an analysis can cause
            // the lazy adapter probe to be cached. The environment variable is
            // also the stable interface for notebooks, services, and workers.
            if cli.no_gpu {
                std::env::set_var("BIOLANG_GPU", "off");
            } else if cli.gpu {
                std::env::set_var("BIOLANG_GPU", "on");
            }
            if let Some(mode) = cli.plot {
                std::env::set_var("BIOLANG_PLOT", mode.as_str());
            }
            if let Some(directory) = cli.plot_dir {
                std::env::set_var("BIOLANG_PLOT_DIR", directory);
            }

            // Background update check for interactive commands
            match &cli.command {
                Some(Commands::Run { events: false, .. })
                | Some(Commands::Repl { json: false })
                | None => {
                    update::check_for_updates_background();
                }
                _ => {}
            }

            match cli.command {
                Some(Commands::Run {
                    file,
                    verbose,
                    events,
                    print_result,
                }) => run_file(&file, verbose, events, print_result),
                Some(Commands::Check { files }) => check_files(&files),
                Some(Commands::Test { files, events }) => run_tests(&files, events),
                Some(Commands::Fmt {
                    files,
                    check,
                    stdout,
                    indent,
                }) => format_files(&files, check, stdout, indent),
                Some(Commands::Notebook {
                    file,
                    serve_file,
                    export,
                    wasm_base,
                    from_ipynb,
                    to_ipynb,
                    bind,
                    port,
                    root,
                    no_open,
                }) => {
                    if file == "serve" {
                        let Some(notebook_path) = serve_file else {
                            eprintln!("Usage: bl notebook serve <NOTEBOOK> [--port PORT] [--no-open]");
                            process::exit(2);
                        };
                        if export.is_some() || from_ipynb || to_ipynb {
                            eprintln!("Error: export and conversion flags cannot be used with notebook serve");
                            process::exit(2);
                        }
                        #[cfg(feature = "notebook-server")]
                        notebook_server::serve(
                            &notebook_path,
                            &bind,
                            port,
                            root.as_deref(),
                            !no_open,
                        );
                        #[cfg(not(feature = "notebook-server"))]
                        {
                            let _ = (notebook_path, bind, port, root, no_open);
                            eprintln!(
                                "Error: this bl binary was built without the `notebook-server` feature"
                            );
                            process::exit(2);
                        }
                    } else if serve_file.is_some() {
                        eprintln!("Error: unexpected second notebook path; use `bl notebook serve <NOTEBOOK>`");
                        process::exit(2);
                    } else if from_ipynb {
                        notebook::ipynb_to_bln(&file);
                    } else if to_ipynb {
                        notebook::bln_to_ipynb(&file);
                    } else if let Some(fmt) = export {
                        match fmt.as_str() {
                            "html" => notebook::export_html(&file),
                            "html-wasm" | "html-live" => {
                                notebook::export_html_wasm(&file, &wasm_base)
                            }
                            "typst" | "typ" => notebook::export_typst(&file),
                            "pdf" => notebook::export_pdf(&file),
                            _ => {
                                eprintln!(
                                    "Unknown export format '{fmt}'. Supported: html, html-wasm, typst, pdf"
                                );
                                process::exit(1);
                            }
                        }
                    } else {
                        notebook::run_notebook(&file);
                    }
                }
                Some(Commands::Lsp) => cmd_lsp(),
                Some(Commands::Add { name, path }) => cmd_add(&name, path.as_deref()),
                Some(Commands::Remove { name }) => cmd_remove(&name),
                Some(Commands::Plugins) => cmd_plugins(),
                Some(Commands::Init { name }) => cmd_init(name.as_deref()),
                Some(Commands::Install {
                    source,
                    git,
                    branch,
                }) => cmd_install(source.as_deref(), git.as_deref(), branch.as_deref()),
                Some(Commands::Examples { package, copy }) => {
                    cmd_examples(&package, copy.as_deref())
                }
                Some(Commands::Import {
                    file,
                    from,
                    name,
                    output,
                    validate,
                    json,
                }) => cmd_import(
                    &file,
                    from.as_deref(),
                    name.as_deref(),
                    output.as_deref(),
                    validate,
                    json,
                ),
                Some(Commands::Convert { arguments }) => cmd_convert(arguments),
                Some(Commands::Doctor) => print!("{}", bl_runtime::capabilities::doctor_report()),
                Some(Commands::Completions { shell }) => {
                    let mut cmd = <Cli as clap::CommandFactory>::command();
                    let name = cmd.get_name().to_string();
                    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
                }
                Some(Commands::Version) => update::cmd_version(),
                Some(Commands::Upgrade) => update::cmd_upgrade(),
                Some(Commands::Metadata { format }) => cmd_metadata(&format),
                Some(Commands::Repl { json: true }) => {
                    if let Err(error) = bl_repl::run_console_protocol() {
                        eprintln!("Console protocol failed: {error}");
                        process::exit(1);
                    }
                }
                Some(Commands::Repl { json: false }) | None => start_repl(),
            }
        })
        .expect("failed to spawn main thread");
    handler.join().expect("main thread panicked");
}

fn cmd_convert(mut arguments: Vec<OsString>) {
    let direct_commands = ["convert", "formats", "inspect", "doctor", "tool", "help"];
    let direct = arguments
        .first()
        .and_then(|argument| argument.to_str())
        .is_some_and(|argument| direct_commands.contains(&argument));
    if !direct && !arguments.is_empty() {
        arguments.insert(0, OsString::from("convert"));
    }

    let sibling = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .map(|directory| directory.join(format!("bl-convert{}", std::env::consts::EXE_SUFFIX)))
        .filter(|candidate| candidate.is_file());
    let executable = sibling.unwrap_or_else(|| PathBuf::from("bl-convert"));
    match process::Command::new(&executable).args(arguments).status() {
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("BL Convert is not installed.");
            eprintln!("Install it from a BioLang checkout with:");
            eprintln!("  cargo install --path crates/bl-convert");
            process::exit(2);
        }
        Err(error) => {
            eprintln!("Could not start '{}': {error}", executable.display());
            process::exit(2);
        }
    }
}

fn cmd_metadata(format: &str) {
    if format != "json" {
        eprintln!("Unknown metadata format '{format}'. Supported: json");
        process::exit(1);
    }
    match serde_json::to_string_pretty(&bl_repl::biolang_metadata()) {
        Ok(document) => println!("{document}"),
        Err(error) => {
            eprintln!("Cannot serialize BioLang metadata: {error}");
            process::exit(1);
        }
    }
}

fn fail_run(message: String, structured_events: bool, start: &Instant) -> ! {
    if structured_events {
        events::emit(serde_json::json!({
            "protocol": "biolang.events/v1",
            "event": "error",
            "message": message,
        }));
        events::emit(serde_json::json!({
            "protocol": "biolang.events/v1",
            "event": "finished",
            "status": "failed",
            "durationMs": start.elapsed().as_millis(),
        }));
    } else {
        eprintln!("{message}");
    }
    process::exit(1);
}

/// Upper bound on typed results promoted from print/println in one run, so a
/// loop printing a table per iteration cannot flood the event stream.
const MAX_DISPLAYED_RESULTS: usize = 32;

/// Upper bound on inline trace events. Higher than the typed-result cap because
/// a trace line is a short string, but still bounded so a print inside a loop
/// over a million reads cannot flood the event stream.
const MAX_TRACE_EVENTS: usize = 500;

/// A "did you mean" for a script path that could not be read.
///
/// The language already does this for identifiers — see `suggest_builtin` — so
/// a mistyped file name gets the same treatment. It only ever runs after a read
/// has already failed, so it cannot change what a working command does; a name
/// that resolves is never second-guessed.
///
/// Deliberately not prefix resolution. Having `bl run ch03` silently pick
/// `ch03-normalization-pca.bl` would make a script's meaning depend on what
/// else is sitting in the directory, so a working command could start running a
/// different file the day someone adds a similarly named one.
fn nearby_script_hint(path: &str) -> String {
    let wanted = PathBuf::from(path);
    let dir = match wanted.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let stem = wanted
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if stem.is_empty() {
        return String::new();
    }

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return String::new();
    };
    let mut candidates: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| {
            let lower = n.to_lowercase();
            lower.ends_with(".bl") || lower.ends_with(".bln") || lower.ends_with(".bl.md")
        })
        .collect();
    if candidates.is_empty() {
        return String::new();
    }

    // A shared prefix is the common case with numbered chapter scripts;
    // otherwise fall back to the closest name by edit distance.
    let bare = stem.trim_end_matches(".bl").trim_end_matches(".bln");
    let mut by_prefix: Vec<&String> = candidates
        .iter()
        .filter(|n| n.to_lowercase().starts_with(bare) && !bare.is_empty())
        .collect();
    by_prefix.sort();
    if !by_prefix.is_empty() {
        let list = by_prefix
            .iter()
            .take(3)
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "
  did you mean {list}?"
        );
    }

    candidates.sort_by_key(|n| edit_distance(&stem, &n.to_lowercase()));
    let best = &candidates[0];
    if edit_distance(&stem, &best.to_lowercase()) <= (stem.len() / 2).max(3) {
        return format!(
            "
  did you mean {best}?"
        );
    }
    String::new()
}

/// Levenshtein distance, for the suggestion above.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn run_file(path: &str, verbose: bool, structured_events: bool, print_result: bool) {
    let start = Instant::now();
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(error) => fail_run(
            format!(
                "Error reading '{path}': {error}{}",
                nearby_script_hint(path)
            ),
            structured_events,
            &start,
        ),
    };

    // Show what we're running
    let display_path = PathBuf::from(path);
    let filename = display_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    if structured_events {
        let compute_backend = bl_runtime::gpu::execution_summary();
        events::emit(serde_json::json!({
            "protocol": "biolang.events/v1",
            "event": "started",
            "path": path,
            "file": filename,
            "computeBackend": compute_backend,
        }));
    } else {
        eprintln!("\x1b[2m▶ running {filename}\x1b[0m");
        eprintln!(
            "\x1b[2m  compute backend: {}\x1b[0m",
            bl_runtime::gpu::execution_summary()
        );
    }

    let tokens = match bl_lexer::Lexer::new(&source).tokenize() {
        Ok(t) => t,
        Err(error) => fail_run(error.format_with_source(&source), structured_events, &start),
    };

    let parse_result = match bl_parser::Parser::new(tokens).parse() {
        Ok(r) => r,
        Err(error) => fail_run(error.format_with_source(&source), structured_events, &start),
    };
    if parse_result.has_errors() {
        fail_run(
            parse_result
                .errors
                .iter()
                .map(|error| error.format_with_source(&source))
                .collect::<Vec<_>>()
                .join("\n"),
            structured_events,
            &start,
        );
    }
    let program = parse_result.program;

    let mut interpreter = bl_runtime::Interpreter::new();
    interpreter.verbose = verbose;
    if let Ok(canonical) = std::fs::canonicalize(path) {
        interpreter.set_current_file(Some(canonical));
    } else {
        interpreter.set_current_file(Some(PathBuf::from(path)));
    }
    if structured_events {
        bl_runtime::builtins::set_output_sink(Some(std::sync::Arc::new(|text| {
            events::emit(serde_json::json!({
                "protocol": "biolang.events/v1",
                "event": "output",
                "stream": "stdout",
                "text": text,
            }));
        })));
        // A printed table is a result the author asked to look at. Emitting one
        // only for the program's trailing expression hid every table written
        // the idiomatic way, as `println(...)` returns Nil.
        let displayed = std::sync::atomic::AtomicUsize::new(0);
        let traced = std::sync::atomic::AtomicUsize::new(0);
        let lines = events::LineIndex::new(&source);
        bl_runtime::builtins::set_display_sink(Some(std::sync::Arc::new(move |value, offset| {
            // A `trace` event carries the line that produced the value so the
            // editor can annotate the source, not just the output log.
            if let Some(offset) = offset {
                if traced.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < MAX_TRACE_EVENTS {
                    events::emit(serde_json::json!({
                        "protocol": "biolang.events/v1",
                        "event": "trace",
                        "line": lines.line_of(offset),
                        "text": events::value_preview(value),
                    }));
                }
            }
            if displayed.load(std::sync::atomic::Ordering::Relaxed) >= MAX_DISPLAYED_RESULTS {
                return;
            }
            if !events::is_structured_result(value) {
                return;
            }
            displayed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            events::emit(serde_json::json!({
                "protocol": "biolang.events/v1",
                "event": "result",
                "value": events::value_to_json(value),
            }));
        })));
    } else {
        // A script may either return an SVG or explicitly print one. Both
        // should obey the same CLI plot policy; normal text is unchanged.
        bl_runtime::builtins::set_output_sink(Some(std::sync::Arc::new(bl_repl::write_cli_output)));
    }
    match interpreter.run(&program) {
        Ok(value) => {
            if structured_events {
                bl_runtime::builtins::flush_trailing_newline();
                bl_runtime::builtins::set_output_sink(None);
                bl_runtime::builtins::set_display_sink(None);
                // A script ending in `println(...)` returns Nil. Reporting that
                // as a result padded the run summary with a phantom entry
                // beside the tables the author actually printed.
                if !matches!(value, bl_core::value::Value::Nil) {
                    events::emit(serde_json::json!({
                        "protocol": "biolang.events/v1",
                        "event": "result",
                        "value": events::value_to_json(&value),
                    }));
                }
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": "finished",
                    "status": "succeeded",
                    "durationMs": start.elapsed().as_millis(),
                }));
            } else {
                bl_runtime::builtins::flush_trailing_newline();
                bl_runtime::builtins::set_output_sink(None);
                if print_result && !matches!(value, bl_core::value::Value::Nil) {
                    bl_repl::print_cli_value(&value);
                }
                let elapsed = start.elapsed();
                eprintln!("\x1b[2m✓ done in {elapsed:.2?}\x1b[0m");
            }
            bl_runtime::tempfiles::cleanup_all();
        }
        Err(e) => {
            if structured_events {
                bl_runtime::builtins::set_output_sink(None);
                bl_runtime::builtins::set_display_sink(None);
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": "error",
                    "message": e.format_with_source(&source),
                }));
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": "finished",
                    "status": "failed",
                    "durationMs": start.elapsed().as_millis(),
                }));
            } else {
                bl_runtime::builtins::flush_trailing_newline();
                bl_runtime::builtins::set_output_sink(None);
                eprintln!("{}", e.format_with_source(&source));
            }
            bl_runtime::tempfiles::cleanup_all();
            process::exit(1);
        }
    }
}

/// Format `files` in place, or report/print instead when asked.
///
/// Exits non-zero when `--check` finds work to do, so it drops straight into a
/// CI step or a pre-commit hook.
fn format_files(files: &[String], check: bool, to_stdout: bool, indent: usize) {
    let options = bl_fmt::FormatOptions {
        indent_width: indent.clamp(1, 16),
        ..bl_fmt::FormatOptions::default()
    };
    let mut changed = 0usize;
    let mut failures = 0usize;

    for path in files {
        if path == "-" {
            let mut source = String::new();
            if let Err(error) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut source) {
                eprintln!("cannot read stdin: {error}");
                failures += 1;
                continue;
            }
            print!("{}", bl_fmt::format_source(&source, options));
            continue;
        }

        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{path}: cannot read file: {error}");
                failures += 1;
                continue;
            }
        };
        let formatted = bl_fmt::format_source(&source, options);
        if to_stdout {
            print!("{formatted}");
            continue;
        }
        if formatted == source {
            continue;
        }
        changed += 1;
        if check {
            println!("{path}");
            continue;
        }
        if let Err(error) = std::fs::write(path, &formatted) {
            eprintln!("{path}: cannot write file: {error}");
            failures += 1;
        } else {
            println!("formatted {path}");
        }
    }

    if failures > 0 {
        process::exit(1);
    }
    if check && changed > 0 {
        eprintln!(
            "{changed} file{} would be reformatted",
            if changed == 1 { "" } else { "s" }
        );
        process::exit(1);
    }
}

/// Run `test_*` functions across the given files or directories.
///
/// Exits non-zero on any failure so it drops into CI without a wrapper.
fn run_tests(files: &[String], structured_events: bool) {
    let targets = if files.is_empty() {
        vec![".".to_string()]
    } else {
        files.to_vec()
    };
    let paths = testing::collect_files(&targets);
    if paths.is_empty() {
        eprintln!("No .bl files found");
        process::exit(1);
    }

    let started = Instant::now();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut files_with_tests = 0usize;

    for path in &paths {
        let display = path.display().to_string().replace('\\', "/");
        let outcomes = match testing::run_file(path) {
            Ok(outcomes) => outcomes,
            Err(error) => {
                failed += 1;
                if structured_events {
                    events::emit(serde_json::json!({
                        "protocol": "biolang.events/v1",
                        "event": "testFailed",
                        "file": display,
                        "name": "(file)",
                        "message": error,
                    }));
                } else {
                    eprintln!("{error}");
                }
                continue;
            }
        };
        if outcomes.is_empty() {
            continue;
        }
        files_with_tests += 1;
        if !structured_events {
            println!("\n{display}");
        }
        for outcome in outcomes {
            let label = testing::describe(&outcome.name);
            if outcome.passed {
                passed += 1;
            } else {
                failed += 1;
            }
            if structured_events {
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": if outcome.passed { "testPassed" } else { "testFailed" },
                    "file": display,
                    "name": outcome.name,
                    "label": label,
                    "durationMs": outcome.duration_ms,
                    "message": outcome.message,
                }));
            } else if outcome.passed {
                println!("  \x1b[32mok\x1b[0m   {label} ({} ms)", outcome.duration_ms);
            } else {
                println!("  \x1b[31mFAIL\x1b[0m {label}");
                if let Some(message) = &outcome.message {
                    println!("       {message}");
                }
            }
        }
    }

    let elapsed = started.elapsed();
    if structured_events {
        events::emit(serde_json::json!({
            "protocol": "biolang.events/v1",
            "event": "testFinished",
            "passed": passed,
            "failed": failed,
            "files": files_with_tests,
            "durationMs": elapsed.as_millis(),
        }));
    } else if passed + failed == 0 {
        println!("No tests found. Name a function `test_something` to make it one.");
    } else {
        println!(
            "\n{passed} passed, {failed} failed in {files_with_tests} file{} ({elapsed:.2?})",
            if files_with_tests == 1 { "" } else { "s" }
        );
    }
    if failed > 0 {
        process::exit(1);
    }
}

fn check_files(files: &[String]) {
    let mut failures = 0usize;
    for path in files {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{path}: cannot read file: {error}");
                failures += 1;
                continue;
            }
        };
        let tokens = match bl_lexer::Lexer::new(&source).tokenize() {
            Ok(tokens) => tokens,
            Err(error) => {
                eprintln!("{path}:\n{}", error.format_with_source(&source));
                failures += 1;
                continue;
            }
        };
        let parsed = match bl_parser::Parser::new(tokens).parse() {
            Ok(parsed) => parsed,
            Err(error) => {
                eprintln!("{path}:\n{}", error.format_with_source(&source));
                failures += 1;
                continue;
            }
        };
        if parsed.has_errors() {
            eprintln!(
                "{path}:\n{}",
                parsed
                    .errors
                    .iter()
                    .map(|error| error.format_with_source(&source))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            failures += 1;
        }
    }
    if failures > 0 {
        eprintln!(
            "{failures} of {} BioLang file{} failed syntax validation",
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        );
        process::exit(1);
    }
    println!(
        "Checked {} BioLang file{}",
        files.len(),
        if files.len() == 1 { "" } else { "s" }
    );
}

fn start_repl() {
    let mut repl = bl_repl::Repl::new();
    if let Err(e) = repl.run() {
        eprintln!("REPL error: {e}");
        process::exit(1);
    }
}

fn cmd_lsp() {
    // Spawn bl-lsp binary (built from bl-lsp crate in same workspace)
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bl"));
    let lsp_exe = exe
        .parent()
        .map(|p| p.join("bl-lsp"))
        .filter(|p| p.exists());

    if let Some(lp) = lsp_exe {
        let status = std::process::Command::new(lp).status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("Error running bl-lsp: {e}");
                process::exit(1);
            }
        }
    } else {
        eprintln!("bl-lsp binary not found. Build it first: cargo build -p bl-lsp");
        process::exit(1);
    }
}

fn cmd_add(name: &str, local_path: Option<&str>) {
    let dir_name = bl_runtime::plugins::normalize_plugin_name(name);

    let target = match bl_runtime::plugins::plugins_dir() {
        Some(d) => d.join(&dir_name),
        None => {
            eprintln!("Error: cannot determine plugins directory (no HOME)");
            process::exit(1);
        }
    };

    let source = match local_path {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Error: --path is required (remote install not yet supported)");
            process::exit(1);
        }
    };

    // Validate source has plugin.json
    let manifest_path = source.join("plugin.json");
    if !manifest_path.is_file() {
        eprintln!("Error: no plugin.json found in '{}'", source.display());
        process::exit(1);
    }

    // Validate plugin.json is valid
    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading plugin.json: {e}");
            process::exit(1);
        }
    };
    if serde_json::from_str::<serde_json::Value>(&content).is_err() {
        eprintln!("Error: invalid JSON in plugin.json");
        process::exit(1);
    }

    // Copy directory contents
    if target.exists() {
        let _ = std::fs::remove_dir_all(&target);
    }
    if let Err(e) = copy_dir_recursive(&source, &target) {
        eprintln!("Error copying plugin: {e}");
        process::exit(1);
    }

    println!("Installed plugin '{name}' to {}", target.display());
}

fn cmd_remove(name: &str) {
    let dir_name = bl_runtime::plugins::normalize_plugin_name(name);

    let target = match bl_runtime::plugins::plugins_dir() {
        Some(d) => d.join(&dir_name),
        None => {
            eprintln!("Error: cannot determine plugins directory");
            process::exit(1);
        }
    };

    if !target.exists() {
        eprintln!("Plugin '{name}' is not installed");
        process::exit(1);
    }

    if let Err(e) = std::fs::remove_dir_all(&target) {
        eprintln!("Error removing plugin: {e}");
        process::exit(1);
    }

    println!("Removed plugin '{name}'");
}

fn cmd_plugins() {
    let plugins = bl_runtime::plugins::list_installed_plugins();

    if plugins.is_empty() {
        println!("No plugins installed.");
        println!("Use 'bl add <name> --path <dir>' to install a plugin.");
        return;
    }

    println!(
        "{:<20} {:<10} {:<12} DESCRIPTION",
        "NAME", "VERSION", "KIND"
    );
    println!("{}", "-".repeat(70));
    for p in &plugins {
        println!(
            "{:<20} {:<10} {:<12} {}",
            p.name, p.version, p.kind, p.description
        );
    }
    println!("\n{} plugin(s) installed.", plugins.len());
}

fn cmd_init(name: Option<&str>) {
    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let pkg_name = name.map(|s| s.to_string()).unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "my-project".to_string())
    });

    match bl_runtime::package::init_package(&dir, &pkg_name) {
        Ok(path) => println!("Created {}", path.display()),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn cmd_install(source: Option<&str>, git: Option<&str>, branch: Option<&str>) {
    // If no args, install all deps from biolang.toml
    if source.is_none() && git.is_none() {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match bl_runtime::package::read_manifest(&dir) {
            Ok(manifest) => {
                if manifest.dependencies.is_empty() {
                    println!("No dependencies to install.");
                    return;
                }
                for (name, dep) in &manifest.dependencies {
                    match dep {
                        bl_runtime::package::Dependency::Version(v) => {
                            println!("Skipping {name}@{v} (registry not yet supported)");
                        }
                        bl_runtime::package::Dependency::Detailed(d) => {
                            if let Some(path) = &d.path {
                                match bl_runtime::package::install_path_dep(
                                    name,
                                    &PathBuf::from(path),
                                ) {
                                    Ok(p) => println!("Installed {name} from {}", p.display()),
                                    Err(e) => eprintln!("Error installing {name}: {e}"),
                                }
                            } else if let Some(url) = &d.git {
                                match bl_runtime::package::install_git_dep(
                                    name,
                                    url,
                                    d.branch.as_deref(),
                                ) {
                                    Ok(p) => println!("Installed {name} from {}", p.display()),
                                    Err(e) => eprintln!("Error installing {name}: {e}"),
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        return;
    }

    // Install a specific package
    if let Some(url) = git {
        let name = source.unwrap_or_else(|| {
            url.rsplit('/')
                .next()
                .unwrap_or("package")
                .trim_end_matches(".git")
        });
        match bl_runtime::package::install_git_dep(name, url, branch) {
            Ok(p) => println!("Installed {name} to {}", p.display()),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
    } else if let Some(path) = source {
        let src = PathBuf::from(path);
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "package".to_string());
        match bl_runtime::package::install_path_dep(&name, &src) {
            Ok(p) => println!("Installed {name} to {}", p.display()),
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
    }
}

fn cmd_examples(package: &str, copy: Option<&str>) {
    let local = PathBuf::from(package);
    let package_dir = if local.is_dir() {
        local
    } else {
        bl_runtime::package::resolve_package(package).unwrap_or_else(|| {
            eprintln!("Package '{package}' is not installed. Install it first with `bl install`.");
            process::exit(1);
        })
    };

    let examples = match bl_runtime::package::list_examples(&package_dir) {
        Ok(examples) => examples,
        Err(error) => {
            eprintln!("Error: {error}");
            process::exit(1);
        }
    };

    if let Some(destination) = copy {
        let destination = PathBuf::from(destination);
        match bl_runtime::package::copy_examples(&package_dir, &destination) {
            Ok(path) => {
                println!(
                    "Copied {} example file(s) to {}",
                    examples.len(),
                    path.display()
                )
            }
            Err(error) => {
                eprintln!("Error: {error}");
                process::exit(1);
            }
        }
        return;
    }

    let root = package_dir.join("examples");
    println!("Examples for {package} ({})", root.display());
    for example in examples {
        println!("{}", example.display());
    }
}

fn cmd_import(
    file: &str,
    from: Option<&str>,
    name: Option<&str>,
    output: Option<&str>,
    validate: bool,
    json: bool,
) {
    let from_stdin = file == "-";

    // The name used for format detection and output naming: explicit --name wins,
    // otherwise the source path's file name. Required when reading from stdin.
    let filename = match name {
        Some(n) => n.to_string(),
        None if from_stdin => {
            eprintln!("Reading from stdin requires --name <file.py|.R|.ipynb|.Rmd>");
            process::exit(1);
        }
        None => PathBuf::from(file)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file.to_string()),
    };

    let source = if from_stdin {
        let mut buffer = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer) {
            eprintln!("Error reading stdin: {e}");
            process::exit(1);
        }
        buffer
    } else {
        match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{file}': {e}");
                process::exit(1);
            }
        }
    };

    let lang = match from {
        Some(l) => l.to_lowercase(),
        None => match import::detect_language(Path::new(&filename)) {
            Some(l) => l.to_string(),
            None => {
                eprintln!(
                    "Cannot detect language from '{filename}'.\n\
                     Use --from python, r, ipynb, or rmd"
                );
                process::exit(1);
            }
        },
    };

    let lang = match import::normalize_format(&lang) {
        Some(format) => format.to_string(),
        None => {
            eprintln!("Unsupported import format '{lang}'. Supported: python, r, ipynb, rmd");
            process::exit(1);
        }
    };

    let is_notebook = import::is_notebook_format(&lang);

    if !json {
        eprintln!(
            "Converting {filename} ({lang} → BioLang {})…",
            if is_notebook {
                ".bln notebook"
            } else {
                ".bl script"
            }
        );
    }

    let imported = match import::import_source(&source, &lang, &filename) {
        Ok(imported) => imported,
        Err(error) => {
            eprintln!("Import failed: {error}");
            process::exit(1);
        }
    };

    // JSON mode: emit the full ImportResult (content + validation) and return.
    // Validation diagnostics are carried in the payload, so exit stays 0.
    if json {
        match serde_json::to_string(&imported) {
            Ok(text) => {
                println!("{text}");
                return;
            }
            Err(error) => {
                eprintln!("Cannot serialize import result: {error}");
                process::exit(1);
            }
        }
    }

    let converted = imported.content;

    // Derive a default output path when --output is not given for notebooks
    let derived_output: Option<String> = if is_notebook && output.is_none() {
        let stem = Path::new(&filename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| filename.clone());
        Some(format!("{stem}.bln"))
    } else {
        None
    };

    let out_path = output.or(derived_output.as_deref());

    match out_path {
        Some(out_path) => {
            if let Err(e) = std::fs::write(out_path, &converted) {
                eprintln!("Error writing '{out_path}': {e}");
                process::exit(1);
            }
            eprintln!("Written to {out_path}");
            if is_notebook {
                eprintln!("Run: bl notebook {out_path}");
            } else {
                eprintln!("Run: bl check {out_path}");
            }
        }
        None => {
            print!("{converted}");
        }
    }
    if validate {
        if imported.validation.valid {
            eprintln!(
                "Validation passed ({} {} checked)",
                imported.validation.units_checked,
                if is_notebook { "cells" } else { "script" }
            );
        } else {
            eprintln!(
                "Validation found {} diagnostic{}:",
                imported.validation.diagnostics.len(),
                if imported.validation.diagnostics.len() == 1 {
                    ""
                } else {
                    "s"
                }
            );
            for diagnostic in &imported.validation.diagnostics {
                eprintln!(
                    "{}:{}:{}: {}",
                    diagnostic.unit, diagnostic.line, diagnostic.column, diagnostic.message
                );
            }
            process::exit(2);
        }
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
