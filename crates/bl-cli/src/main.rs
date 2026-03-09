mod notebook;
mod update;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "bl",
    version = "0.2.0",
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
    },
    /// Start the interactive REPL
    Repl,
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
    /// Run a literate notebook (.bln file)
    Notebook {
        /// Path to the .bln or .ipynb file
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
    /// Show version and check for updates
    Version,
    /// Upgrade to the latest release
    Upgrade,
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
                Some(Commands::Run { .. })
                | Some(Commands::Repl)
                | None => {
                    update::check_for_updates_background();
                }
                _ => {}
            }

            match cli.command {
                Some(Commands::Run { file, verbose }) => run_file(&file, verbose),
                Some(Commands::Notebook { file, export, from_ipynb, to_ipynb }) => {
                    if from_ipynb {
                        notebook::ipynb_to_bln(&file);
                    } else if to_ipynb {
                        notebook::bln_to_ipynb(&file);
                    } else if let Some(fmt) = export {
                        match fmt.as_str() {
                            "html" => notebook::export_html(&file),
                            _ => {
                                eprintln!("Unknown export format '{fmt}'. Supported: html");
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
                Some(Commands::Version) => update::cmd_version(),
                Some(Commands::Upgrade) => update::cmd_upgrade(),
                Some(Commands::Repl) | None => start_repl(),
            }
        })
        .expect("failed to spawn main thread");
    handler.join().expect("main thread panicked");
}

fn run_file(path: &str, verbose: bool) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{path}': {e}");
            process::exit(1);
        }
    };

    // Show what we're running
    let display_path = PathBuf::from(path);
    let filename = display_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    eprintln!("\x1b[2m▶ running {filename}\x1b[0m");

    let tokens = match bl_lexer::Lexer::new(&source).tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e.format_with_source(&source));
            process::exit(1);
        }
    };

    let parse_result = match bl_parser::Parser::new(tokens).parse() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e.format_with_source(&source));
            process::exit(1);
        }
    };
    if parse_result.has_errors() {
        for e in &parse_result.errors {
            eprintln!("{}", e.format_with_source(&source));
        }
        process::exit(1);
    }
    let program = parse_result.program;

    let mut interpreter = bl_runtime::Interpreter::new();
    interpreter.verbose = verbose;
    if let Ok(canonical) = std::fs::canonicalize(path) {
        interpreter.set_current_file(Some(canonical));
    } else {
        interpreter.set_current_file(Some(PathBuf::from(path)));
    }
    let start = Instant::now();
    match interpreter.run(&program) {
        Ok(_) => {
            bl_runtime::builtins::flush_trailing_newline();
            let elapsed = start.elapsed();
            eprintln!("\x1b[2m✓ done in {elapsed:.2?}\x1b[0m");
        }
        Err(e) => {
            bl_runtime::builtins::flush_trailing_newline();
            eprintln!("{}", e.format_with_source(&source));
            process::exit(1);
        }
    }
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
        eprintln!(
            "Error: no plugin.json found in '{}'",
            source.display()
        );
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
