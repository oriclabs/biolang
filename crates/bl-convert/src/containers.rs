use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::ConvertError;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ToolSpec {
    pub name: &'static str,
    pub image: &'static str,
    pub commands: &'static [&'static str],
    pub version_args: &'static [&'static str],
    pub category: &'static str,
    pub description: &'static str,
    pub license: &'static str,
}

pub const TOOL_CATALOG: &[ToolSpec] = &[
    ToolSpec {
        name: "samtools",
        image: "quay.io/biocontainers/samtools:1.24--h9dcdb79_1",
        commands: &["samtools"],
        version_args: &["--version"],
        category: "alignment formats",
        description: "SAM/BAM/CRAM viewing, conversion, sorting and indexing",
        license: "MIT",
    },
    ToolSpec {
        name: "bcftools",
        image: "quay.io/biocontainers/bcftools:1.24--h118bc1c_2",
        commands: &["bcftools"],
        version_args: &["--version"],
        category: "variant formats",
        description: "VCF/BCF conversion, normalization, querying and indexing",
        license: "GPL-3.0-or-later",
    },
    ToolSpec {
        name: "htslib",
        image: "quay.io/biocontainers/htslib:1.24--ha79157c_0",
        commands: &["bgzip", "tabix"],
        version_args: &["--version"],
        category: "compression and indexing",
        description: "BGZF compression and tabix indexing through HTSlib",
        license: "MIT",
    },
    ToolSpec {
        name: "bedtools",
        image: "quay.io/biocontainers/bedtools:2.31.1--h13024bc_3",
        commands: &["bedtools"],
        version_args: &["--version"],
        category: "genomic intervals",
        description: "Interval sorting, merging, intersection and genome arithmetic",
        license: "MIT",
    },
    ToolSpec {
        name: "seqkit",
        image: "quay.io/biocontainers/seqkit:2.13.0--he881be0_0",
        commands: &["seqkit"],
        version_args: &["version"],
        category: "sequence",
        description: "Fast FASTA/FASTQ inspection, filtering and transformation",
        license: "MIT",
    },
    ToolSpec {
        name: "fastp",
        image: "quay.io/biocontainers/fastp:1.3.6--h43da1c4_0",
        commands: &["fastp"],
        version_args: &["--version"],
        category: "quality control",
        description: "FASTQ quality control, adapter trimming and filtering",
        license: "MIT",
    },
    ToolSpec {
        name: "fastqc",
        image: "quay.io/biocontainers/fastqc:0.12.1--hdfd78af_0",
        commands: &["fastqc"],
        version_args: &["--version"],
        category: "quality control",
        description: "Read-quality reports for FASTQ and alignment files",
        license: "GPL-3.0-or-later",
    },
    ToolSpec {
        name: "multiqc",
        image: "quay.io/biocontainers/multiqc:1.35--pyhdfd78af_1",
        commands: &["multiqc"],
        version_args: &["--version"],
        category: "quality control",
        description: "Aggregate results from many bioinformatics tools",
        license: "GPL-3.0-or-later",
    },
    ToolSpec {
        name: "cutadapt",
        image: "quay.io/biocontainers/cutadapt:5.2--py310h6813faf_2",
        commands: &["cutadapt"],
        version_args: &["--version"],
        category: "preprocessing",
        description: "Adapter and primer trimming for sequencing reads",
        license: "MIT",
    },
    ToolSpec {
        name: "minimap2",
        image: "quay.io/biocontainers/minimap2:2.31--h118bc1c_0",
        commands: &["minimap2"],
        version_args: &["--version"],
        category: "alignment",
        description: "Long-read, assembly and spliced-read alignment",
        license: "MIT",
    },
    ToolSpec {
        name: "bowtie2",
        image: "quay.io/biocontainers/bowtie2:2.5.5--ha27dd3b_0",
        commands: &["bowtie2", "bowtie2-build", "bowtie2-inspect"],
        version_args: &["--version"],
        category: "alignment",
        description: "Short-read alignment",
        license: "GPL-3.0-or-later",
    },
    ToolSpec {
        name: "star",
        image: "quay.io/biocontainers/star:2.7.11b--h5ca1c30_8",
        commands: &["STAR"],
        version_args: &["--version"],
        category: "alignment",
        description: "RNA-seq alignment and STARsolo single-cell processing",
        license: "GPL-3.0-or-later",
    },
];

pub fn tool_spec(name: &str) -> Option<&'static ToolSpec> {
    TOOL_CATALOG
        .iter()
        .find(|tool| tool.name.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Apptainer,
    Singularity,
}

impl ContainerRuntime {
    pub fn command(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Apptainer => "apptainer",
            Self::Singularity => "singularity",
        }
    }

    fn is_oci(self) -> bool {
        matches!(self, Self::Docker | Self::Podman)
    }

    pub fn parse(value: &str) -> Result<Self, ConvertError> {
        match value.to_ascii_lowercase().as_str() {
            "docker" => Ok(Self::Docker),
            "podman" => Ok(Self::Podman),
            "apptainer" => Ok(Self::Apptainer),
            "singularity" => Ok(Self::Singularity),
            _ => Err(ConvertError::new(format!(
                "unsupported container runtime '{value}'; use docker, podman, apptainer or singularity"
            ))),
        }
    }
}

impl fmt::Display for ContainerRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.command())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub runtime: ContainerRuntime,
    pub available: bool,
    pub ready: bool,
    pub version: Option<String>,
    pub problem: Option<String>,
}

pub fn runtime_statuses() -> Vec<RuntimeStatus> {
    [
        ContainerRuntime::Docker,
        ContainerRuntime::Podman,
        ContainerRuntime::Apptainer,
        ContainerRuntime::Singularity,
    ]
    .into_iter()
    .map(|runtime| {
        let version_output = Command::new(runtime.command()).arg("--version").output();
        match version_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if runtime.is_oci() {
                    match Command::new(runtime.command()).arg("info").output() {
                        Ok(ready) if ready.status.success() => RuntimeStatus {
                            runtime,
                            available: true,
                            ready: true,
                            version: Some(version),
                            problem: None,
                        },
                        Ok(ready) => RuntimeStatus {
                            runtime,
                            available: true,
                            ready: false,
                            version: Some(version),
                            problem: Some(
                                String::from_utf8_lossy(&ready.stderr)
                                    .lines()
                                    .next()
                                    .unwrap_or("container daemon is not ready")
                                    .to_string(),
                            ),
                        },
                        Err(error) => RuntimeStatus {
                            runtime,
                            available: true,
                            ready: false,
                            version: Some(version),
                            problem: Some(error.to_string()),
                        },
                    }
                } else {
                    RuntimeStatus {
                        runtime,
                        available: true,
                        ready: true,
                        version: Some(version),
                        problem: None,
                    }
                }
            }
            _ => RuntimeStatus {
                runtime,
                available: false,
                ready: false,
                version: None,
                problem: Some("executable not found on PATH".into()),
            },
        }
    })
    .collect()
}

pub fn detect_runtime(preferred: Option<&str>) -> Result<ContainerRuntime, ConvertError> {
    if let Some(preferred) = preferred {
        let runtime = ContainerRuntime::parse(preferred)?;
        let status = runtime_statuses()
            .into_iter()
            .find(|status| status.runtime == runtime)
            .expect("all supported runtimes are probed");
        return if status.ready {
            Ok(runtime)
        } else {
            Err(ConvertError::new(format!(
                "requested runtime '{}' is not ready: {}",
                runtime.command(),
                status.problem.as_deref().unwrap_or("unknown problem")
            )))
        };
    }
    runtime_statuses()
        .into_iter()
        .find(|status| status.ready)
        .map(|status| status.runtime)
        .ok_or_else(|| {
            ConvertError::new(
                "no container runtime found; install Docker, Podman, Apptainer or Singularity",
            )
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolBackend {
    Local,
    Wsl,
    Docker,
    Podman,
    Apptainer,
    Singularity,
}

impl ToolBackend {
    fn from_runtime(runtime: ContainerRuntime) -> Self {
        match runtime {
            ContainerRuntime::Docker => Self::Docker,
            ContainerRuntime::Podman => Self::Podman,
            ContainerRuntime::Apptainer => Self::Apptainer,
            ContainerRuntime::Singularity => Self::Singularity,
        }
    }

    fn runtime(&self) -> Option<ContainerRuntime> {
        match self {
            Self::Docker => Some(ContainerRuntime::Docker),
            Self::Podman => Some(ContainerRuntime::Podman),
            Self::Apptainer => Some(ContainerRuntime::Apptainer),
            Self::Singularity => Some(ContainerRuntime::Singularity),
            Self::Local | Self::Wsl => None,
        }
    }
}

impl fmt::Display for ToolBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Local => "local",
            Self::Wsl => "wsl",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Apptainer => "apptainer",
            Self::Singularity => "singularity",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledTool {
    pub name: String,
    pub backend: ToolBackend,
    pub image: Option<String>,
    pub image_reference: Option<String>,
    pub executable: String,
    pub wsl_distribution: Option<String>,
    pub tool_version: String,
    pub installed_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolManifest {
    schema: String,
    tools: Vec<InstalledTool>,
}

impl Default for ToolManifest {
    fn default() -> Self {
        Self {
            schema: "bl-convert.tools/v1".into(),
            tools: Vec::new(),
        }
    }
}

pub fn state_directory() -> Result<PathBuf, ConvertError> {
    if let Some(path) = env::var_os("BL_CONVERT_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .ok_or_else(|| {
            ConvertError::new(
                "cannot locate the user home directory; set BL_CONVERT_HOME explicitly",
            )
        })?;
    Ok(PathBuf::from(home).join(".biolang").join("convert"))
}

fn manifest_path() -> Result<PathBuf, ConvertError> {
    Ok(state_directory()?.join("tools.json"))
}

fn read_manifest() -> Result<ToolManifest, ConvertError> {
    let path = manifest_path()?;
    if !path.exists() {
        return Ok(ToolManifest::default());
    }
    let file = fs::File::open(&path).map_err(|error| {
        ConvertError::new(format!(
            "cannot read tool manifest '{}': {error}",
            path.display()
        ))
    })?;
    serde_json::from_reader(file).map_err(|error| {
        ConvertError::new(format!(
            "invalid tool manifest '{}': {error}",
            path.display()
        ))
    })
}

fn write_manifest(manifest: &ToolManifest) -> Result<(), ConvertError> {
    let path = manifest_path()?;
    let parent = path.parent().expect("manifest has a parent");
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".tools-")
        .tempfile_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, manifest)?;
    use std::io::Write;
    temporary.write_all(b"\n")?;
    temporary.flush()?;
    temporary.into_temp_path().persist(&path).map_err(|error| {
        ConvertError::new(format!(
            "cannot save tool manifest '{}': {}",
            path.display(),
            error.error
        ))
    })?;
    Ok(())
}

pub fn installed_tools() -> Result<Vec<InstalledTool>, ConvertError> {
    Ok(read_manifest()?.tools)
}

pub fn installed_tool(name: &str) -> Result<Option<InstalledTool>, ConvertError> {
    Ok(read_manifest()?
        .tools
        .into_iter()
        .find(|tool| tool.name.eq_ignore_ascii_case(name)))
}

fn save_installed_tool(installed: InstalledTool) -> Result<InstalledTool, ConvertError> {
    let mut manifest = read_manifest()?;
    manifest
        .tools
        .retain(|tool| !tool.name.eq_ignore_ascii_case(&installed.name));
    manifest.tools.push(installed.clone());
    manifest
        .tools
        .sort_by(|left, right| left.name.cmp(&right.name));
    write_manifest(&manifest)?;
    Ok(installed)
}

fn installation_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn version_text(output: std::process::Output, context: &str) -> Result<String, ConvertError> {
    if !output.status.success() {
        return Err(ConvertError::new(format!(
            "{context} version check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let version = if stdout.is_empty() { stderr } else { stdout };
    if version.is_empty() {
        return Err(ConvertError::new(format!(
            "{context} version check produced no version text"
        )));
    }
    Ok(version)
}

fn find_executable(command: &str) -> Result<PathBuf, ConvertError> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 || command_path.is_absolute() {
        return command_path.canonicalize().map_err(|error| {
            ConvertError::new(format!("cannot resolve executable '{command}': {error}"))
        });
    }
    let path = env::var_os("PATH").ok_or_else(|| ConvertError::new("PATH is not set"))?;
    #[cfg(windows)]
    let extensions = env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .filter(|extension| !extension.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![".EXE".into(), ".CMD".into(), ".BAT".into()]);
    #[cfg(not(windows))]
    let extensions = vec![String::new()];
    for directory in env::split_paths(&path) {
        let direct = directory.join(command);
        if direct.is_file() {
            return direct.canonicalize().map_err(ConvertError::from);
        }
        for extension in &extensions {
            let candidate = directory.join(format!("{command}{extension}"));
            if candidate.is_file() {
                return candidate.canonicalize().map_err(ConvertError::from);
            }
        }
    }
    Err(ConvertError::new(format!(
        "executable '{command}' was not found on PATH"
    )))
}

pub fn register_local_tool(
    name: &str,
    explicit_path: Option<&Path>,
) -> Result<InstalledTool, ConvertError> {
    let spec = tool_spec(name).ok_or_else(|| {
        ConvertError::new(format!(
            "unknown curated tool '{name}'; run 'bl-convert tool catalog'"
        ))
    })?;
    let executable = match explicit_path {
        Some(path) => path.canonicalize().map_err(|error| {
            ConvertError::new(format!(
                "cannot resolve local executable '{}': {error}",
                path.display()
            ))
        })?,
        None => find_executable(spec.commands[0])?,
    };
    if !executable.is_file() {
        return Err(ConvertError::new(format!(
            "local executable '{}' is not a file",
            executable.display()
        )));
    }
    let output = Command::new(&executable)
        .args(spec.version_args)
        .output()
        .map_err(|error| {
            ConvertError::new(format!(
                "cannot run local executable '{}': {error}",
                executable.display()
            ))
        })?;
    let tool_version = version_text(output, spec.name)?;
    save_installed_tool(InstalledTool {
        name: spec.name.into(),
        backend: ToolBackend::Local,
        image: None,
        image_reference: None,
        executable: executable.to_string_lossy().into_owned(),
        wsl_distribution: None,
        tool_version,
        installed_at_unix: installation_time(),
    })
}

fn wsl_command(distribution: Option<&str>) -> Command {
    let mut command = Command::new("wsl");
    if let Some(distribution) = distribution {
        command.args(["--distribution", distribution]);
    }
    command
}

pub fn register_wsl_tool(
    name: &str,
    distribution: Option<&str>,
) -> Result<InstalledTool, ConvertError> {
    let spec = tool_spec(name).ok_or_else(|| {
        ConvertError::new(format!(
            "unknown curated tool '{name}'; run 'bl-convert tool catalog'"
        ))
    })?;
    let located = wsl_command(distribution)
        .args(["--", "which", spec.commands[0]])
        .output()
        .map_err(|error| {
            ConvertError::new(format!(
                "cannot start WSL: {error}; install WSL or choose --local/container installation"
            ))
        })?;
    if !located.status.success() {
        return Err(ConvertError::new(format!(
            "'{}' is not installed in the selected WSL distribution",
            spec.commands[0]
        )));
    }
    let executable = String::from_utf8_lossy(&located.stdout).trim().to_string();
    if executable.is_empty() || !executable.starts_with('/') {
        return Err(ConvertError::new("WSL returned an invalid executable path"));
    }
    let output = wsl_command(distribution)
        .arg("--")
        .arg(&executable)
        .args(spec.version_args)
        .output()
        .map_err(|error| ConvertError::new(format!("cannot verify WSL tool: {error}")))?;
    let tool_version = version_text(output, spec.name)?;
    save_installed_tool(InstalledTool {
        name: spec.name.into(),
        backend: ToolBackend::Wsl,
        image: None,
        image_reference: None,
        executable,
        wsl_distribution: distribution.map(str::to_string),
        tool_version,
        installed_at_unix: installation_time(),
    })
}

fn sif_path(spec: &ToolSpec) -> Result<PathBuf, ConvertError> {
    Ok(state_directory()?.join("images").join(format!(
        "{}-{}.sif",
        spec.name,
        image_tag(spec.image)
    )))
}

fn image_tag(image: &str) -> &str {
    image.rsplit_once(':').map_or("pinned", |(_, tag)| tag)
}

fn runtime_host_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{unc}");
        }
        if let Some(local) = value.strip_prefix(r"\\?\") {
            return local.to_string();
        }
    }
    value.into_owned()
}

fn execute_and_capture(
    runtime: ContainerRuntime,
    image_reference: &str,
    workdir: &Path,
    read_only: bool,
    command: &str,
    arguments: &[impl AsRef<OsStr>],
) -> Result<std::process::Output, ConvertError> {
    let host = workdir.canonicalize().map_err(|error| {
        ConvertError::new(format!(
            "cannot resolve working directory '{}': {error}",
            workdir.display()
        ))
    })?;
    if !host.is_dir() {
        return Err(ConvertError::new(format!(
            "working directory '{}' is not a directory",
            host.display()
        )));
    }
    let mut process = Command::new(runtime.command());
    if runtime.is_oci() {
        process.args(["run", "--rm", "--network", "none"]);
        let suffix = if read_only { ":/data:ro" } else { ":/data" };
        process
            .arg("-v")
            .arg(format!("{}{suffix}", runtime_host_path(&host)));
        process.args(["-w", "/data", image_reference, command]);
    } else {
        process.args(["exec", "--containall", "--cleanenv"]);
        let suffix = if read_only { ":/data:ro" } else { ":/data" };
        process
            .arg("--bind")
            .arg(format!("{}{suffix}", runtime_host_path(&host)))
            .args(["--pwd", "/data", image_reference, command]);
    }
    process.args(arguments);
    process.output().map_err(|error| {
        ConvertError::new(format!(
            "cannot start {} container: {error}",
            runtime.command()
        ))
    })
}

fn version_check(
    runtime: ContainerRuntime,
    image_reference: &str,
    spec: &ToolSpec,
) -> Result<String, ConvertError> {
    let current = env::current_dir()?;
    let output = execute_and_capture(
        runtime,
        image_reference,
        &current,
        true,
        spec.commands[0],
        spec.version_args,
    )?;
    version_text(output, &format!("{} image", spec.name))
}

pub fn install_tool(
    name: &str,
    preferred_runtime: Option<&str>,
) -> Result<InstalledTool, ConvertError> {
    let spec = tool_spec(name).ok_or_else(|| {
        ConvertError::new(format!(
            "unknown curated tool '{name}'; run 'bl-convert tool catalog'"
        ))
    })?;
    let runtime = detect_runtime(preferred_runtime)?;
    let image_reference = if runtime.is_oci() {
        let status = Command::new(runtime.command())
            .arg("pull")
            .arg(spec.image)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                ConvertError::new(format!("cannot start {} pull: {error}", runtime.command()))
            })?;
        if !status.success() {
            return Err(ConvertError::new(format!(
                "{} failed to pull '{}'",
                runtime.command(),
                spec.image
            )));
        }
        spec.image.to_string()
    } else {
        let destination = sif_path(spec)?;
        let parent = destination.parent().expect("SIF path has a parent");
        fs::create_dir_all(parent)?;
        let status = Command::new(runtime.command())
            .args(["pull", "--force"])
            .arg(&destination)
            .arg(format!("docker://{}", spec.image))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                ConvertError::new(format!("cannot start {} pull: {error}", runtime.command()))
            })?;
        if !status.success() {
            return Err(ConvertError::new(format!(
                "{} failed to pull '{}'",
                runtime.command(),
                spec.image
            )));
        }
        destination.to_string_lossy().into_owned()
    };
    let tool_version = version_check(runtime, &image_reference, spec)?;
    let installed = InstalledTool {
        name: spec.name.into(),
        backend: ToolBackend::from_runtime(runtime),
        image: Some(spec.image.into()),
        image_reference: Some(image_reference),
        executable: spec.commands[0].into(),
        wsl_distribution: None,
        tool_version,
        installed_at_unix: installation_time(),
    };
    save_installed_tool(installed)
}

pub fn remove_tool(name: &str, purge: bool) -> Result<InstalledTool, ConvertError> {
    let mut manifest = read_manifest()?;
    let index = manifest
        .tools
        .iter()
        .position(|tool| tool.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| ConvertError::new(format!("tool '{name}' is not installed")))?;
    let installed = manifest.tools[index].clone();
    if purge {
        let runtime = installed.backend.runtime().ok_or_else(|| {
            ConvertError::new(
                "--purge only removes managed container images; local and WSL software must be removed with its own package manager",
            )
        })?;
        let image_reference = installed.image_reference.as_deref().ok_or_else(|| {
            ConvertError::new("container installation is missing its image reference")
        })?;
        if runtime.is_oci() {
            let status = Command::new(runtime.command())
                .args(["image", "rm"])
                .arg(image_reference)
                .status()?;
            if !status.success() {
                return Err(ConvertError::new(format!(
                    "{} could not remove '{}'",
                    runtime.command(),
                    image_reference
                )));
            }
        } else {
            let target = PathBuf::from(image_reference);
            let images = state_directory()?.join("images");
            let canonical_images = images.canonicalize()?;
            let canonical_target = target.canonicalize()?;
            if !canonical_target.starts_with(&canonical_images) {
                return Err(ConvertError::new(
                    "refusing to delete a SIF image outside BL Convert's managed image directory",
                ));
            }
            fs::remove_file(canonical_target)?;
        }
    }
    manifest.tools.remove(index);
    write_manifest(&manifest)?;
    Ok(installed)
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtraMount {
    pub host: PathBuf,
    pub container: String,
    pub read_only: bool,
}

impl ExtraMount {
    pub fn parse(value: &str) -> Result<Self, ConvertError> {
        let (host, container) = value
            .split_once('=')
            .ok_or_else(|| ConvertError::new("--mount must use HOST=/container/path[:ro|:rw]"))?;
        let (container, read_only) = if let Some(path) = container.strip_suffix(":ro") {
            (path, true)
        } else if let Some(path) = container.strip_suffix(":rw") {
            (path, false)
        } else {
            (container, true)
        };
        let reserved = ["/", "/proc", "/sys", "/dev", "/etc", "/run", "/data"];
        if !container.starts_with('/')
            || container.split('/').any(|component| component == "..")
            || reserved
                .iter()
                .any(|path| container == *path || container.starts_with(&format!("{path}/")))
        {
            return Err(ConvertError::new(
                "container mount target must be an absolute non-system path without '..' and cannot overlap /data",
            ));
        }
        let host = Path::new(host).canonicalize().map_err(|error| {
            ConvertError::new(format!("cannot resolve mount source '{host}': {error}"))
        })?;
        Ok(Self {
            host,
            container: container.to_string(),
            read_only,
        })
    }

    fn binding(&self) -> String {
        format!(
            "{}:{}{}",
            runtime_host_path(&self.host),
            self.container,
            if self.read_only { ":ro" } else { "" }
        )
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RunControls {
    pub read_only: bool,
    pub allow_network: bool,
    pub cpus: Option<f64>,
    pub memory: Option<String>,
    pub mounts: Vec<ExtraMount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolRunReport {
    pub schema: &'static str,
    pub tool: String,
    pub backend: ToolBackend,
    pub tool_version: String,
    pub registered_executable: String,
    pub selected_executable: String,
    pub image: Option<String>,
    pub image_reference: Option<String>,
    pub wsl_distribution: Option<String>,
    pub workdir: PathBuf,
    pub arguments: Vec<String>,
    pub controls: RunControls,
    pub network_policy: String,
    pub exit_code: i32,
    pub elapsed_ms: u128,
}

fn validate_controls(controls: &RunControls) -> Result<(), ConvertError> {
    if controls
        .cpus
        .is_some_and(|cpus| !cpus.is_finite() || cpus <= 0.0 || cpus > 1024.0)
    {
        return Err(ConvertError::new(
            "--cpus must be greater than 0 and at most 1024",
        ));
    }
    if let Some(memory) = controls.memory.as_deref() {
        let split = memory
            .find(|character: char| !character.is_ascii_digit() && character != '.')
            .unwrap_or(memory.len());
        let (number, unit) = memory.split_at(split);
        let number = number.parse::<f64>().ok();
        let valid_unit = matches!(
            unit.to_ascii_lowercase().as_str(),
            "" | "b" | "k" | "kb" | "m" | "mb" | "g" | "gb" | "t" | "tb"
        );
        let invalid_number = match number {
            Some(number) => !number.is_finite() || number <= 0.0,
            None => true,
        };
        if memory.len() > 32 || invalid_number || !valid_unit {
            return Err(ConvertError::new(
                "--memory must be a positive container limit such as 8g or 512m",
            ));
        }
    }
    let mut targets = std::collections::HashSet::new();
    for mount in &controls.mounts {
        if !targets.insert(&mount.container) {
            return Err(ConvertError::new(format!(
                "container mount target '{}' is specified more than once",
                mount.container
            )));
        }
    }
    Ok(())
}

fn local_command(
    installed: &InstalledTool,
    command: &str,
    default_command: &str,
) -> Result<PathBuf, ConvertError> {
    let registered = Path::new(&installed.executable);
    if command == default_command {
        return Ok(registered.to_path_buf());
    }
    let parent = registered
        .parent()
        .ok_or_else(|| ConvertError::new("registered executable has no parent directory"))?;
    let candidate = parent.join(format!("{command}{}", env::consts::EXE_SUFFIX));
    if !candidate.is_file() {
        return Err(ConvertError::new(format!(
            "executable '{command}' was not found beside '{}'",
            registered.display()
        )));
    }
    candidate.canonicalize().map_err(ConvertError::from)
}

fn wsl_workdir(distribution: Option<&str>, host: &Path) -> Result<String, ConvertError> {
    let output = wsl_command(distribution)
        .args(["--", "wslpath", "-a", "-u"])
        .arg(host)
        .output()
        .map_err(|error| {
            ConvertError::new(format!("cannot translate path through WSL: {error}"))
        })?;
    if !output.status.success() {
        return Err(ConvertError::new(format!(
            "WSL could not translate '{}': {}",
            host.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn wsl_resolve_executable(
    distribution: Option<&str>,
    command: &str,
) -> Result<String, ConvertError> {
    let output = wsl_command(distribution)
        .args(["--", "which", command])
        .output()
        .map_err(|error| ConvertError::new(format!("cannot query WSL executable: {error}")))?;
    if !output.status.success() {
        return Err(ConvertError::new(format!(
            "executable '{command}' is not available in the selected WSL distribution"
        )));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !path.starts_with('/') {
        return Err(ConvertError::new("WSL returned an invalid executable path"));
    }
    Ok(path)
}

pub fn run_tool(
    name: &str,
    workdir: &Path,
    executable: Option<&str>,
    controls: &RunControls,
    arguments: &[OsString],
) -> Result<ToolRunReport, ConvertError> {
    let started = Instant::now();
    validate_controls(controls)?;
    let installed = installed_tool(name)?.ok_or_else(|| {
        ConvertError::new(format!(
            "tool '{name}' is not installed; run 'bl-convert tool install {name}'"
        ))
    })?;
    let spec = tool_spec(&installed.name).ok_or_else(|| {
        ConvertError::new(format!(
            "installed tool '{}' is no longer in the curated catalog",
            installed.name
        ))
    })?;
    let command = executable.unwrap_or(spec.commands[0]);
    if !spec.commands.contains(&command) {
        return Err(ConvertError::new(format!(
            "tool '{}' does not expose executable '{}'; allowed: {}",
            spec.name,
            command,
            spec.commands.join(", ")
        )));
    }
    let host = workdir.canonicalize().map_err(|error| {
        ConvertError::new(format!(
            "cannot resolve working directory '{}': {error}",
            workdir.display()
        ))
    })?;
    if !host.is_dir() {
        return Err(ConvertError::new("--workdir must name a directory"));
    }
    let mut process = match &installed.backend {
        ToolBackend::Local => {
            if !controls.mounts.is_empty() || controls.cpus.is_some() || controls.memory.is_some() {
                return Err(ConvertError::new(
                    "--mount, --cpus and --memory apply only to container backends",
                ));
            }
            if controls.read_only {
                return Err(ConvertError::new(
                    "--read-only cannot be enforced for a local executable",
                ));
            }
            let mut local = Command::new(local_command(&installed, command, spec.commands[0])?);
            local.current_dir(&host);
            local
        }
        ToolBackend::Wsl => {
            if !controls.mounts.is_empty() || controls.cpus.is_some() || controls.memory.is_some() {
                return Err(ConvertError::new(
                    "--mount, --cpus and --memory apply only to container backends",
                ));
            }
            if controls.read_only {
                return Err(ConvertError::new(
                    "--read-only cannot be enforced for a WSL executable",
                ));
            }
            let linux_workdir = wsl_workdir(installed.wsl_distribution.as_deref(), &host)?;
            let wsl_executable = if command == spec.commands[0] {
                installed.executable.clone()
            } else {
                wsl_resolve_executable(installed.wsl_distribution.as_deref(), command)?
            };
            let mut wsl = wsl_command(installed.wsl_distribution.as_deref());
            wsl.args(["--cd", &linux_workdir, "--", &wsl_executable]);
            wsl
        }
        backend => {
            let runtime = backend.runtime().expect("container backend has a runtime");
            let image_reference = installed.image_reference.as_deref().ok_or_else(|| {
                ConvertError::new("container installation is missing its image reference")
            })?;
            let mut container = Command::new(runtime.command());
            if runtime.is_oci() {
                container.args(["run", "--rm"]);
                if !controls.allow_network {
                    container.args(["--network", "none"]);
                }
                if let Some(cpus) = controls.cpus {
                    container.arg("--cpus").arg(cpus.to_string());
                }
                if let Some(memory) = &controls.memory {
                    container.arg("--memory").arg(memory);
                }
                let suffix = if controls.read_only {
                    ":/data:ro"
                } else {
                    ":/data"
                };
                container
                    .arg("-v")
                    .arg(format!("{}{suffix}", runtime_host_path(&host)));
                for mount in &controls.mounts {
                    container.arg("-v").arg(mount.binding());
                }
                container.args(["-w", "/data", image_reference, command]);
            } else {
                if controls.cpus.is_some() || controls.memory.is_some() {
                    return Err(ConvertError::new(
                        "--cpus and --memory are currently supported only by Docker and Podman",
                    ));
                }
                let suffix = if controls.read_only {
                    ":/data:ro"
                } else {
                    ":/data"
                };
                container
                    .args(["exec", "--containall", "--cleanenv"])
                    .arg("--bind")
                    .arg(format!("{}{suffix}", runtime_host_path(&host)));
                for mount in &controls.mounts {
                    container.arg("--bind").arg(mount.binding());
                }
                container.args(["--pwd", "/data", image_reference, command]);
            }
            container
        }
    };
    let status = process
        .args(arguments)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            ConvertError::new(format!(
                "cannot run tool '{}' through {}: {error}",
                spec.name, installed.backend
            ))
        })?;
    let exit_code = status.code().unwrap_or(1);
    let network_policy = match installed.backend {
        ToolBackend::Docker | ToolBackend::Podman => {
            if controls.allow_network {
                "runtime-default"
            } else {
                "disabled"
            }
        }
        ToolBackend::Apptainer | ToolBackend::Singularity => "runtime-default",
        ToolBackend::Local | ToolBackend::Wsl => "host",
    };
    Ok(ToolRunReport {
        schema: "bl-convert.tool-run/v1",
        tool: installed.name,
        backend: installed.backend,
        tool_version: installed.tool_version,
        registered_executable: installed.executable,
        selected_executable: command.to_string(),
        image: installed.image,
        image_reference: installed.image_reference,
        wsl_distribution: installed.wsl_distribution,
        workdir: host,
        arguments: arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect(),
        controls: controls.clone(),
        network_policy: network_policy.into(),
        exit_code,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn curated_tool_names_and_images_are_unique_and_pinned() {
        let mut names = HashSet::new();
        let mut images = HashSet::new();
        for tool in TOOL_CATALOG {
            assert!(names.insert(tool.name));
            assert!(images.insert(tool.image));
            assert!(tool.image.starts_with("quay.io/biocontainers/"));
            assert!(!tool.image.ends_with(":latest"));
        }
    }

    #[test]
    fn runtime_names_are_strict() {
        assert_eq!(
            ContainerRuntime::parse("Podman").unwrap(),
            ContainerRuntime::Podman
        );
        assert!(ContainerRuntime::parse("shell").is_err());
    }

    #[test]
    fn extra_mounts_are_explicit_and_protect_reserved_targets() {
        let current = env::current_dir().unwrap();
        let value = format!("{}=/refs:rw", current.display());
        let mount = ExtraMount::parse(&value).unwrap();
        assert_eq!(mount.container, "/refs");
        assert!(!mount.read_only);

        let reserved = format!("{}=/data/reference:ro", current.display());
        assert!(ExtraMount::parse(&reserved).is_err());
    }

    #[test]
    fn resource_limits_are_validated_before_runtime_execution() {
        let valid = RunControls {
            cpus: Some(4.0),
            memory: Some("8g".into()),
            ..RunControls::default()
        };
        validate_controls(&valid).unwrap();

        let invalid = RunControls {
            memory: Some("everything".into()),
            ..RunControls::default()
        };
        assert!(validate_controls(&invalid).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn runtime_paths_remove_windows_verbatim_prefixes() {
        assert_eq!(
            runtime_host_path(Path::new(r"\\?\C:\analysis with spaces")),
            r"C:\analysis with spaces"
        );
        assert_eq!(
            runtime_host_path(Path::new(r"\\?\UNC\server\share\reads")),
            r"\\server\share\reads"
        );
    }
}
