mod events;
mod notebook;
mod update;

use clap::{Parser, Subcommand};
use bl_import as import;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "bl",
    version = env!("CARGO_PKG_VERSION"),
    about = "BioLang — pipe-first bioinformatics DSL"
)]
struct Cli {
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
    },
    /// Parse BioLang files without executing them
    Check {
        /// Paths to one or more .bl files
        #[arg(required = true)]
        files: Vec<String>,
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
        /// Path to the .bln, .bl.md, or .ipynb file
        file: String,
        /// Export format: html
        #[arg(long)]
        export: Option<String>,
        /// Convert Jupyter .ipynb to .bln format (prints to stdout)
        #[arg(long)]
        from_ipynb: bool,
        /// Convert .bln to Jupyter .ipynb format (prints to stdout)
        #[arg(long)]
        to_ipynb: bool,
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
    /// Check the environment and per-capability readiness (native vs container)
    Doctor,
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
    // Spawn on a thread with a larger stack (8 MB) to handle deeply nested
    // scripts (the default 1 MB stack overflows on complex BioLang programs).
    let builder = std::thread::Builder::new()
        .name("bl-main".into())
        .stack_size(64 * 1024 * 1024);
    let handler = builder
        .spawn(|| {
            let cli = Cli::parse();

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
                }) => run_file(&file, verbose, events),
                Some(Commands::Check { files }) => check_files(&files),
                Some(Commands::Notebook {
                    file,
                    export,
                    from_ipynb,
                    to_ipynb,
                }) => {
                    if from_ipynb {
                        notebook::ipynb_to_bln(&file);
                    } else if to_ipynb {
                        notebook::bln_to_ipynb(&file);
                    } else if let Some(fmt) = export {
                        match fmt.as_str() {
                            "html" => notebook::export_html(&file),
                            "typst" | "typ" => notebook::export_typst(&file),
                            "pdf" => notebook::export_pdf(&file),
                            _ => {
                                eprintln!(
                                    "Unknown export format '{fmt}'. Supported: html, typst, pdf"
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
                Some(Commands::Import {
                    file,
                    from,
                    name,
                    output,
                    validate,
                    json,
                }) => {
                    cmd_import(&file, from.as_deref(), name.as_deref(), output.as_deref(), validate, json)
                }
                Some(Commands::Doctor) => print!("{}", bl_runtime::capabilities::doctor_report()),
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

fn run_file(path: &str, verbose: bool, structured_events: bool) {
    let start = Instant::now();
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(error) => fail_run(
            format!("Error reading '{path}': {error}"),
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
        events::emit(serde_json::json!({
            "protocol": "biolang.events/v1",
            "event": "started",
            "path": path,
            "file": filename,
        }));
    } else {
        eprintln!("\x1b[2m▶ running {filename}\x1b[0m");
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
    let output_buffer = structured_events.then(|| {
        let buffer = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        bl_runtime::builtins::set_output_buffer(Some(buffer.clone()));
        buffer
    });
    match interpreter.run(&program) {
        Ok(value) => {
            if structured_events {
                bl_runtime::builtins::set_output_buffer(None);
                if let Some(buffer) = output_buffer {
                    let output = buffer.lock().expect("output buffer").clone();
                    if !output.is_empty() {
                        events::emit(serde_json::json!({
                            "protocol": "biolang.events/v1",
                            "event": "output",
                            "stream": "stdout",
                            "text": output,
                        }));
                    }
                }
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": "result",
                    "value": events::value_to_json(&value),
                }));
                events::emit(serde_json::json!({
                    "protocol": "biolang.events/v1",
                    "event": "finished",
                    "status": "succeeded",
                    "durationMs": start.elapsed().as_millis(),
                }));
            } else {
                bl_runtime::builtins::flush_trailing_newline();
                let elapsed = start.elapsed();
                eprintln!("\x1b[2m✓ done in {elapsed:.2?}\x1b[0m");
            }
            bl_runtime::tempfiles::cleanup_all();
        }
        Err(e) => {
            if structured_events {
                bl_runtime::builtins::set_output_buffer(None);
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
                eprintln!("{}", e.format_with_source(&source));
            }
            bl_runtime::tempfiles::cleanup_all();
            process::exit(1);
        }
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
        eprintln!("Converting {filename} ({lang} → BioLang {})…", if is_notebook { ".bln notebook" } else { ".bl script" });
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
        let stem = Path::new(&filename).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| filename.clone());
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
                if imported.validation.diagnostics.len() == 1 { "" } else { "s" }
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
