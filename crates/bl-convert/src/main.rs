use bl_convert::{
    convert, supported_pairs, validate_file, ConversionReport, ConvertOptions, Format,
};
use clap::{ArgGroup, Args, Parser, Subcommand};
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(
    name = "bl-convert",
    version,
    about = "Safe biological and tabular format conversion",
    long_about = "A separate BioLang converter with explicit coordinate rules, loss reporting, atomic output and no effect on bl.exe size."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert one file to another.
    Convert(ConvertArgs),
    /// List supported source and target formats.
    Formats,
    /// Detect and validate a file without converting it.
    Inspect(InspectArgs),
    /// Inspect container runtimes and the BL Convert tool installation.
    Doctor(DoctorArgs),
    /// Manage explicitly installed BioContainers tools.
    Tool(ToolArgs),
}

#[derive(Debug, Args)]
struct DoctorArgs {
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolArgs {
    #[command(subcommand)]
    command: ToolCommand,
}

#[derive(Debug, Subcommand)]
enum ToolCommand {
    /// List the curated tools that BL Convert knows how to run.
    Catalog(OutputArgs),
    /// List tools explicitly installed through BL Convert.
    List(OutputArgs),
    /// Pull a pinned BioContainers image and run its version check.
    Install(InstallToolArgs),
    /// Register an existing native or WSL installation without downloading it.
    Register(RegisterToolArgs),
    /// Show the recorded installation for one tool.
    Status(StatusToolArgs),
    /// Unregister a tool; pass --purge to remove its image as well.
    Remove(RemoveToolArgs),
    /// Run an installed tool with arguments passed without a shell.
    Run(RunToolArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InstallToolArgs {
    /// Curated tool name, such as samtools or bcftools.
    name: String,
    /// Require a specific runtime: docker, podman, apptainer or singularity.
    #[arg(long)]
    runtime: Option<String>,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("source")
        .required(true)
        .multiple(false)
        .args(["local", "path", "wsl"])
))]
struct RegisterToolArgs {
    /// Curated tool name, such as samtools or bcftools.
    name: String,
    /// Find the tool on the native PATH.
    #[arg(long, group = "source")]
    local: bool,
    /// Register this exact native executable path.
    #[arg(long, group = "source")]
    path: Option<PathBuf>,
    /// Find the tool inside WSL.
    #[arg(long, group = "source")]
    wsl: bool,
    /// Select a WSL distribution rather than the default distribution.
    #[arg(long, requires = "wsl")]
    distribution: Option<String>,
}

#[derive(Debug, Args)]
struct StatusToolArgs {
    name: String,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RemoveToolArgs {
    name: String,
    /// Also delete the managed SIF or ask Docker/Podman to remove the image.
    #[arg(long)]
    purge: bool,
}

#[derive(Debug, Args)]
struct RunToolArgs {
    name: String,
    /// Select an allowed executable from a multi-command image, such as tabix for htslib.
    #[arg(long)]
    executable: Option<String>,
    /// Directory mounted into the container as /data.
    #[arg(long, default_value = ".")]
    workdir: PathBuf,
    /// Mount /data read-only; useful for inspection commands.
    #[arg(long)]
    read_only: bool,
    /// Permit network access for Docker/Podman (disabled by default).
    #[arg(long)]
    allow_network: bool,
    /// Docker/Podman CPU limit, such as 4 or 1.5.
    #[arg(long)]
    cpus: Option<f64>,
    /// Docker/Podman memory limit, such as 8g or 512m.
    #[arg(long)]
    memory: Option<String>,
    /// Additional mount: HOST=/container/path[:ro|:rw]. Repeat as needed.
    #[arg(long, value_name = "HOST=CONTAINER[:ro|:rw]")]
    mount: Vec<String>,
    /// Save a JSON provenance report after the tool finishes.
    #[arg(long)]
    report: Option<PathBuf>,
    /// Replace an existing tool-run report.
    #[arg(long, requires = "report")]
    force_report: bool,
    /// Tool arguments. Place them after `--`.
    #[arg(last = true, allow_hyphen_values = true)]
    arguments: Vec<OsString>,
}

#[derive(Debug, Args)]
struct ConvertArgs {
    /// Input file. Gzip input is detected from .gz or .bgz.
    input: PathBuf,
    /// Output file. A .gz suffix writes ordinary gzip data.
    output: PathBuf,
    /// Override input format detection.
    #[arg(long)]
    from: Option<String>,
    /// Override output format detection.
    #[arg(long)]
    to: Option<String>,
    /// Replace an existing output file.
    #[arg(long)]
    force: bool,
    /// Parse and convert to a sink without creating the output.
    #[arg(long)]
    dry_run: bool,
    /// GFF/GTF feature type to retain, such as gene or exon.
    #[arg(long)]
    feature: Option<String>,
    /// Preferred GFF/GTF attribute for the BED name column.
    #[arg(long)]
    name_attribute: Option<String>,
    /// FASTA output line width.
    #[arg(long, default_value_t = 80)]
    line_width: usize,
    /// Print the conversion report as JSON.
    #[arg(long)]
    json: bool,
    /// Also save the conversion report as JSON.
    #[arg(long)]
    report: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct InspectArgs {
    input: PathBuf,
    /// Override extension-based detection.
    #[arg(long)]
    from: Option<String>,
    /// Print JSON rather than a short human-readable summary.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct Inspection {
    schema: &'static str,
    input: PathBuf,
    format: Format,
    bytes: u64,
    records: u64,
    valid: bool,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    schema: &'static str,
    state_directory: PathBuf,
    runtimes: Vec<bl_convert::containers::RuntimeStatus>,
    installed_tools: usize,
}

fn parse_format(value: Option<String>, flag: &str) -> Result<Option<Format>, String> {
    value
        .map(|value| {
            Format::from_str(&value).map_err(|error| format!("invalid {flag} value: {error}"))
        })
        .transpose()
}

fn write_report<T: Serialize>(path: &Path, report: &T, force: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create report directory '{}': {error}",
            parent.display()
        )
    })?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".bl-convert-report-")
        .tempfile_in(parent)
        .map_err(|error| {
            format!(
                "cannot create temporary report in '{}': {error}",
                parent.display()
            )
        })?;
    serde_json::to_writer_pretty(&mut temporary, report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;
    temporary
        .write_all(b"\n")
        .and_then(|_| temporary.flush())
        .map_err(|error| format!("cannot finish report: {error}"))?;
    let temporary = temporary.into_temp_path();
    let result = if force {
        temporary.persist(path)
    } else {
        temporary.persist_noclobber(path)
    };
    result.map_err(|error| format!("cannot save report '{}': {}", path.display(), error.error))
}

fn path_identity(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("cannot resolve '{}': {error}", path.display()));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot determine current directory: {error}"))?
            .join(path)
    };
    let parent = absolute
        .parent()
        .filter(|parent| parent.exists())
        .and_then(|parent| parent.canonicalize().ok());
    Ok(match (parent, absolute.file_name()) {
        (Some(parent), Some(name)) => parent.join(name),
        _ => absolute,
    })
}

fn preflight_report(args: &ConvertArgs) -> Result<(), String> {
    let Some(report) = &args.report else {
        return Ok(());
    };
    let report_identity = path_identity(report)?;
    if report_identity == path_identity(&args.input)? {
        return Err("--report cannot overwrite the conversion input".into());
    }
    if report_identity == path_identity(&args.output)? {
        return Err("--report must differ from the conversion output".into());
    }
    if report.is_dir() {
        return Err(format!("report '{}' is a directory", report.display()));
    }
    if report.exists() && !args.force {
        return Err(format!(
            "report '{}' already exists; pass --force to replace it",
            report.display()
        ));
    }
    Ok(())
}

fn human_report(report: &ConversionReport) {
    println!(
        "{} -> {}  {} record(s)  {} ms",
        report.from, report.to, report.records_written, report.elapsed_ms
    );
    if report.dry_run {
        println!("dry run: output was not created");
    } else {
        println!(
            "output: {} ({} bytes, validated)",
            report.output.display(),
            report.output_bytes.unwrap_or(0)
        );
    }
    if report.records_skipped > 0 {
        println!("filtered/skipped: {} record(s)", report.records_skipped);
    }
    println!("lossy: {}", if report.lossy { "yes" } else { "no" });
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
}

fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Formats => {
            println!("FROM     TO       LOSSY");
            for (from, to, lossy) in supported_pairs() {
                println!(
                    "{:<8} {:<8} {}",
                    from.to_string(),
                    to.to_string(),
                    if lossy { "yes" } else { "no" }
                );
            }
            Ok(())
        }
        Command::Convert(args) => {
            preflight_report(&args)?;
            let options = ConvertOptions {
                from: parse_format(args.from, "--from")?,
                to: parse_format(args.to, "--to")?,
                force: args.force,
                dry_run: args.dry_run,
                feature: args.feature,
                name_attribute: args.name_attribute,
                line_width: args.line_width,
            };
            let report =
                convert(&args.input, &args.output, &options).map_err(|error| error.to_string())?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|error| format!("cannot serialize report: {error}"))?
                );
            } else {
                human_report(&report);
            }
            if let Some(path) = args.report {
                write_report(&path, &report, args.force)?;
            }
            Ok(())
        }
        Command::Inspect(args) => {
            if !args.input.is_file() {
                return Err(format!("input '{}' does not exist", args.input.display()));
            }
            let format = parse_format(args.from, "--from")?
                .map(Ok)
                .unwrap_or_else(|| Format::detect(&args.input))?;
            let records = validate_file(&args.input, format).map_err(|error| error.to_string())?;
            let inspection = Inspection {
                schema: "bl-convert.inspection/v1",
                bytes: args
                    .input
                    .metadata()
                    .map_err(|error| error.to_string())?
                    .len(),
                input: args.input,
                format,
                records,
                valid: true,
            };
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inspection).map_err(|error| error.to_string())?
                );
            } else {
                println!("format: {}", inspection.format);
                println!("records: {}", inspection.records);
                println!("bytes: {}", inspection.bytes);
                println!("valid: yes");
            }
            Ok(())
        }
        Command::Doctor(args) => {
            let report = DoctorReport {
                schema: "bl-convert.doctor/v1",
                state_directory: bl_convert::containers::state_directory()
                    .map_err(|error| error.to_string())?,
                runtimes: bl_convert::containers::runtime_statuses(),
                installed_tools: bl_convert::containers::installed_tools()
                    .map_err(|error| error.to_string())?
                    .len(),
            };
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
                );
            } else {
                println!("BL Convert container support");
                for runtime in &report.runtimes {
                    if runtime.ready {
                        println!(
                            "  {:<11} ready      {}",
                            runtime.runtime.to_string(),
                            runtime.version.as_deref().unwrap_or("version unknown")
                        );
                    } else if runtime.available {
                        println!(
                            "  {:<11} unavailable {}",
                            runtime.runtime.to_string(),
                            runtime.problem.as_deref().unwrap_or("runtime is not ready")
                        );
                    } else {
                        println!("  {:<11} not found", runtime.runtime.to_string());
                    }
                }
                println!("  installed tools: {}", report.installed_tools);
                println!("  state: {}", report.state_directory.display());
            }
            Ok(())
        }
        Command::Tool(args) => run_tool_command(args.command),
    }
}

fn run_tool_command(command: ToolCommand) -> Result<(), String> {
    use bl_convert::containers;
    match command {
        ToolCommand::Catalog(args) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(containers::TOOL_CATALOG)
                        .map_err(|error| error.to_string())?
                );
            } else {
                println!("TOOL       CATEGORY                  LICENSE             IMAGE");
                for tool in containers::TOOL_CATALOG {
                    println!(
                        "{:<10} {:<25} {:<19} {}",
                        tool.name, tool.category, tool.license, tool.image
                    );
                    println!("           {}", tool.description);
                }
            }
            Ok(())
        }
        ToolCommand::List(args) => {
            let tools = containers::installed_tools().map_err(|error| error.to_string())?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tools).map_err(|error| error.to_string())?
                );
            } else if tools.is_empty() {
                println!("No tools installed. Try: bl-convert tool install samtools");
            } else {
                println!("TOOL       BACKEND      SOURCE");
                for tool in tools {
                    println!(
                        "{:<10} {:<12} {}",
                        tool.name,
                        tool.backend.to_string(),
                        tool.image.as_deref().unwrap_or(&tool.executable)
                    );
                }
            }
            Ok(())
        }
        ToolCommand::Install(args) => {
            let installed = containers::install_tool(&args.name, args.runtime.as_deref())
                .map_err(|error| error.to_string())?;
            println!("installed: {}", installed.name);
            println!("backend: {}", installed.backend);
            println!(
                "image: {}",
                installed.image.as_deref().unwrap_or("not applicable")
            );
            println!(
                "tool version: {}",
                installed.tool_version.lines().next().unwrap_or("unknown")
            );
            Ok(())
        }
        ToolCommand::Register(args) => {
            let installed = if args.wsl {
                containers::register_wsl_tool(&args.name, args.distribution.as_deref())
            } else {
                let _ = args.local;
                containers::register_local_tool(&args.name, args.path.as_deref())
            }
            .map_err(|error| error.to_string())?;
            println!("registered: {}", installed.name);
            println!("backend: {}", installed.backend);
            println!("executable: {}", installed.executable);
            println!(
                "tool version: {}",
                installed.tool_version.lines().next().unwrap_or("unknown")
            );
            Ok(())
        }
        ToolCommand::Status(args) => {
            let installed = containers::installed_tool(&args.name)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    format!(
                        "tool '{}' is not installed; run 'bl-convert tool install {}'",
                        args.name, args.name
                    )
                })?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&installed).map_err(|error| error.to_string())?
                );
            } else {
                println!("tool: {}", installed.name);
                println!("backend: {}", installed.backend);
                println!(
                    "image: {}",
                    installed.image.as_deref().unwrap_or("not applicable")
                );
                println!(
                    "reference: {}",
                    installed
                        .image_reference
                        .as_deref()
                        .unwrap_or("not applicable")
                );
                println!("executable: {}", installed.executable);
                if let Some(distribution) = &installed.wsl_distribution {
                    println!("WSL distribution: {distribution}");
                }
                println!("version: {}", installed.tool_version);
            }
            Ok(())
        }
        ToolCommand::Remove(args) => {
            let removed = containers::remove_tool(&args.name, args.purge)
                .map_err(|error| error.to_string())?;
            println!("unregistered: {}", removed.name);
            if args.purge {
                println!(
                    "image removed: {}",
                    removed
                        .image_reference
                        .as_deref()
                        .unwrap_or("not applicable")
                );
            } else if removed.image.is_some() {
                println!("image retained; use --purge to remove it explicitly");
            } else {
                println!("external software was not modified");
            }
            Ok(())
        }
        ToolCommand::Run(args) => {
            if let Some(report) = &args.report {
                if report.is_dir() {
                    return Err(format!("report '{}' is a directory", report.display()));
                }
                if report.exists() && !args.force_report {
                    return Err(format!(
                        "report '{}' already exists; pass --force-report to replace it",
                        report.display()
                    ));
                }
            }
            let mounts = args
                .mount
                .iter()
                .map(|mount| containers::ExtraMount::parse(mount))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            let controls = containers::RunControls {
                read_only: args.read_only,
                allow_network: args.allow_network,
                cpus: args.cpus,
                memory: args.memory,
                mounts,
            };
            let outcome = containers::run_tool(
                &args.name,
                &args.workdir,
                args.executable.as_deref(),
                &controls,
                &args.arguments,
            )
            .map_err(|error| error.to_string())?;
            if let Some(report) = &args.report {
                write_report(report, &outcome, args.force_report)?;
            }
            if outcome.exit_code == 0 {
                Ok(())
            } else {
                Err(format!(
                    "tool '{}' exited with code {}",
                    args.name, outcome.exit_code
                ))
            }
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}
