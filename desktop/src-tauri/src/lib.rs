use base64::Engine;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_TEXT_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PREVIEW_BYTES: u64 = 1024 * 1024;
const MAX_MEDIA_PREVIEW_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PREVIEW_ROWS: usize = 500;
const MAX_SEARCH_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SEARCH_RESULTS: usize = 200;
const MAX_IMPORT_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TREE_DEPTH: usize = 8;
const MAX_TREE_ENTRIES: usize = 12_000;

struct TerminalSession {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

struct LspProcess {
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
}

struct ConsoleProcess {
    io: Mutex<()>,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    child: Mutex<Child>,
}

struct StudioKernelProcess {
    namespace: String,
    console: Arc<ConsoleProcess>,
    root: PathBuf,
    environment_root: PathBuf,
    bl: PathBuf,
}

struct SomerTunnel {
    child: Arc<Mutex<Child>>,
    local_url: String,
}

struct AppState {
    workspace: Mutex<Option<PathBuf>>,
    workspace_trusted: Mutex<bool>,
    jobs: Mutex<HashMap<u64, Arc<Mutex<Child>>>>,
    terminals: Mutex<HashMap<u64, Arc<TerminalSession>>>,
    console: Mutex<Option<Arc<ConsoleProcess>>>,
    studio_kernel: Mutex<Option<StudioKernelProcess>>,
    lsp: Mutex<Option<LspProcess>>,
    somer_tunnels: Mutex<HashMap<String, SomerTunnel>>,
    next_id: AtomicU64,
}

impl AppState {
    fn new() -> Self {
        Self {
            workspace: Mutex::new(configured_workspace()),
            workspace_trusted: Mutex::new(false),
            jobs: Mutex::new(HashMap::new()),
            terminals: Mutex::new(HashMap::new()),
            console: Mutex::new(None),
            studio_kernel: Mutex::new(None),
            lsp: Mutex::new(None),
            somer_tunnels: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    fn id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    name: String,
    path: String,
    kind: &'static str,
    size: u64,
    children: Vec<FileEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceSnapshot {
    name: String,
    root: String,
    entries: Vec<FileEntry>,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitFileStatus {
    path: String,
    index_status: String,
    worktree_status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitStatusSnapshot {
    available: bool,
    branch: Option<String>,
    files: Vec<GitFileStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentInfo {
    platform: String,
    architecture: String,
    workspace: String,
    bl_path: Option<String>,
    bl_version: Option<String>,
    lsp_available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobOutput {
    job_id: u64,
    stream: &'static str,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobResult {
    job_id: u64,
    value: serde_json::Value,
}

/// One printed value with the source line that produced it, for the editor's
/// inline run annotations.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobTrace {
    job_id: u64,
    entries: Vec<JobTraceEntry>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobTraceEntry {
    line: u32,
    text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobArtifact {
    name: String,
    path: String,
    size: u64,
    media_type: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobArtifacts {
    job_id: u64,
    artifacts: Vec<JobArtifact>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobFinished {
    job_id: u64,
    exit_code: Option<i32>,
    duration_ms: u128,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    session_id: u64,
    data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageInfo {
    name: String,
    version: Option<String>,
    source: String,
    installed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchHit {
    path: String,
    line: usize,
    column: usize,
    preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DataPreview {
    kind: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    sequence: Option<String>,
    sequences: Vec<SequenceRecord>,
    content: Option<String>,
    summary: Vec<String>,
    truncated: bool,
    total_bytes: u64,
    provenance: Option<FileProvenance>,
    /// Quality metrics for formats where a table of raw lines says nothing.
    metrics: Option<bl_qc::PreviewMetrics>,
}

#[derive(Clone, Serialize)]
struct SequenceRecord {
    name: String,
    sequence: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileProvenance {
    path: String,
    format: String,
    size: u64,
    modified_ms: Option<u128>,
    imported_from: Option<String>,
    imported_at_ms: Option<u128>,
    sha256: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRecord {
    path: String,
    imported_from: String,
    imported_at_ms: u128,
    size: u64,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileChecksum {
    path: String,
    size: u64,
    modified_ms: Option<u128>,
    sha256: Option<String>,
    checksum_status: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowDocument {
    schema_version: u32,
    name: String,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
}

#[derive(Deserialize)]
struct WorkflowNode {
    id: String,
    operation: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    parameters: Vec<WorkflowParameter>,
    #[serde(default)]
    strategy: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowParameter {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct WorkflowEdge {
    from: String,
    to: String,
}

fn configured_workspace() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("BIOLANG_WORKSPACE") {
        let candidate = PathBuf::from(path);
        if candidate.is_dir() {
            return Some(candidate.canonicalize().unwrap_or(candidate));
        }
    }
    None
}

fn workspace_root(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable".to_string())?
        .clone()
        .ok_or_else(|| "Open a workspace first".to_string())
}

fn require_trusted_workspace(state: &State<'_, AppState>) -> Result<(), String> {
    let trusted = *state
        .workspace_trusted
        .lock()
        .map_err(|_| "Workspace trust state is unavailable".to_string())?;
    if trusted {
        Ok(())
    } else {
        Err("Trust this workspace before running native tools".into())
    }
}

fn somer_secret_entry(profile_id: &str) -> Result<keyring::Entry, String> {
    if profile_id.is_empty() || profile_id.len() > 200 {
        return Err("Invalid SOMER profile id".into());
    }
    keyring::Entry::new("org.biolang.desktop.somer", profile_id)
        .map_err(|error| format!("Credential store is unavailable: {error}"))
}

#[tauri::command]
fn get_somer_secret(profile_id: String) -> Result<Option<String>, String> {
    let entry = somer_secret_entry(&profile_id)?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Cannot read SOMER credential: {error}")),
    }
}

#[tauri::command]
fn set_somer_secret(profile_id: String, secret: String) -> Result<(), String> {
    if secret.is_empty() {
        return delete_somer_secret(profile_id);
    }
    somer_secret_entry(&profile_id)?
        .set_password(&secret)
        .map_err(|error| format!("Cannot save SOMER credential: {error}"))
}

#[tauri::command]
fn delete_somer_secret(profile_id: String) -> Result<(), String> {
    let entry = somer_secret_entry(&profile_id)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Cannot remove SOMER credential: {error}")),
    }
}

#[tauri::command]
fn start_somer_tunnel(
    profile_id: String,
    ssh_host: String,
    ssh_user: String,
    ssh_port: u16,
    remote_host: String,
    remote_port: u16,
    identity_file: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    require_trusted_workspace(&state)?;
    if profile_id.is_empty()
        || ssh_host.trim().is_empty()
        || ssh_user.trim().is_empty()
        || remote_host.trim().is_empty()
        || ssh_port == 0
        || remote_port == 0
    {
        return Err("Complete all SSH tunnel fields".into());
    }
    if let Some(existing) = state
        .somer_tunnels
        .lock()
        .map_err(|_| "SOMER tunnel state is unavailable")?
        .get(&profile_id)
    {
        if existing
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok())
            .flatten()
            .is_none()
        {
            return Ok(existing.local_url.clone());
        }
    }
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("Cannot allocate a local tunnel port: {error}"))?;
    let local_port = listener
        .local_addr()
        .map_err(|error| format!("Cannot inspect the local tunnel port: {error}"))?
        .port();
    drop(listener);

    let mut command = Command::new("ssh");
    command
        .args([
            "-N",
            "-L",
            &format!("127.0.0.1:{local_port}:{remote_host}:{remote_port}"),
            "-p",
            &ssh_port.to_string(),
            "-o",
            "BatchMode=yes",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "ServerAliveInterval=30",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(identity) = identity_file.filter(|value| !value.trim().is_empty()) {
        command.arg("-i").arg(identity);
    }
    command.arg(format!("{ssh_user}@{ssh_host}"));
    let mut child = command
        .spawn()
        .map_err(|error| format!("Cannot start SSH tunnel: {error}"))?;
    thread::sleep(Duration::from_millis(180));
    if let Some(status) = child
        .try_wait()
        .map_err(|error| format!("Cannot inspect SSH tunnel: {error}"))?
    {
        return Err(format!("SSH tunnel exited with {status}"));
    }
    let local_url = format!("http://127.0.0.1:{local_port}");
    let tunnel = SomerTunnel {
        child: Arc::new(Mutex::new(child)),
        local_url: local_url.clone(),
    };
    let mut tunnels = state
        .somer_tunnels
        .lock()
        .map_err(|_| "SOMER tunnel state is unavailable")?;
    if let Some(previous) = tunnels.insert(profile_id, tunnel) {
        if let Ok(mut child) = previous.child.lock() {
            let _ = child.kill();
        }
    }
    Ok(local_url)
}

#[tauri::command]
fn stop_somer_tunnel(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(tunnel) = state
        .somer_tunnels
        .lock()
        .map_err(|_| "SOMER tunnel state is unavailable")?
        .remove(&profile_id)
    {
        if let Ok(mut child) = tunnel.child.lock() {
            let _ = child.kill();
        }
    }
    Ok(())
}

fn restrict_workspace(state: &State<'_, AppState>) -> Result<(), String> {
    *state
        .workspace_trusted
        .lock()
        .map_err(|_| "Workspace trust state is unavailable".to_string())? = false;
    shutdown_processes(state.inner());
    Ok(())
}

fn relative_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn resolve_existing_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("Cannot access {}: {error}", candidate.display()))?;
    if !canonical.starts_with(root) {
        return Err("Path is outside the active workspace".into());
    }
    Ok(canonical)
}

fn build_tree(
    root: &Path,
    directory: &Path,
    depth: usize,
    count: &mut usize,
    truncated: &mut bool,
) -> Vec<FileEntry> {
    if depth > MAX_TREE_DEPTH || *count >= MAX_TREE_ENTRIES {
        *truncated = true;
        return Vec::new();
    }

    let mut paths = match fs::read_dir(directory) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    paths.sort_by(|left, right| {
        let left_dir = left.file_type().map(|ty| ty.is_dir()).unwrap_or(false);
        let right_dir = right.file_type().map(|ty| ty.is_dir()).unwrap_or(false);
        right_dir.cmp(&left_dir).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });

    let mut result = Vec::new();
    for entry in paths {
        if *count >= MAX_TREE_ENTRIES {
            *truncated = true;
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            ".git" | "node_modules" | "target" | "dist" | ".idea"
        ) {
            continue;
        }

        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        *count += 1;
        let is_dir = metadata.is_dir();
        let children = if is_dir {
            build_tree(root, &path, depth + 1, count, truncated)
        } else {
            Vec::new()
        };
        result.push(FileEntry {
            name,
            path: relative_display(path.strip_prefix(root).unwrap_or(&path)),
            kind: if is_dir { "directory" } else { "file" },
            size: metadata.len(),
            children,
        });
    }
    result
}

fn command_works(path: &Path, argument: &str) -> bool {
    Command::new(path)
        .arg(argument)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn find_binary(root: &Path, name: &str) -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    if name == "bl" {
        if let Some(value) = std::env::var_os("BIOLANG_BIN") {
            let path = PathBuf::from(value);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    let mut search_roots = vec![root.to_path_buf()];
    if let Ok(current) = std::env::current_dir() {
        search_roots.push(current);
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            search_roots.push(parent.to_path_buf());
        }
    }

    let mut checked = HashSet::new();
    for search_root in search_roots {
        for ancestor in search_root.ancestors().take(8) {
            if !checked.insert(ancestor.to_path_buf()) {
                continue;
            }
            for profile in ["debug", "release"] {
                let candidate = ancestor.join("target").join(profile).join(&executable);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let path = PathBuf::from(&executable);
    if command_works(&path, "--version") {
        Some(path)
    } else {
        None
    }
}

fn biolang_path(root: &Path, bl: &Path) -> Option<std::ffi::OsString> {
    let mut paths = std::env::var_os("BIOLANG_PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut search_roots = vec![root.to_path_buf()];
    if let Ok(current) = std::env::current_dir() {
        search_roots.push(current);
    }
    if let Some(parent) = bl.parent() {
        search_roots.push(parent.to_path_buf());
    }

    for search_root in search_roots {
        for ancestor in search_root.ancestors().take(8) {
            let packages = ancestor.join("packages");
            if packages.is_dir() && !paths.contains(&packages) {
                paths.push(packages);
            }
        }
    }
    std::env::join_paths(paths).ok()
}

/// Credential names the workbench offers to store.
///
/// A fixed list rather than anything the user types: these are the variables
/// `bl-apis` and the LLM builtins actually read, so an entry that is not here
/// would be stored and then silently ignored.
const CREDENTIAL_NAMES: &[&str] = &[
    "NCBI_API_KEY",
    "COSMIC_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "LLM_API_KEY",
    "GITHUB_TOKEN",
    "TELEGRAM_BOT_TOKEN",
];

fn credential_entry(name: &str) -> Result<keyring::Entry, String> {
    if !CREDENTIAL_NAMES.contains(&name) {
        return Err(format!("{name} is not a BioLang credential"));
    }
    keyring::Entry::new("org.biolang.desktop.credentials", name)
        .map_err(|error| format!("Credential storage is unavailable: {error}"))
}

fn credential_value(name: &str) -> Option<String> {
    credential_entry(name)
        .ok()
        .and_then(|entry| entry.get_password().ok())
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialStatus {
    name: String,
    /// True when a value is stored. The value itself is never sent to the UI.
    configured: bool,
    /// True when the surrounding process already exports it, in which case the
    /// stored value is redundant.
    from_environment: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceBuild {
    name: String,
    assets: std::collections::BTreeMap<String, String>,
    /// Which asset paths are missing, so a stale registry is visible.
    missing: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreRequest {
    #[serde(default)]
    packages: std::collections::BTreeMap<String, String>,
    biolang_version: Option<String>,
    source_snapshot: Option<String>,
    entrypoint: Option<String>,
    #[serde(default)]
    inputs: Vec<RestoreInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreInput {
    path: String,
    sha256: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreDrift {
    kind: String,
    name: String,
    recorded: String,
    current: String,
    /// True when the workbench can put this back.
    restorable: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreReport {
    /// True when the workspace was actually inspected, so an empty `drift`
    /// means "nothing changed" rather than "could not check".
    checked: bool,
    drift: Vec<RestoreDrift>,
    /// Why some drift cannot be undone, so the report does not overpromise.
    notes: Vec<String>,
}

/// Installed package versions, read from the workspace package directories.
fn installed_package_versions(root: &Path) -> HashMap<String, String> {
    let mut versions = HashMap::new();
    for directory in [
        root.join("packages"),
        root.join(".biolang").join("packages"),
    ] {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(text) = fs::read_to_string(entry.path().join("biolang.toml")) else {
                continue;
            };
            let Ok(parsed) = text.parse::<toml::Value>() else {
                continue;
            };
            let package = parsed.get("package");
            if let (Some(name), Some(version)) = (
                package
                    .and_then(|value| value.get("name"))
                    .and_then(|v| v.as_str()),
                package
                    .and_then(|value| value.get("version"))
                    .and_then(|v| v.as_str()),
            ) {
                versions.insert(name.to_string(), version.to_string());
            }
        }
    }
    versions
}

/// Compare the workspace as it is now with the state a run recorded.
///
/// This is the question provenance was collected to answer — why the same
/// script gives different numbers than it did last month — and until now the
/// data was only ever stored, never used.
#[tauri::command]
fn compare_run_environment(
    request: RestoreRequest,
    state: State<'_, AppState>,
) -> Result<RestoreReport, String> {
    let root = workspace_root(&state)?;
    let mut drift = Vec::new();
    let mut notes = Vec::new();

    let installed = installed_package_versions(&root);
    for (name, recorded) in &request.packages {
        let current = installed
            .get(name)
            .cloned()
            .unwrap_or_else(|| "not installed".to_string());
        if &current != recorded {
            drift.push(RestoreDrift {
                kind: "package".into(),
                name: name.clone(),
                recorded: recorded.clone(),
                current,
                restorable: true,
            });
        }
    }

    if let Some(recorded) = request.biolang_version.as_deref() {
        let current = find_binary(&root, "bl")
            .and_then(|bl| {
                Command::new(bl)
                    .arg("--version")
                    .output()
                    .ok()
                    .map(|output| {
                        String::from_utf8_lossy(&output.stdout)
                            .split_whitespace()
                            .last()
                            .unwrap_or("unknown")
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "unknown".to_string());
        if current != recorded {
            drift.push(RestoreDrift {
                kind: "biolang".into(),
                name: "BioLang".into(),
                recorded: recorded.to_string(),
                current,
                restorable: false,
            });
            notes.push(
                "The BioLang version cannot be changed from here. Install the recorded release to match it."
                    .into(),
            );
        }
    }

    for input in &request.inputs {
        let Some(recorded) = input.sha256.as_deref() else {
            continue;
        };
        let current = resolve_existing_path(&root, &input.path)
            .ok()
            .and_then(|path| sha256_file(&path).ok())
            .unwrap_or_else(|| "missing".to_string());
        if current != recorded {
            drift.push(RestoreDrift {
                kind: "input".into(),
                name: input.path.clone(),
                recorded: recorded.chars().take(12).collect(),
                current: current.chars().take(12).collect(),
                restorable: false,
            });
        }
    }
    if drift.iter().any(|entry| entry.kind == "input") {
        notes.push(
            "Input data changed. Provenance stores checksums, not the files, so the data has to come from your own archive."
                .into(),
        );
    }

    if let (Some(snapshot), Some(entrypoint)) = (
        request.source_snapshot.as_deref(),
        request.entrypoint.as_deref(),
    ) {
        let current = resolve_existing_path(&root, entrypoint)
            .ok()
            .and_then(|path| fs::read_to_string(path).ok());
        if current.as_deref() != Some(snapshot) {
            drift.push(RestoreDrift {
                kind: "source".into(),
                name: entrypoint.to_string(),
                recorded: "as recorded".into(),
                current: if current.is_some() {
                    "edited since".into()
                } else {
                    "missing".into()
                },
                restorable: true,
            });
        }
    }

    Ok(RestoreReport {
        checked: true,
        drift,
        notes,
    })
}

/// Pin `biolang.toml` to the recorded package versions, and optionally put the
/// recorded script back.
///
/// Deliberately narrow. It does not touch input data or the interpreter version
/// because it cannot, and a restore that silently skipped half the state would
/// be worse than one that says what it did.
#[tauri::command]
fn restore_run_environment(
    request: RestoreRequest,
    restore_source: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let mut done: Vec<String> = Vec::new();

    if !request.packages.is_empty() {
        let manifest_path = root.join("biolang.toml");
        let text = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("Cannot read biolang.toml: {error}"))?;
        let mut manifest: toml::Value = text
            .parse()
            .map_err(|error| format!("Cannot parse biolang.toml: {error}"))?;
        let table = manifest
            .as_table_mut()
            .ok_or_else(|| "biolang.toml is not a table".to_string())?;
        let dependencies = table
            .entry("dependencies".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .ok_or_else(|| "biolang.toml dependencies is not a table".to_string())?;
        for (name, version) in &request.packages {
            dependencies.insert(name.clone(), toml::Value::String(version.clone()));
        }
        fs::write(&manifest_path, manifest.to_string())
            .map_err(|error| format!("Cannot write biolang.toml: {error}"))?;
        done.push(format!(
            "pinned {} package(s) in biolang.toml",
            request.packages.len()
        ));
    }

    if restore_source {
        if let (Some(snapshot), Some(entrypoint)) = (
            request.source_snapshot.as_deref(),
            request.entrypoint.as_deref(),
        ) {
            fs::write(root.join(entrypoint), snapshot)
                .map_err(|error| format!("Cannot restore {entrypoint}: {error}"))?;
            done.push(format!("restored {entrypoint} from the run snapshot"));
        }
    }

    if done.is_empty() {
        return Ok("Nothing to restore".into());
    }
    Ok(done.join("; "))
}

#[tauri::command]
fn list_reference_builds() -> Vec<ReferenceBuild> {
    let mut builds: Vec<ReferenceBuild> = bl_refs::load()
        .into_iter()
        .map(|(name, assets)| {
            let missing = assets
                .iter()
                .filter(|(key, path)| {
                    // `description` is prose, not a path.
                    key.as_str() != bl_refs::DESCRIPTION_KEY && !Path::new(path).exists()
                })
                .map(|(key, _)| key.clone())
                .collect();
            ReferenceBuild {
                name,
                assets: assets.into_iter().collect(),
                missing,
            }
        })
        .collect();
    builds.sort_by(|left, right| left.name.cmp(&right.name));
    builds
}

#[tauri::command]
fn save_reference_build(
    name: String,
    assets: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("A reference build needs a name".into());
    }
    let mut registry = bl_refs::load();
    registry.insert(
        name,
        assets
            .into_iter()
            .filter(|(_, value)| !value.trim().is_empty())
            .collect(),
    );
    bl_refs::save(&registry)
        .map_err(|error| format!("Cannot write the reference registry: {error}"))
}

#[tauri::command]
fn delete_reference_build(name: String) -> Result<(), String> {
    let mut registry = bl_refs::load();
    registry.remove(&name);
    bl_refs::save(&registry)
        .map_err(|error| format!("Cannot write the reference registry: {error}"))
}

#[tauri::command]
fn list_credentials() -> Vec<CredentialStatus> {
    CREDENTIAL_NAMES
        .iter()
        .map(|name| CredentialStatus {
            name: (*name).to_string(),
            configured: credential_value(name).is_some(),
            from_environment: std::env::var(name).is_ok_and(|value| !value.is_empty()),
        })
        .collect()
}

#[tauri::command]
fn set_credential(name: String, value: String) -> Result<(), String> {
    if value.is_empty() {
        return delete_credential(name);
    }
    credential_entry(&name)?
        .set_password(&value)
        .map_err(|error| format!("Cannot store {name}: {error}"))
}

#[tauri::command]
fn delete_credential(name: String) -> Result<(), String> {
    let entry = credential_entry(&name)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Cannot remove {name}: {error}")),
    }
}

fn configure_biolang_command(command: &mut Command, root: &Path, bl: &Path) {
    if let Some(path) = biolang_path(root, bl) {
        command.env("BIOLANG_PATH", path);
    }
    // Stored credentials reach BioLang as environment variables because that is
    // what `bl-apis` already reads. A variable exported by the surrounding shell
    // wins, so a workspace-specific key set outside the app is not overridden.
    for name in CREDENTIAL_NAMES {
        if std::env::var(name).is_ok_and(|value| !value.is_empty()) {
            continue;
        }
        if let Some(value) = credential_value(name) {
            command.env(name, value);
        }
    }
}

#[tauri::command]
fn write_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Clipboard is unavailable: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("Cannot copy text: {error}"))
}

#[tauri::command]
fn select_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceSnapshot>, String> {
    let selected = rfd::FileDialog::new().pick_folder();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let canonical = selected
        .canonicalize()
        .map_err(|error| format!("Cannot open workspace: {error}"))?;
    restrict_workspace(&state)?;
    *state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable")? = Some(canonical);
    workspace_snapshot(state)
}

/// Absolute path picker for settings (references, SSH identity). Not scoped to
/// the workspace root — callers store absolute paths intentionally.
#[tauri::command]
fn pick_path(
    title: Option<String>,
    filters: Option<Vec<FileFilter>>,
) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = title {
        dialog = dialog.set_title(title);
    }
    if let Some(filters) = filters {
        for filter in filters {
            if filter.extensions.is_empty() {
                continue;
            }
            let extensions: Vec<&str> = filter
                .extensions
                .iter()
                .map(|extension| extension.as_str())
                .collect();
            dialog = dialog.add_filter(&filter.name, &extensions);
        }
    }
    Ok(dialog
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned()))
}

#[derive(Clone, serde::Deserialize)]
struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

#[tauri::command]
fn open_workspace(path: String, state: State<'_, AppState>) -> Result<WorkspaceSnapshot, String> {
    let selected = PathBuf::from(path);
    if !selected.is_dir() {
        return Err("The recent workspace is no longer available".into());
    }
    let canonical = selected
        .canonicalize()
        .map_err(|error| format!("Cannot open workspace: {error}"))?;
    restrict_workspace(&state)?;
    *state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable")? = Some(canonical);
    workspace_snapshot(state)?.ok_or_else(|| "Cannot open workspace".to_string())
}

#[tauri::command]
fn close_workspace(state: State<'_, AppState>) -> Result<(), String> {
    restrict_workspace(&state)?;
    *state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable")? = None;
    Ok(())
}

#[tauri::command]
fn set_workspace_trust(
    root: String,
    trusted: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let active_root = workspace_root(&state)?;
    let requested_root = PathBuf::from(root)
        .canonicalize()
        .map_err(|error| format!("Cannot verify workspace trust: {error}"))?;
    if requested_root != active_root {
        return Err("Workspace trust request does not match the active workspace".into());
    }
    *state
        .workspace_trusted
        .lock()
        .map_err(|_| "Workspace trust state is unavailable".to_string())? = trusted;
    if !trusted {
        shutdown_processes(state.inner());
    }
    Ok(())
}

#[tauri::command]
fn workspace_snapshot(state: State<'_, AppState>) -> Result<Option<WorkspaceSnapshot>, String> {
    let root = match workspace_root(&state) {
        Ok(root) => root,
        Err(_) => return Ok(None),
    };
    let mut count = 0;
    let mut truncated = false;
    let entries = build_tree(&root, &root, 0, &mut count, &mut truncated);
    Ok(Some(WorkspaceSnapshot {
        name: root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "Workspace".into()),
        root: root.display().to_string(),
        entries,
        truncated,
    }))
}

#[tauri::command]
/// Run a git subcommand in the workspace, returning stdout.
///
/// Paths are passed after `--` so a file named like a flag cannot be read as
/// one, and every write is gated on workspace trust.
fn git(root: &Path, args: &[&str], paths: &[String]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    if !paths.is_empty() {
        command.arg("--");
        command.args(paths);
    }
    let output = command
        .output()
        .map_err(|error| format!("Cannot run git: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if message.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        message
    })
}

#[tauri::command]
fn git_stage(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    if paths.is_empty() {
        return Err("Select at least one file to stage".into());
    }
    git(&root, &["add", "--"], &paths).map(|_| ())
}

#[tauri::command]
fn git_unstage(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    if paths.is_empty() {
        return Err("Select at least one file to unstage".into());
    }
    // `restore --staged` rather than `reset`, so an unstage on a repository with
    // no commits yet does not fail against a missing HEAD.
    git(&root, &["restore", "--staged", "--"], &paths).map(|_| ())
}

#[tauri::command]
fn git_commit(message: String, state: State<'_, AppState>) -> Result<String, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("A commit needs a message".into());
    }
    let staged = git(&root, &["diff", "--cached", "--name-only"], &[])?;
    if staged.trim().is_empty() {
        return Err("Nothing staged to commit".into());
    }
    git(&root, &["commit", "-m", &message], &[])
}

/// Unified diff for one file, staged or unstaged.
#[tauri::command]
fn git_diff(path: String, staged: bool, state: State<'_, AppState>) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    let diff = git(&root, &args, &[path.clone()])?;
    if !diff.trim().is_empty() {
        return Ok(diff);
    }

    // An untracked file has nothing to diff against, and `--no-index` against
    // the null device is unreliable across platforms. Rendering it as all-new
    // here keeps the pane honest without shelling out again.
    let resolved = resolve_existing_path(&root, &path)?;
    let Ok(content) = fs::read_to_string(&resolved) else {
        return Ok(String::new());
    };
    let lines: Vec<&str> = content.lines().collect();
    let mut rendered = format!(
        "--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n",
        lines.len()
    );
    for line in lines {
        rendered.push('+');
        rendered.push_str(line);
        rendered.push('\n');
    }
    Ok(rendered)
}

#[tauri::command]
fn git_status(state: State<'_, AppState>) -> Result<GitStatusSnapshot, String> {
    let root = workspace_root(&state)?;
    let output = match Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .current_dir(&root)
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(_) | Err(_) => {
            return Ok(GitStatusSnapshot {
                available: false,
                branch: None,
                files: Vec::new(),
            })
        }
    };
    let records = String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|record| !record.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < records.len() {
        let record = &records[index];
        if record.len() < 4 {
            index += 1;
            continue;
        }
        let mut status = record.chars();
        let index_status = status.next().unwrap_or(' ').to_string();
        let worktree_status = status.next().unwrap_or(' ').to_string();
        let path = record.get(3..).unwrap_or_default().replace('\\', "/");
        files.push(GitFileStatus {
            path,
            index_status: index_status.clone(),
            worktree_status: worktree_status.clone(),
        });
        if matches!(index_status.as_str(), "R" | "C")
            || matches!(worktree_status.as_str(), "R" | "C")
        {
            index += 1;
        }
        index += 1;
    }
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&root)
        .output()
        .ok()
        .filter(|result| result.status.success())
        .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(GitStatusSnapshot {
        available: true,
        branch,
        files,
    })
}

fn resolve_new_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = root.join(relative);
    if candidate.exists() {
        return Err(format!("{} already exists", relative_display(&candidate)));
    }
    let parent = candidate
        .parent()
        .ok_or_else(|| "Invalid workspace path".to_string())?
        .canonicalize()
        .map_err(|error| format!("Cannot access parent directory: {error}"))?;
    if !parent.starts_with(root) {
        return Err("Path is outside the active workspace".into());
    }
    Ok(candidate)
}

#[tauri::command]
fn create_entry(path: String, kind: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let target = resolve_new_path(&root, &path)?;
    match kind.as_str() {
        "file" => {
            let content = if path.to_lowercase().ends_with(".blflow") {
                "{\n  \"schemaVersion\": 1,\n  \"name\": \"BioLang workflow\",\n  \"nodes\": [],\n  \"edges\": []\n}\n"
            } else if path.to_lowercase().ends_with(".bl.md")
                || path.to_lowercase().ends_with(".bln")
            {
                "# BioLang notebook\n\n```biolang\nprintln(\"Hello from BioLang\")\n```\n"
            } else {
                ""
            };
            fs::write(&target, content).map_err(|error| format!("Cannot create {path}: {error}"))
        }
        "directory" => fs::create_dir(&target)
            .map_err(|error| format!("Cannot create directory {path}: {error}")),
        _ => Err("Entry kind must be file or directory".into()),
    }
}

#[tauri::command]
fn rename_entry(
    path: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if new_name.trim().is_empty()
        || new_name.contains('/')
        || new_name.contains('\\')
        || matches!(new_name.as_str(), "." | "..")
    {
        return Err("Enter a valid file or folder name".into());
    }
    let root = workspace_root(&state)?;
    let source = resolve_existing_path(&root, &path)?;
    let parent = source
        .parent()
        .ok_or_else(|| "Cannot rename the workspace root".to_string())?;
    let destination = parent.join(new_name.trim());
    if destination.exists() {
        return Err(format!("{} already exists", destination.display()));
    }
    fs::rename(&source, &destination).map_err(|error| format!("Cannot rename {path}: {error}"))?;
    Ok(relative_display(
        destination.strip_prefix(&root).unwrap_or(&destination),
    ))
}

/// Move a file or folder into another directory inside the workspace.
///
/// `destination_directory` is relative to the workspace root; empty string means the root.
#[tauri::command]
fn move_entry(
    path: String,
    destination_directory: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let source = resolve_existing_path(&root, &path)?;
    if source == root {
        return Err("Cannot move the workspace root".into());
    }
    let dest_dir = if destination_directory.trim().is_empty() {
        root.clone()
    } else {
        let directory = resolve_existing_path(&root, &destination_directory)?;
        if !directory.is_dir() {
            return Err("Drop target must be a folder".into());
        }
        directory
    };
    if source.is_dir() && dest_dir.starts_with(&source) {
        return Err("Cannot move a folder into itself".into());
    }
    let name = source
        .file_name()
        .ok_or_else(|| "Entry has no name".to_string())?;
    let destination = dest_dir.join(name);
    if destination == source {
        return Ok(relative_display(
            source.strip_prefix(&root).unwrap_or(&source),
        ));
    }
    if destination.exists() {
        return Err(format!(
            "{} already exists in the destination",
            name.to_string_lossy()
        ));
    }
    fs::rename(&source, &destination).map_err(|error| format!("Cannot move {path}: {error}"))?;
    Ok(relative_display(
        destination.strip_prefix(&root).unwrap_or(&destination),
    ))
}

/// Create a new workspace file (parents as needed) and write its full content.
///
/// Used by Explorer OS drag-and-drop imports into `data/` (and other folders).
#[tauri::command]
fn write_new_file(
    path: String,
    content: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let target = root.join(&path);
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid workspace path".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create parent folders for {path}: {error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("Cannot access parent directory: {error}"))?;
    if !parent.starts_with(&root) {
        return Err("Path is outside the active workspace".into());
    }
    if target.exists() {
        return Err(format!("{path} already exists"));
    }
    fs::write(&target, content).map_err(|error| format!("Cannot write {path}: {error}"))?;
    let canonical = target
        .canonicalize()
        .map_err(|error| format!("Cannot verify written file: {error}"))?;
    Ok(relative_display(
        canonical.strip_prefix(&root).unwrap_or(&canonical),
    ))
}

#[tauri::command]
fn delete_entry(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let target = resolve_existing_path(&root, &path)?;
    if target == root {
        return Err("Cannot delete the workspace root".into());
    }
    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|error| format!("Cannot delete {path}: {error}"))
    } else {
        fs::remove_file(&target).map_err(|error| format!("Cannot delete {path}: {error}"))
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination)
        .map_err(|error| format!("Cannot create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_directory(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn duplicate_entry(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let source = resolve_existing_path(&root, &path)?;
    if source == root {
        return Err("Cannot duplicate the workspace root".into());
    }
    let parent = source
        .parent()
        .ok_or_else(|| "Entry has no parent directory".to_string())?;
    let stem = source
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "copy".into());
    let extension = source
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    let mut number = 1;
    let destination = loop {
        let suffix = if number == 1 {
            " copy".to_string()
        } else {
            format!(" copy {number}")
        };
        let candidate = parent.join(format!("{stem}{suffix}{extension}"));
        if !candidate.exists() {
            break candidate;
        }
        number += 1;
    };
    if source.is_dir() {
        copy_directory(&source, &destination)?;
    } else {
        fs::copy(&source, &destination)
            .map_err(|error| format!("Cannot duplicate {path}: {error}"))?;
    }
    Ok(relative_display(
        destination.strip_prefix(&root).unwrap_or(&destination),
    ))
}

#[tauri::command]
fn reveal_entry(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let target = resolve_existing_path(&root, &path)?;
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer.exe");
        command.arg("/select,").arg(&target);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(&target);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(target.parent().unwrap_or(&root));
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot reveal {path}: {error}"))
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Only HTTP and HTTPS documentation links can be opened".into());
    }
    let mut command = if cfg!(windows) {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler").arg(&url);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(&url);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(&url);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot open documentation link: {error}"))
}

/// Build the matching expression for a workspace search.
///
/// Shared by search and replace so the set of hits shown is exactly the set
/// that gets rewritten — a mismatch there silently edits code nobody looked at.
fn search_regex(
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    is_regex: bool,
) -> Result<regex::Regex, String> {
    let body = if is_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let source = if whole_word {
        format!(r"\b(?:{body})\b")
    } else {
        body
    };
    regex::RegexBuilder::new(&source)
        .case_insensitive(!case_sensitive)
        .build()
        .map_err(|error| format!("Invalid search pattern: {error}"))
}

/// Walk the workspace, yielding readable text files under the size cap.
fn each_text_file(root: &Path, directory: &Path, visit: &mut impl FnMut(&Path) -> bool) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return true;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            ".git" | "node_modules" | "target" | "dist" | ".idea"
        ) {
            continue;
        }
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if !each_text_file(root, &path, visit) {
                return false;
            }
            continue;
        }
        if metadata.len() > MAX_SEARCH_FILE_BYTES {
            continue;
        }
        if !visit(&path) {
            return false;
        }
    }
    true
}

#[tauri::command]
fn replace_in_workspace(
    query: String,
    replacement: String,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let root = workspace_root(&state)?;
    let query = query.trim();
    if query.len() < 2 {
        return Ok(0);
    }
    let pattern = search_regex(query, case_sensitive, whole_word, regex)?;
    // A literal search must not let `$1` in the replacement expand: someone
    // replacing a price with "$1" means the text "$1".
    let replacement = if regex {
        replacement.clone()
    } else {
        replacement.replace('$', "$$")
    };

    let mut changed = 0usize;
    let mut failure: Option<String> = None;
    each_text_file(&root, &root, &mut |path| {
        let Ok(content) = fs::read_to_string(path) else {
            return true;
        };
        if !pattern.is_match(&content) {
            return true;
        }
        let next = pattern.replace_all(&content, replacement.as_str());
        match fs::write(path, next.as_ref()) {
            Ok(()) => {
                changed += 1;
                true
            }
            Err(error) => {
                failure = Some(format!("{}: {error}", path.display()));
                false
            }
        }
    });

    match failure {
        Some(error) => Err(error),
        None => Ok(changed),
    }
}

fn search_directory(
    root: &Path,
    directory: &Path,
    pattern: &regex::Regex,
    hits: &mut Vec<SearchHit>,
) {
    if hits.len() >= MAX_SEARCH_RESULTS {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if hits.len() >= MAX_SEARCH_RESULTS {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            ".git" | "node_modules" | "target" | "dist" | ".idea"
        ) {
            continue;
        }
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            search_directory(root, &path, pattern, hits);
            continue;
        }
        if metadata.len() > MAX_SEARCH_FILE_BYTES {
            continue;
        }
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        for (line_index, line) in BufReader::new(file).lines().take(20_000).enumerate() {
            let Ok(line) = line else {
                break;
            };
            if let Some(found) = pattern.find(&line) {
                let column = found.start();
                hits.push(SearchHit {
                    path: relative_display(path.strip_prefix(root).unwrap_or(&path)),
                    line: line_index + 1,
                    column: column + 1,
                    preview: line.trim().chars().take(180).collect(),
                });
                if hits.len() >= MAX_SEARCH_RESULTS {
                    return;
                }
            }
        }
    }
}

#[tauri::command]
fn search_workspace(
    query: String,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
    state: State<'_, AppState>,
) -> Result<Vec<SearchHit>, String> {
    let root = workspace_root(&state)?;
    let query = query.trim();
    if query.len() < 2 {
        return Ok(Vec::new());
    }
    let pattern = search_regex(query, case_sensitive, whole_word, regex)?;
    let mut hits = Vec::new();
    search_directory(&root, &root, &pattern, &mut hits);
    Ok(hits)
}

fn preview_delimited(text: &str, delimiter: u8) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(text.as_bytes());
    let columns = reader
        .headers()
        .map_err(|error| format!("Cannot parse table header: {error}"))?
        .iter()
        .take(50)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rows = reader
        .records()
        .take(MAX_PREVIEW_ROWS)
        .filter_map(Result::ok)
        .map(|record| record.iter().take(50).map(str::to_string).collect())
        .collect();
    Ok((columns, rows))
}

fn preview_fasta(text: &str) -> (Vec<Vec<String>>, Option<String>, Vec<SequenceRecord>) {
    let mut rows = Vec::new();
    let mut sequences = Vec::new();
    let mut current_name = String::new();
    let mut current_sequence = String::new();
    let mut first_sequence = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix('>') {
            if !current_name.is_empty() {
                if first_sequence.is_none() {
                    first_sequence = Some(current_sequence.clone());
                }
                rows.push(vec![
                    current_name.clone(),
                    current_sequence.len().to_string(),
                ]);
                sequences.push(SequenceRecord {
                    name: std::mem::take(&mut current_name),
                    sequence: current_sequence.clone(),
                });
                if rows.len() >= MAX_PREVIEW_ROWS {
                    break;
                }
            }
            current_name = name.to_string();
            current_sequence.clear();
        } else {
            current_sequence.push_str(line.trim());
        }
    }
    if !current_name.is_empty() && rows.len() < MAX_PREVIEW_ROWS {
        if first_sequence.is_none() {
            first_sequence = Some(current_sequence.clone());
        }
        rows.push(vec![current_name, current_sequence.len().to_string()]);
        let name = rows
            .last()
            .and_then(|row| row.first())
            .cloned()
            .unwrap_or_default();
        sequences.push(SequenceRecord {
            name,
            sequence: current_sequence,
        });
    }
    (rows, first_sequence, sequences)
}

fn import_ledger(root: &Path) -> Vec<ImportRecord> {
    let path = root.join(".biolang").join("imports.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn file_provenance(
    root: &Path,
    relative_path: &str,
    format: &str,
    metadata: &fs::Metadata,
) -> FileProvenance {
    let imported = import_ledger(root)
        .into_iter()
        .find(|record| record.path == relative_path);
    FileProvenance {
        path: relative_path.to_string(),
        format: format.to_string(),
        size: metadata.len(),
        modified_ms: metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis()),
        imported_from: imported.as_ref().map(|record| record.imported_from.clone()),
        imported_at_ms: imported.as_ref().map(|record| record.imported_at_ms),
        sha256: imported.map(|record| record.sha256),
    }
}

fn media_preview(
    root: &Path,
    relative_path: &str,
    resolved: &Path,
    extension: &str,
    metadata: &fs::Metadata,
) -> Result<Option<DataPreview>, String> {
    let media = match extension {
        "png" => Some(("image", "image/png")),
        "jpg" | "jpeg" => Some(("image", "image/jpeg")),
        "gif" => Some(("image", "image/gif")),
        "webp" => Some(("image", "image/webp")),
        "svg" => Some(("svg", "image/svg+xml")),
        "pdf" => Some(("pdf", "application/pdf")),
        _ => None,
    };
    let Some((kind, mime)) = media else {
        return Ok(None);
    };
    let total_bytes = metadata.len();
    let mut bytes = Vec::new();
    fs::File::open(resolved)
        .map_err(|error| format!("Cannot open {relative_path}: {error}"))?
        .take(MAX_MEDIA_PREVIEW_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot preview {relative_path}: {error}"))?;
    let content = format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    Ok(Some(DataPreview {
        kind: kind.into(),
        columns: Vec::new(),
        rows: Vec::new(),
        sequence: None,
        sequences: Vec::new(),
        content: Some(content),
        summary: vec![format!("{total_bytes} bytes")],
        truncated: total_bytes > MAX_MEDIA_PREVIEW_BYTES,
        total_bytes,
        provenance: Some(file_provenance(root, relative_path, extension, metadata)),
        metrics: None,
    }))
}

fn build_preview(root: &Path, path: &str) -> Result<DataPreview, String> {
    let resolved = resolve_existing_path(root, path)?;
    let metadata = fs::metadata(&resolved).map_err(|error| error.to_string())?;
    let total_bytes = metadata.len();
    let extension = resolved
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if let Some(preview) = media_preview(root, path, &resolved, &extension, &metadata)? {
        return Ok(preview);
    }
    let mut text = String::new();
    fs::File::open(&resolved)
        .map_err(|error| format!("Cannot open {path}: {error}"))?
        .take(MAX_PREVIEW_BYTES)
        .read_to_string(&mut text)
        .map_err(|error| format!("Cannot preview {path}: {error}"))?;
    let mut preview = DataPreview {
        kind: "text".into(),
        columns: vec!["Line".into(), "Content".into()],
        rows: text
            .lines()
            .take(MAX_PREVIEW_ROWS)
            .enumerate()
            .map(|(index, line)| vec![(index + 1).to_string(), line.to_string()])
            .collect(),
        sequence: None,
        sequences: Vec::new(),
        content: None,
        summary: vec![format!("{total_bytes} bytes")],
        truncated: total_bytes > MAX_PREVIEW_BYTES,
        total_bytes,
        provenance: Some(file_provenance(root, path, &extension, &metadata)),
        metrics: None,
    };
    match extension.as_str() {
        "fasta" | "fa" | "fna" | "faa" => {
            let (rows, sequence, sequences) = preview_fasta(&text);
            preview.kind = "fasta".into();
            preview.columns = vec!["Record".into(), "Length".into()];
            preview
                .summary
                .push(format!("{} records sampled", rows.len()));
            preview.rows = rows;
            preview.sequence = sequence;
            preview.sequences = sequences;
        }
        "fastq" | "fq" => {
            let lines = text.lines().collect::<Vec<_>>();
            preview.kind = "fastq".into();
            preview.columns = vec!["Read".into(), "Length".into(), "Quality length".into()];
            preview.rows = lines
                .chunks(4)
                .take(MAX_PREVIEW_ROWS)
                .filter(|chunk| chunk.len() == 4)
                .map(|chunk| {
                    vec![
                        chunk[0].trim_start_matches('@').to_string(),
                        chunk[1].len().to_string(),
                        chunk[3].len().to_string(),
                    ]
                })
                .collect();
            preview
                .summary
                .push(format!("{} reads sampled", preview.rows.len()));
            preview.metrics = bl_qc::metrics_for("fastq", &text);
        }
        "vcf" => {
            let mut columns = Vec::new();
            let mut rows = Vec::new();
            for line in text.lines() {
                if line.starts_with("#CHROM") {
                    columns = line
                        .trim_start_matches('#')
                        .split('\t')
                        .map(str::to_string)
                        .collect();
                } else if !line.starts_with('#')
                    && !line.is_empty()
                    && rows.len() < MAX_PREVIEW_ROWS
                {
                    rows.push(line.split('\t').map(str::to_string).collect());
                }
            }
            preview.kind = "vcf".into();
            preview.columns = columns;
            preview.rows = rows;
            preview
                .summary
                .push(format!("{} variants sampled", preview.rows.len()));
            preview.metrics = bl_qc::metrics_for("vcf", &text);
        }
        "bed" => {
            preview.kind = "bed".into();
            preview.columns = vec![
                "Chromosome".into(),
                "Start".into(),
                "End".into(),
                "Name".into(),
                "Score".into(),
                "Strand".into(),
            ];
            preview.rows = text
                .lines()
                .filter(|line| !line.starts_with('#') && !line.is_empty())
                .take(MAX_PREVIEW_ROWS)
                .map(|line| line.split('\t').take(6).map(str::to_string).collect())
                .collect();
            preview
                .summary
                .push(format!("{} intervals sampled", preview.rows.len()));
        }
        "gff" | "gff3" | "gtf" => {
            preview.kind = "gff".into();
            preview.columns = vec![
                "Sequence".into(),
                "Source".into(),
                "Feature".into(),
                "Start".into(),
                "End".into(),
                "Score".into(),
                "Strand".into(),
                "Phase".into(),
                "Attributes".into(),
            ];
            preview.rows = text
                .lines()
                .filter(|line| !line.starts_with('#') && !line.is_empty())
                .take(MAX_PREVIEW_ROWS)
                .map(|line| line.split('\t').take(9).map(str::to_string).collect())
                .collect();
            preview
                .summary
                .push(format!("{} features sampled", preview.rows.len()));
        }
        "sam" => {
            preview.kind = "sam".into();
            preview.columns = [
                "QNAME", "FLAG", "RNAME", "POS", "MAPQ", "CIGAR", "RNEXT", "PNEXT", "TLEN", "SEQ",
                "QUAL",
            ]
            .into_iter()
            .map(str::to_string)
            .collect();
            preview.rows = text
                .lines()
                .filter(|line| !line.starts_with('@') && !line.is_empty())
                .take(MAX_PREVIEW_ROWS)
                .map(|line| line.split('\t').take(11).map(str::to_string).collect())
                .collect();
            preview
                .summary
                .push(format!("{} alignments sampled", preview.rows.len()));
        }
        "nwk" | "newick" | "tree" => {
            preview.kind = "newick".into();
            preview.columns = Vec::new();
            preview.rows = Vec::new();
            preview.content = Some(text.trim().to_string());
            let leaves = text
                .split(['(', ')', ',', ':', ';'])
                .filter(|value| {
                    let value = value.trim();
                    !value.is_empty() && value.parse::<f64>().is_err()
                })
                .count();
            preview.summary.push(format!("{leaves} labeled nodes"));
        }
        "pdb" | "ent" => {
            preview.kind = "structure".into();
            preview.columns = vec![
                "Serial".into(),
                "Atom".into(),
                "Residue".into(),
                "Chain".into(),
                "X".into(),
                "Y".into(),
                "Z".into(),
                "Element".into(),
            ];
            preview.rows = text
                .lines()
                .filter(|line| line.starts_with("ATOM  ") || line.starts_with("HETATM"))
                .take(MAX_PREVIEW_ROWS)
                .map(|line| {
                    let field = |start: usize, end: usize| {
                        line.get(start..end).unwrap_or_default().trim().to_string()
                    };
                    vec![
                        field(6, 11),
                        field(12, 16),
                        field(17, 20),
                        field(21, 22),
                        field(30, 38),
                        field(38, 46),
                        field(46, 54),
                        field(76, 78),
                    ]
                })
                .collect();
            preview.content = Some(text);
            preview
                .summary
                .push(format!("{} atoms sampled", preview.rows.len()));
        }
        "cif" | "mmcif" => {
            preview.kind = "structure".into();
            preview.columns = vec!["Line".into(), "Record".into()];
            preview.rows = text
                .lines()
                .filter(|line| line.starts_with("ATOM ") || line.starts_with("HETATM "))
                .take(MAX_PREVIEW_ROWS)
                .enumerate()
                .map(|(index, line)| vec![(index + 1).to_string(), line.to_string()])
                .collect();
            preview.content = Some(text);
            preview
                .summary
                .push(format!("{} atom records sampled", preview.rows.len()));
        }
        "csv" | "tsv" => {
            let (columns, rows) =
                preview_delimited(&text, if extension == "csv" { b',' } else { b'\t' })?;
            preview.kind = "table".into();
            preview.columns = columns;
            preview.rows = rows;
            preview
                .summary
                .push(format!("{} rows sampled", preview.rows.len()));
        }
        "json" => {
            let value: Value = serde_json::from_str(&text)
                .map_err(|error| format!("Cannot parse JSON preview: {error}"))?;
            preview.kind = "json".into();
            preview.columns = vec!["Key".into(), "Value".into()];
            preview.rows = match value {
                Value::Object(values) => values
                    .into_iter()
                    .take(MAX_PREVIEW_ROWS)
                    .map(|(key, value)| vec![key, value.to_string()])
                    .collect(),
                Value::Array(values) => values
                    .into_iter()
                    .take(MAX_PREVIEW_ROWS)
                    .enumerate()
                    .map(|(index, value)| vec![index.to_string(), value.to_string()])
                    .collect(),
                value => vec![vec!["value".into(), value.to_string()]],
            };
        }
        _ => {}
    }
    Ok(preview)
}

#[tauri::command]
fn preview_file(path: String, state: State<'_, AppState>) -> Result<DataPreview, String> {
    let root = workspace_root(&state)?;
    build_preview(&root, &path)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Cannot checksum {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Cannot checksum {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[tauri::command]
fn checksum_workspace_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<FileChecksum>, String> {
    let root = workspace_root(&state)?;
    let checksums = paths
        .into_iter()
        .take(128)
        .filter_map(|path| {
            let resolved = resolve_existing_path(&root, &path).ok()?;
            let metadata = fs::metadata(&resolved).ok()?;
            if !metadata.is_file() {
                return None;
            }
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis());
            let (sha256, checksum_status) = match sha256_file(&resolved) {
                Ok(value) => (Some(value), "complete"),
                Err(_) => (None, "unavailable"),
            };
            Some(FileChecksum {
                path,
                size: metadata.len(),
                modified_ms,
                sha256,
                checksum_status,
            })
        })
        .collect::<Vec<_>>();
    Ok(checksums)
}

fn unique_import_target(directory: &Path, name: &str) -> PathBuf {
    let source = Path::new(name);
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let extension = source.extension().and_then(|value| value.to_str());
    for number in 0..10_000 {
        let candidate_name = if number == 0 {
            name.to_string()
        } else if let Some(extension) = extension {
            format!("{stem} {number}.{extension}")
        } else {
            format!("{stem} {number}")
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem}-imported"))
}

#[tauri::command]
fn import_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let root = workspace_root(&state)?;
    let selected = rfd::FileDialog::new()
        .set_directory(&root)
        .pick_files()
        .unwrap_or_default();
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let destination = root.join("data");
    fs::create_dir_all(&destination)
        .map_err(|error| format!("Cannot create data directory: {error}"))?;
    let destination = destination
        .canonicalize()
        .map_err(|error| format!("Cannot access data directory: {error}"))?;
    if !destination.starts_with(&root) {
        return Err("Import destination is outside the active workspace".into());
    }

    let mut ledger = import_ledger(&root);
    let mut imported = Vec::new();
    for source in selected {
        if !source.is_file() {
            continue;
        }
        let name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("data");
        let target = unique_import_target(&destination, name);
        let size = fs::copy(&source, &target)
            .map_err(|error| format!("Cannot import {}: {error}", source.display()))?;
        let relative = relative_display(target.strip_prefix(&root).unwrap_or(&target));
        ledger.push(ImportRecord {
            path: relative.clone(),
            imported_from: source.display().to_string(),
            imported_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            size,
            sha256: sha256_file(&target)?,
        });
        imported.push(relative);
    }
    let ledger_directory = root.join(".biolang");
    fs::create_dir_all(&ledger_directory)
        .map_err(|error| format!("Cannot create provenance directory: {error}"))?;
    let ledger_text = serde_json::to_string_pretty(&ledger)
        .map_err(|error| format!("Cannot serialize import provenance: {error}"))?;
    fs::write(ledger_directory.join("imports.json"), ledger_text)
        .map_err(|error| format!("Cannot save import provenance: {error}"))?;
    Ok(imported)
}

// ── Code import (Python/R/notebook → BioLang) ────────────────────────────────
//
// The desktop is a *consumer* of BioLang: rather than linking the `bl-import`
// crate, it invokes the installed/bundled `bl` binary and consumes the stable
// `bl import ... --json` contract. These structs mirror bl-import's
// `ImportResult` wire shape (camelCase) so the frontend contract is unchanged.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    source_format: String,
    source_name: String,
    #[serde(default)]
    source_content: String,
    suggested_name: String,
    notebook: bool,
    content: String,
    validation: ValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidationReport {
    valid: bool,
    units_checked: usize,
    diagnostics: Vec<ValidationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidationDiagnostic {
    unit: String,
    line: usize,
    column: usize,
    message: String,
    rendered: String,
}

#[tauri::command]
fn validate_import_code(content: String, notebook: bool) -> Result<ValidationReport, String> {
    if content.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err("Converted BioLang source is too large to validate".into());
    }
    let report = bl_import::validate_biolang(&content, notebook);
    Ok(ValidationReport {
        valid: report.valid,
        units_checked: report.units_checked,
        diagnostics: report
            .diagnostics
            .into_iter()
            .map(|diagnostic| ValidationDiagnostic {
                unit: diagnostic.unit,
                line: diagnostic.line,
                column: diagnostic.column,
                message: diagnostic.message,
                rendered: diagnostic.rendered,
            })
            .collect(),
    })
}

const IMPORT_EXTENSIONS: &[&str] = &["py", "r", "R", "ipynb", "rmd", "Rmd", "RMD"];

/// Locate the `bl` executable via the documented discovery order
/// (`BIOLANG_BIN`, workspace/bundled targets, then PATH).
fn locate_bl(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let root = state
        .workspace
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })
}

/// Parse a `bl import --json` invocation's captured output into an `ImportResult`.
fn parse_import_output(
    status: std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<ImportResult, String> {
    if !status.success() {
        let message = String::from_utf8_lossy(stderr);
        let message = message.trim();
        return Err(if message.is_empty() {
            "BioLang import failed".to_string()
        } else {
            message.to_string()
        });
    }
    serde_json::from_slice(stdout)
        .map_err(|error| format!("Cannot parse BioLang import result: {error}"))
}

#[tauri::command]
fn import_code(state: State<'_, AppState>) -> Result<Option<ImportResult>, String> {
    let selected = rfd::FileDialog::new()
        .add_filter("Python, R, and notebooks", IMPORT_EXTENSIONS)
        .pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let metadata = fs::metadata(&selected)
        .map_err(|error| format!("Cannot inspect {}: {error}", selected.display()))?;
    if metadata.len() > MAX_IMPORT_SOURCE_BYTES {
        return Err(format!(
            "{} is too large to import ({:.1} MB; limit is 32 MB)",
            selected.display(),
            metadata.len() as f64 / 1_048_576.0
        ));
    }
    let bl = locate_bl(&state)?;
    let output = Command::new(&bl)
        .arg("import")
        .arg(&selected)
        .arg("--json")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Cannot run BioLang import: {error}"))?;
    parse_import_output(output.status, &output.stdout, &output.stderr).map(Some)
}

#[tauri::command]
fn import_code_url(url: String, state: State<'_, AppState>) -> Result<ImportResult, String> {
    let parsed = reqwest::Url::parse(url.trim())
        .map_err(|error| format!("Enter a valid script URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Script URLs must use HTTP or HTTPS".into());
    }
    let filename = parsed
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .unwrap_or("imported")
        .to_string();
    let has_supported_extension = Path::new(&filename)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| IMPORT_EXTENSIONS.contains(&ext));
    if !has_supported_extension {
        return Err("The URL must end in .py, .R, .ipynb, or .Rmd".into());
    }
    let bl = locate_bl(&state)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("BioLang-Desktop/0.1")
        .build()
        .map_err(|error| format!("Cannot initialize the downloader: {error}"))?;
    let mut response = client
        .get(parsed)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Cannot download {url}: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMPORT_SOURCE_BYTES)
    {
        return Err("The remote script exceeds the 32 MB import limit".into());
    }
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(MAX_IMPORT_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read the downloaded script: {error}"))?;
    if bytes.len() as u64 > MAX_IMPORT_SOURCE_BYTES {
        return Err("The remote script exceeds the 32 MB import limit".into());
    }
    let source = String::from_utf8(bytes)
        .map_err(|_| "The downloaded script is not valid UTF-8 text".to_string())?;

    // Pipe the downloaded source to `bl import - --name <file> --json`; the CLI
    // detects the format from the name and returns the ImportResult as JSON.
    let mut child = Command::new(&bl)
        .arg("import")
        .arg("-")
        .arg("--name")
        .arg(&filename)
        .arg("--json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Cannot run BioLang import: {error}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "Cannot pipe source to BioLang import".to_string())?
        .write_all(source.as_bytes())
        .map_err(|error| format!("Cannot send source to BioLang import: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("BioLang import did not complete: {error}"))?;
    parse_import_output(output.status, &output.stdout, &output.stderr)
}

fn preview_export(preview: &DataPreview, format: &str) -> Result<Vec<u8>, String> {
    match format {
        "json" => serde_json::to_vec_pretty(preview)
            .map_err(|error| format!("Cannot serialize preview: {error}")),
        "csv" | "tsv" => {
            let mut writer = csv::WriterBuilder::new()
                .delimiter(if format == "csv" { b',' } else { b'\t' })
                .from_writer(Vec::new());
            if !preview.columns.is_empty() {
                writer
                    .write_record(&preview.columns)
                    .map_err(|error| error.to_string())?;
            }
            for row in &preview.rows {
                writer
                    .write_record(row)
                    .map_err(|error| error.to_string())?;
            }
            writer
                .into_inner()
                .map_err(|error| format!("Cannot finish export: {error}"))
        }
        "fasta" => preview
            .sequence
            .as_ref()
            .map(|sequence| format!(">exported_sequence\n{sequence}\n").into_bytes())
            .ok_or_else(|| "This preview has no sequence to export".into()),
        "newick" => preview
            .content
            .as_ref()
            .map(|content| content.as_bytes().to_vec())
            .ok_or_else(|| "This preview has no Newick content to export".into()),
        _ => Err(format!("Unsupported preview export format: {format}")),
    }
}

#[tauri::command]
fn export_preview(
    path: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let root = workspace_root(&state)?;
    let preview = build_preview(&root, &path)?;
    let suggested_stem = Path::new(&path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("preview");
    let selected = rfd::FileDialog::new()
        .set_directory(&root)
        .set_file_name(format!("{suggested_stem}.preview.{format}"))
        .save_file();
    let Some(destination) = selected else {
        return Ok(None);
    };
    let bytes = preview_export(&preview, &format)?;
    fs::write(&destination, bytes)
        .map_err(|error| format!("Cannot export {}: {error}", destination.display()))?;
    Ok(Some(destination.display().to_string()))
}

#[tauri::command]
fn export_text(
    suggested_name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let root = workspace_root(&state)?;
    let safe_name = Path::new(&suggested_name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("biolang-output.log");
    let selected = rfd::FileDialog::new()
        .set_directory(&root)
        .set_file_name(safe_name)
        .save_file();
    let Some(destination) = selected else {
        return Ok(None);
    };
    fs::write(&destination, content)
        .map_err(|error| format!("Cannot export {}: {error}", destination.display()))?;
    Ok(Some(destination.display().to_string()))
}

#[tauri::command]
fn export_binary(
    suggested_name: String,
    content: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let root = workspace_root(&state)?;
    let safe_name = Path::new(&suggested_name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("biolang-run.zip");
    let selected = rfd::FileDialog::new()
        .set_directory(&root)
        .set_file_name(safe_name)
        .save_file();
    let Some(destination) = selected else {
        return Ok(None);
    };
    fs::write(&destination, content)
        .map_err(|error| format!("Cannot export {}: {error}", destination.display()))?;
    Ok(Some(destination.display().to_string()))
}

fn run_history_connection(app: &AppHandle) -> Result<Connection, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Cannot locate application data: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Cannot create application data directory: {error}"))?;
    let connection = Connection::open(directory.join("run-history.sqlite3"))
        .map_err(|error| format!("Cannot open run history: {error}"))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS runs (
               id TEXT PRIMARY KEY,
               started_at INTEGER NOT NULL,
               pinned INTEGER NOT NULL DEFAULT 0,
               payload TEXT NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS runs_started_at ON runs(started_at DESC);",
        )
        .map_err(|error| format!("Cannot initialize run history: {error}"))?;
    Ok(connection)
}

#[tauri::command]
fn load_run_history(app: AppHandle) -> Result<Vec<Value>, String> {
    let connection = run_history_connection(&app)?;
    let mut statement = connection
        .prepare(
            "SELECT payload FROM runs
             ORDER BY pinned DESC, started_at DESC
             LIMIT 500",
        )
        .map_err(|error| format!("Cannot read run history: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Cannot query run history: {error}"))?;
    let mut jobs = Vec::new();
    for row in rows {
        let payload = row.map_err(|error| format!("Cannot read run history row: {error}"))?;
        if let Ok(value) = serde_json::from_str(&payload) {
            jobs.push(value);
        }
    }
    Ok(jobs)
}

#[tauri::command]
fn save_run_history(jobs: Vec<Value>, app: AppHandle) -> Result<(), String> {
    let mut connection = run_history_connection(&app)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Cannot update run history: {error}"))?;
    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    for job in jobs.iter().take(500) {
        let Some(id) = job.get("id").and_then(Value::as_str) else {
            continue;
        };
        let started_at = job
            .get("startedAt")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let pinned = i64::from(job.get("pinned").and_then(Value::as_bool).unwrap_or(false));
        let payload = serde_json::to_string(job)
            .map_err(|error| format!("Cannot serialize run history: {error}"))?;
        transaction
            .execute(
                "INSERT INTO runs(id, started_at, pinned, payload, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                   started_at=excluded.started_at,
                   pinned=excluded.pinned,
                   payload=excluded.payload,
                   updated_at=excluded.updated_at",
                params![id, started_at, pinned, payload, updated_at],
            )
            .map_err(|error| format!("Cannot save run history: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("Cannot commit run history: {error}"))?;
    let _ = app.emit("run-history-changed", ());
    Ok(())
}

#[tauri::command]
fn delete_run_history(job_id: String, app: AppHandle) -> Result<(), String> {
    let connection = run_history_connection(&app)?;
    connection
        .execute("DELETE FROM runs WHERE id = ?1", params![job_id])
        .map_err(|error| format!("Cannot delete run history: {error}"))?;
    let _ = app.emit("run-history-changed", ());
    Ok(())
}

#[tauri::command]
fn read_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let resolved = resolve_existing_path(&root, &path)?;
    let metadata = fs::metadata(&resolved).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_TEXT_FILE_BYTES {
        return Err(format!(
            "{} is too large for the text editor ({:.1} MB). Open it in a data viewer.",
            path,
            metadata.len() as f64 / 1_048_576.0
        ));
    }
    fs::read_to_string(&resolved).map_err(|error| format!("Cannot read {path}: {error}"))
}

#[tauri::command]
fn read_workspace_binary(path: String, state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let root = workspace_root(&state)?;
    let resolved = resolve_existing_path(&root, &path)?;
    let metadata = fs::metadata(&resolved).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Artifact path is not a file".into());
    }
    if metadata.len() > 256 * 1024 * 1024 {
        return Err("Artifacts larger than 256 MiB must be opened from the workspace".into());
    }
    fs::read(&resolved).map_err(|error| format!("Cannot read artifact {path}: {error}"))
}

#[tauri::command]
fn read_workspace_binary_range(
    path: String,
    offset: u64,
    length: usize,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    let root = workspace_root(&state)?;
    let resolved = resolve_existing_path(&root, &path)?;
    let metadata = fs::metadata(&resolved).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Artifact path is not a file".into());
    }
    let length = length.clamp(1, 8 * 1024 * 1024);
    let mut file = fs::File::open(&resolved)
        .map_err(|error| format!("Cannot open artifact {path}: {error}"))?;
    file.seek(SeekFrom::Start(offset.min(metadata.len())))
        .map_err(|error| format!("Cannot seek artifact {path}: {error}"))?;
    let mut bytes = vec![0_u8; length.min(metadata.len().saturating_sub(offset) as usize)];
    file.read_exact(&mut bytes)
        .map_err(|error| format!("Cannot read artifact {path}: {error}"))?;
    Ok(bytes)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonlPage {
    rows: Vec<Vec<Value>>,
    offset: usize,
    limit: usize,
    total_rows: usize,
    filtered_rows: usize,
}

#[tauri::command]
fn read_jsonl_page(
    path: String,
    offset: usize,
    limit: usize,
    search: Option<String>,
    sort_column: Option<usize>,
    descending: bool,
    state: State<'_, AppState>,
) -> Result<JsonlPage, String> {
    let root = workspace_root(&state)?;
    let resolved = resolve_existing_path(&root, &path)?;
    let file = fs::File::open(&resolved)
        .map_err(|error| format!("Cannot open result data {path}: {error}"))?;
    let search = search
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let offset = offset;
    let limit = limit.clamp(1, 1_000);
    let mut total_rows = 0_usize;
    let mut filtered_rows = 0_usize;
    let mut page = Vec::with_capacity(limit);
    let mut sortable = sort_column.map(|_| Vec::new());
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| format!("Cannot read result data {path}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        total_rows += 1;
        if search
            .as_deref()
            .is_some_and(|needle| !line.to_lowercase().contains(needle))
        {
            continue;
        }
        let row = serde_json::from_str::<Vec<Value>>(&line)
            .map_err(|error| format!("Invalid result row {total_rows}: {error}"))?;
        if let Some(rows) = sortable.as_mut() {
            rows.push(row);
        } else {
            if filtered_rows >= offset && page.len() < limit {
                page.push(row);
            }
            filtered_rows += 1;
        }
    }
    if let (Some(column), Some(mut rows)) = (sort_column, sortable) {
        rows.sort_by(|left, right| {
            let scalar = |row: &Vec<Value>| {
                row.get(column)
                    .map(|value| value.get("value").unwrap_or(value))
                    .cloned()
            };
            let left_value = scalar(left);
            let right_value = scalar(right);
            let ordering = match (
                left_value.as_ref().and_then(Value::as_f64),
                right_value.as_ref().and_then(Value::as_f64),
            ) {
                (Some(left), Some(right)) => left.total_cmp(&right),
                _ => left_value
                    .map(|value| value.to_string())
                    .cmp(&right_value.map(|value| value.to_string())),
            };
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        filtered_rows = rows.len();
        page = rows.into_iter().skip(offset).take(limit).collect();
    }
    Ok(JsonlPage {
        rows: page,
        offset,
        limit,
        total_rows,
        filtered_rows,
    })
}

#[tauri::command]
fn write_file(path: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let resolved = resolve_existing_path(&root, &path)?;
    if !resolved.is_file() {
        return Err("Only existing workspace files can be saved in this release".into());
    }
    fs::write(&resolved, content).map_err(|error| format!("Cannot save {path}: {error}"))
}

#[tauri::command]
fn save_file_as(
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let root = workspace_root(&state)?;
    let source = resolve_existing_path(&root, &path)?;
    if !source.is_file() {
        return Err("Only workspace files can be saved as a new file".into());
    }
    let source_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("untitled.bl");
    let selected = rfd::FileDialog::new()
        .set_directory(source.parent().unwrap_or(&root))
        .set_file_name(source_name)
        .save_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let file_name = selected
        .file_name()
        .ok_or_else(|| "Choose a file name inside the active workspace".to_string())?;
    let parent = selected
        .parent()
        .ok_or_else(|| "Choose a destination inside the active workspace".to_string())?
        .canonicalize()
        .map_err(|error| format!("Cannot access the selected directory: {error}"))?;
    if !parent.starts_with(&root) {
        return Err("Save As destinations must be inside the active workspace".into());
    }
    let destination = parent.join(file_name);
    if destination.exists() {
        let canonical = destination
            .canonicalize()
            .map_err(|error| format!("Cannot verify the selected file: {error}"))?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err("The selected destination is not a workspace file".into());
        }
    }
    fs::write(&destination, content)
        .map_err(|error| format!("Cannot save {}: {error}", destination.display()))?;
    let relative = destination
        .strip_prefix(&root)
        .map(relative_display)
        .map_err(|_| "The selected destination is outside the active workspace".to_string())?;
    Ok(Some(relative))
}

#[tauri::command]
fn get_environment(state: State<'_, AppState>) -> Result<EnvironmentInfo, String> {
    let selected = state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable")?
        .clone();
    let root = selected.clone().unwrap_or_else(|| {
        let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if current.file_name().is_some_and(|name| name == "desktop") {
            current.parent().unwrap_or(&current).to_path_buf()
        } else {
            current
        }
    });
    let bl_path = find_binary(&root, "bl");
    let lsp_available = find_binary(&root, "bl-lsp").is_some()
        || bl_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|parent| {
                parent
                    .join(if cfg!(windows) {
                        "bl-lsp.exe"
                    } else {
                        "bl-lsp"
                    })
                    .is_file()
            })
            .unwrap_or(false);
    let bl_version = bl_path.as_ref().and_then(|path| {
        Command::new(path)
            .arg("--version")
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|version| !version.is_empty())
    });
    Ok(EnvironmentInfo {
        platform: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        workspace: selected
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        bl_path: bl_path.map(|path| path.display().to_string()),
        bl_version,
        lsp_available,
    })
}

fn stream_reader<R: Read + Send + 'static>(
    reader: R,
    app: AppHandle,
    job_id: u64,
    stream: &'static str,
    structured: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if structured {
                        if let Ok(event) = serde_json::from_slice::<serde_json::Value>(&buffer) {
                            if event.get("protocol").and_then(|value| value.as_str())
                                == Some("biolang.events/v1")
                            {
                                match event.get("event").and_then(|value| value.as_str()) {
                                    Some("output") => {
                                        let event_stream = event
                                            .get("stream")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or("stdout");
                                        let stream = if event_stream == "stderr" {
                                            "stderr"
                                        } else {
                                            "stdout"
                                        };
                                        let data = event
                                            .get("text")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or_default()
                                            .to_string();
                                        let _ = app.emit(
                                            "job-output",
                                            JobOutput {
                                                job_id,
                                                stream,
                                                data,
                                            },
                                        );
                                    }
                                    Some("result") => {
                                        if let Some(value) = event.get("value") {
                                            if value.get("kind").and_then(|kind| kind.as_str())
                                                != Some("nil")
                                            {
                                                let _ = app.emit(
                                                    "job-result",
                                                    JobResult {
                                                        job_id,
                                                        value: value.clone(),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                    Some("trace") => {
                                        // Emitted one per printed value. They
                                        // are forwarded singly rather than
                                        // batched so annotations appear as a
                                        // long run progresses.
                                        if let (Some(line), Some(text)) = (
                                            event.get("line").and_then(|value| value.as_u64()),
                                            event.get("text").and_then(|value| value.as_str()),
                                        ) {
                                            let _ = app.emit(
                                                "job-trace",
                                                JobTrace {
                                                    job_id,
                                                    entries: vec![JobTraceEntry {
                                                        line: line as u32,
                                                        text: text.to_string(),
                                                    }],
                                                },
                                            );
                                        }
                                    }
                                    Some("error") => {
                                        let data = event
                                            .get("message")
                                            .and_then(|value| value.as_str())
                                            .unwrap_or("BioLang execution failed")
                                            .to_string();
                                        let _ = app.emit(
                                            "job-output",
                                            JobOutput {
                                                job_id,
                                                stream: "stderr",
                                                data: format!("{data}\n"),
                                            },
                                        );
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                        }
                    }
                    let _ = app.emit(
                        "job-output",
                        JobOutput {
                            job_id,
                            stream,
                            data: String::from_utf8_lossy(&buffer).to_string(),
                        },
                    );
                }
            }
        }
    })
}

fn start_biolang_job(
    path: String,
    command: &str,
    allowed: &[&str],
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let script = resolve_existing_path(&root, &path)?;
    let lowercase = path.to_lowercase();
    if !allowed
        .iter()
        .any(|extension| lowercase.ends_with(extension))
    {
        return Err(format!("Unsupported file type for BioLang {command}"));
    }
    let bl = find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;

    spawn_biolang_job(script, command, root, bl, None, app, state)
}

fn spawn_biolang_job(
    script: PathBuf,
    command: &str,
    root: PathBuf,
    bl: PathBuf,
    cleanup_path: Option<PathBuf>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let job_id = state.id();
    let files_before = workspace_file_snapshot(&root);
    let mut process = Command::new(&bl);
    configure_biolang_command(&mut process, &root, &bl);
    let structured = command == "run";
    process
        .arg(command)
        .arg(&script)
        .args(structured.then_some("--events"))
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    if structured {
        process.env("BIOLANG_RESULT_DIR", format!("results/run-{job_id}"));
    }
    let child = process.spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            if let Some(path) = cleanup_path {
                let _ = fs::remove_file(path);
            }
            return Err(format!("Cannot start BioLang: {error}"));
        }
    };

    let stdout_reader = child
        .stdout
        .take()
        .map(|stdout| stream_reader(stdout, app.clone(), job_id, "stdout", structured));
    let stderr_reader = child
        .stderr
        .take()
        .map(|stderr| stream_reader(stderr, app.clone(), job_id, "stderr", false));

    let child = Arc::new(Mutex::new(child));
    state
        .jobs
        .lock()
        .map_err(|_| "Job state is unavailable")?
        .insert(job_id, child.clone());

    let handle = app.clone();
    thread::spawn(move || {
        let started = Instant::now();
        let mut stdout_reader = stdout_reader;
        let mut stderr_reader = stderr_reader;
        loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut child| child.try_wait().ok())
                .flatten();
            if let Some(status) = status {
                // A process may exit before the reader threads dispatch their
                // final lines. Drain both pipes before the terminal event.
                if let Some(reader) = stdout_reader.take() {
                    let _ = reader.join();
                }
                if let Some(reader) = stderr_reader.take() {
                    let _ = reader.join();
                }
                let artifacts = changed_workspace_files(
                    &root,
                    &files_before,
                    cleanup_path.as_deref().unwrap_or(&script),
                );
                if !artifacts.is_empty() {
                    let _ = handle.emit("job-artifacts", JobArtifacts { job_id, artifacts });
                }
                let _ = handle.emit(
                    "job-finished",
                    JobFinished {
                        job_id,
                        exit_code: status.code(),
                        duration_ms: started.elapsed().as_millis(),
                    },
                );
                if let Some(state) = handle.try_state::<AppState>() {
                    if let Ok(mut jobs) = state.jobs.lock() {
                        jobs.remove(&job_id);
                    }
                }
                if let Some(path) = &cleanup_path {
                    let _ = fs::remove_file(path);
                }
                break;
            }
            thread::sleep(Duration::from_millis(80));
        }
    });

    Ok(job_id)
}

fn workspace_file_snapshot(root: &Path) -> HashMap<PathBuf, (u64, u128)> {
    let mut files = HashMap::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if relative.components().next().is_some_and(|component| {
                matches!(component, std::path::Component::Normal(name) if name == ".git" || name == ".biolang" || name == "target" || name == "node_modules")
            }) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                directories.push(path);
            } else if metadata.is_file() {
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map(|value| value.as_millis())
                    .unwrap_or_default();
                files.insert(relative.to_path_buf(), (metadata.len(), modified));
            }
        }
    }
    files
}

fn changed_workspace_files(
    root: &Path,
    before: &HashMap<PathBuf, (u64, u128)>,
    script: &Path,
) -> Vec<JobArtifact> {
    let script = script.strip_prefix(root).unwrap_or(script);
    let after = workspace_file_snapshot(root);
    let mut artifacts = after
        .into_iter()
        .filter(|(path, metadata)| path != script && before.get(path) != Some(metadata))
        .map(|(path, (size, _))| {
            let display = relative_display(&path);
            JobArtifact {
                name: path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("artifact")
                    .to_string(),
                path: display,
                size,
                media_type: desktop_media_type(&path).map(str::to_string),
            }
        })
        .collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    artifacts
}

fn desktop_media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "pdf" => Some("application/pdf"),
        "html" | "htm" => Some("text/html"),
        "json" => Some("application/json"),
        "jsonl" | "ndjson" => Some("application/x-ndjson"),
        "csv" => Some("text/csv"),
        "tsv" => Some("text/tab-separated-values"),
        "txt" | "log" | "md" | "bl" => Some("text/plain"),
        _ => None,
    }
}

#[tauri::command]
fn run_file(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    start_biolang_job(path, "run", &[".bl"], app, state)
}

#[tauri::command]
fn run_source(source: String, app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    require_trusted_workspace(&state)?;
    if source.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err("Script execution source is too large".into());
    }
    let root = workspace_root(&state)?;
    let bl = find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("biolang-desktop-{}-{nonce}.bl", std::process::id()));
    fs::write(&path, source)
        .map_err(|error| format!("Cannot prepare script execution: {error}"))?;
    spawn_biolang_job(path.clone(), "run", root, bl, Some(path), app, state)
}

#[tauri::command]
fn run_notebook(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    start_biolang_job(path, "notebook", &[".bln", ".bl.md"], app, state)
}

#[tauri::command]
fn run_notebook_source(
    source: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    require_trusted_workspace(&state)?;
    if source.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err("Notebook execution source is too large".into());
    }
    let root = workspace_root(&state)?;
    let bl = find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "biolang-desktop-{}-{nonce}.bl.md",
        std::process::id()
    ));
    fs::write(&path, source)
        .map_err(|error| format!("Cannot prepare notebook execution: {error}"))?;
    spawn_biolang_job(path.clone(), "notebook", root, bl, Some(path), app, state)
}

fn workflow_identifier(value: &str) -> String {
    let mut identifier = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if identifier.is_empty()
        || identifier
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    {
        identifier.insert_str(0, "step_");
    }
    identifier
}

fn workflow_source(document: &WorkflowDocument) -> Result<String, String> {
    if document.schema_version != 1 {
        return Err(format!(
            "Unsupported .blflow schema version {}",
            document.schema_version
        ));
    }
    if document.nodes.len() > 500 {
        return Err("A workflow cannot contain more than 500 nodes".into());
    }
    let mut nodes = HashMap::new();
    let mut generated_ids = HashMap::new();
    for node in &document.nodes {
        if node.id.trim().is_empty() {
            return Err("Every workflow node requires an id".into());
        }
        if nodes.insert(node.id.as_str(), node).is_some() {
            return Err(format!("Duplicate workflow node '{}'", node.id));
        }
        let generated_id = workflow_identifier(&node.id);
        if let Some(previous_id) = generated_ids.insert(generated_id, node.id.as_str()) {
            return Err(format!(
                "Workflow node ids '{previous_id}' and '{}' generate the same BioLang variable",
                node.id
            ));
        }
        let mut operation = node.operation.chars();
        if !operation
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
            || !operation.all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(format!("Invalid BioLang operation '{}'", node.operation));
        }
    }
    let mut indegree = document
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing = document
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), Vec::new()))
        .collect::<HashMap<_, Vec<&str>>>();
    let mut edge_keys = HashSet::new();
    for edge in &document.edges {
        if !nodes.contains_key(edge.from.as_str()) || !nodes.contains_key(edge.to.as_str()) {
            return Err(format!(
                "Workflow edge '{}' -> '{}' references a missing node",
                edge.from, edge.to
            ));
        }
        if edge.from == edge.to {
            return Err(format!(
                "Workflow node '{}' cannot connect to itself",
                edge.from
            ));
        }
        if !edge_keys.insert((edge.from.as_str(), edge.to.as_str())) {
            return Err(format!(
                "Duplicate workflow edge '{}' -> '{}'",
                edge.from, edge.to
            ));
        }
        *indegree.entry(edge.to.as_str()).or_default() += 1;
        outgoing
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }
    let mut queue = document
        .nodes
        .iter()
        .filter(|node| indegree.get(node.id.as_str()) == Some(&0))
        .map(|node| node.id.as_str())
        .collect::<VecDeque<_>>();
    let mut ordered = Vec::new();
    while let Some(id) = queue.pop_front() {
        ordered.push(nodes[id]);
        for next in outgoing.get(id).into_iter().flatten() {
            let remaining = indegree.get_mut(next).expect("edge target was validated");
            *remaining -= 1;
            if *remaining == 0 {
                queue.push_back(next);
            }
        }
    }
    if ordered.len() != document.nodes.len() {
        return Err("Workflow contains a cycle".into());
    }

    let mut source = format!(
        "# Generated workflow: {}\n",
        document.name.replace('\n', " ")
    );
    for node in ordered {
        let id = workflow_identifier(&node.id);
        let incoming = document
            .edges
            .iter()
            .filter(|edge| edge.to == node.id)
            .map(|edge| workflow_identifier(&edge.from))
            .collect::<Vec<_>>();
        let parameters = if node.parameters.is_empty() {
            node.arguments.clone()
        } else {
            node.parameters
                .iter()
                .map(|parameter| {
                    let _ = &parameter.name;
                    parameter.value.clone()
                })
                .filter(|value| !value.trim().is_empty())
                .collect()
        };
        let strategy = node.strategy.as_deref().unwrap_or("standard");
        let expression = match strategy {
            "scatter" => {
                if incoming.len() != 1 {
                    return Err(format!(
                        "Scatter node '{}' requires exactly one input",
                        node.id
                    ));
                }
                let mut args = vec!["item".to_string()];
                args.extend(parameters);
                format!(
                    "{} |> map(|item| {}({}))",
                    incoming[0],
                    node.operation,
                    args.join(", ")
                )
            }
            "gather" => {
                if incoming.is_empty() {
                    return Err(format!(
                        "Gather node '{}' requires at least one input",
                        node.id
                    ));
                }
                format!(
                    "[{}] |> {}({})",
                    incoming.join(", "),
                    node.operation,
                    parameters.join(", ")
                )
            }
            "standard" => {
                if incoming.len() == 1 {
                    format!(
                        "{} |> {}({})",
                        incoming[0],
                        node.operation,
                        parameters.join(", ")
                    )
                } else {
                    let mut args = incoming;
                    args.extend(parameters);
                    format!("{}({})", node.operation, args.join(", "))
                }
            }
            other => return Err(format!("Unknown workflow strategy '{other}'")),
        };
        source.push_str(&format!("let {id} = {expression}\n"));
    }
    let source_nodes = document
        .edges
        .iter()
        .map(|edge| edge.from.as_str())
        .collect::<HashSet<_>>();
    for sink in document
        .nodes
        .iter()
        .filter(|node| !source_nodes.contains(node.id.as_str()))
    {
        source.push_str(&format!("println({})\n", workflow_identifier(&sink.id)));
    }
    Ok(source)
}

#[tauri::command]
fn run_workflow(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    if !path.to_lowercase().ends_with(".blflow") {
        return Err("Only .blflow files can be run as workflows".into());
    }
    let workflow_path = resolve_existing_path(&root, &path)?;
    let document: WorkflowDocument = serde_json::from_str(
        &fs::read_to_string(&workflow_path)
            .map_err(|error| format!("Cannot read {path}: {error}"))?,
    )
    .map_err(|error| format!("Cannot parse {path}: {error}"))?;
    let source = workflow_source(&document)?;
    let generated_directory = root.join(".biolang").join("workflows");
    fs::create_dir_all(&generated_directory)
        .map_err(|error| format!("Cannot create generated workflow directory: {error}"))?;
    let stem = workflow_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("workflow");
    let generated = generated_directory.join(format!("{stem}.bl"));
    fs::write(&generated, source)
        .map_err(|error| format!("Cannot write generated workflow: {error}"))?;
    let relative = relative_display(generated.strip_prefix(&root).unwrap_or(&generated));
    start_biolang_job(relative, "run", &[".bl"], app, state)
}

#[tauri::command]
fn stop_job(job_id: u64, state: State<'_, AppState>) -> Result<(), String> {
    let job = state
        .jobs
        .lock()
        .map_err(|_| "Job state is unavailable")?
        .get(&job_id)
        .cloned()
        .ok_or_else(|| "Job is no longer running".to_string())?;
    let result = job
        .lock()
        .map_err(|_| "Job process is unavailable")?
        .kill()
        .map_err(|error| format!("Cannot stop job: {error}"));
    result
}

fn dependency_source(value: &toml::Value) -> (Option<String>, String) {
    if let Some(version) = value.as_str() {
        return (Some(version.into()), "registry".into());
    }
    if let Some(table) = value.as_table() {
        let version = table
            .get("version")
            .and_then(toml::Value::as_str)
            .map(str::to_string);
        if let Some(path) = table.get("path").and_then(toml::Value::as_str) {
            return (version, format!("path: {path}"));
        }
        if let Some(git) = table.get("git").and_then(toml::Value::as_str) {
            return (version, format!("git: {git}"));
        }
        return (version, "configured".into());
    }
    (None, "unknown".into())
}

#[tauri::command]
fn list_packages(state: State<'_, AppState>) -> Result<Vec<PackageInfo>, String> {
    let root = workspace_root(&state)?;
    let manifest_path = root.join("biolang.toml");
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Cannot read biolang.toml: {error}"))?;
    let manifest = text
        .parse::<toml::Value>()
        .map_err(|error| format!("Invalid biolang.toml: {error}"))?;
    let installed_root = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".biolang")
        .join("packages");

    let mut packages = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .map(|dependencies| {
            dependencies
                .iter()
                .map(|(name, value)| {
                    let (version, source) = dependency_source(value);
                    PackageInfo {
                        name: name.clone(),
                        version,
                        source,
                        installed: installed_root.join(name).is_dir(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

#[tauri::command]
fn install_packages(state: State<'_, AppState>) -> Result<String, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let bl = find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let mut command = Command::new(&bl);
    configure_biolang_command(&mut command, &root, &bl);
    let output = command
        .arg("install")
        .current_dir(&root)
        .output()
        .map_err(|error| format!("Cannot run package installation: {error}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(text)
    } else {
        Err(text.trim().to_string())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestResult {
    file: String,
    name: String,
    label: String,
    passed: bool,
    duration_ms: Option<u64>,
    message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestRunSummary {
    results: Vec<TestResult>,
    passed: usize,
    failed: usize,
    duration_ms: u64,
}

/// Run `bl test --events` over the workspace or a single file.
///
/// Blocking rather than streamed: a suite over analysis code finishes in
/// seconds, and the JSON Lines protocol already gives one clean event per test.
#[tauri::command]
fn run_workspace_tests(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<TestRunSummary, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let bl = find_binary(&root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let mut command = Command::new(&bl);
    configure_biolang_command(&mut command, &root, &bl);
    command.arg("test").arg("--events");
    if let Some(path) = path.as_deref() {
        command.arg(path);
    }
    let output = command
        .current_dir(&root)
        .output()
        .map_err(|error| format!("Cannot run tests: {error}"))?;

    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut duration_ms = 0u64;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let text = |key: &str| {
            event
                .get(key)
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        match event.get("event").and_then(|value| value.as_str()) {
            Some(kind @ ("testPassed" | "testFailed")) => {
                let ok = kind == "testPassed";
                if ok {
                    passed += 1;
                } else {
                    failed += 1;
                }
                results.push(TestResult {
                    file: text("file").unwrap_or_default(),
                    name: text("name").unwrap_or_default(),
                    label: text("label").or_else(|| text("name")).unwrap_or_default(),
                    passed: ok,
                    duration_ms: event.get("durationMs").and_then(|value| value.as_u64()),
                    message: text("message"),
                });
            }
            Some("testFinished") => {
                duration_ms = event
                    .get("durationMs")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default();
            }
            _ => {}
        }
    }

    // A non-zero exit with no parsed results means the runner itself failed,
    // which is a different thing from a test failing and must not look green.
    if results.is_empty() && !output.status.success() {
        let mut message = String::from_utf8_lossy(&output.stderr).to_string();
        if message.trim().is_empty() {
            message = String::from_utf8_lossy(&output.stdout).to_string();
        }
        return Err(message.trim().to_string());
    }

    Ok(TestRunSummary {
        results,
        passed,
        failed,
        duration_ms,
    })
}

fn spawn_console(root: &Path) -> Result<Arc<ConsoleProcess>, String> {
    let bl = find_binary(root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    spawn_console_with_binary(root, root, &bl)
}

fn spawn_console_with_binary(
    root: &Path,
    environment_root: &Path,
    bl: &Path,
) -> Result<Arc<ConsoleProcess>, String> {
    let mut command = Command::new(bl);
    configure_biolang_command(&mut command, environment_root, bl);
    let mut child = command
        .args(["repl", "--json"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot start BioLang Console: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "BioLang Console stdin is unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "BioLang Console stdout is unavailable".to_string())?;
    Ok(Arc::new(ConsoleProcess {
        io: Mutex::new(()),
        stdin: Mutex::new(stdin),
        stdout: Mutex::new(BufReader::new(stdout)),
        child: Mutex::new(child),
    }))
}

fn active_console(state: &AppState, root: &Path) -> Result<Arc<ConsoleProcess>, String> {
    let mut active = state
        .console
        .lock()
        .map_err(|_| "Console state is unavailable")?;
    if let Some(process) = active.as_ref() {
        let running = process
            .child
            .lock()
            .map_err(|_| "Console process is unavailable")?
            .try_wait()
            .map_err(|error| format!("Cannot inspect BioLang Console: {error}"))?
            .is_none();
        if running {
            return Ok(process.clone());
        }
    }
    let process = spawn_console(root)?;
    *active = Some(process.clone());
    Ok(process)
}

fn send_console_request(
    process: &ConsoleProcess,
    id: u64,
    command: &str,
    source: Option<&str>,
) -> Result<Value, String> {
    send_console_payload(
        process,
        id,
        serde_json::json!({
            "id": id,
            "command": command,
            "source": source,
        }),
    )
}

fn send_console_payload(
    process: &ConsoleProcess,
    id: u64,
    request: Value,
) -> Result<Value, String> {
    let _request = process
        .io
        .lock()
        .map_err(|_| "Console request state is unavailable")?;
    {
        let mut stdin = process
            .stdin
            .lock()
            .map_err(|_| "Console stdin is unavailable")?;
        serde_json::to_writer(&mut *stdin, &request)
            .map_err(|error| format!("Cannot encode BioLang Console request: {error}"))?;
        stdin
            .write_all(b"\n")
            .and_then(|_| stdin.flush())
            .map_err(|error| format!("Cannot send BioLang Console request: {error}"))?;
    }

    let mut stdout = process
        .stdout
        .lock()
        .map_err(|_| "Console stdout is unavailable")?;
    loop {
        let mut line = String::new();
        let read = stdout
            .read_line(&mut line)
            .map_err(|error| format!("Cannot read BioLang Console response: {error}"))?;
        if read == 0 {
            return Err("BioLang Console stopped before returning a response".into());
        }
        let Ok(response) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if response.get("protocol").and_then(Value::as_str) == Some("biolang.console/v1")
            && response.get("id").and_then(Value::as_u64) == Some(id)
        {
            return Ok(response);
        }
    }
}

fn console_command(
    command: &str,
    source: Option<&str>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let process = active_console(&state, &root)?;
    send_console_request(&process, state.id(), command, source)
}

#[tauri::command]
fn start_console(state: State<'_, AppState>) -> Result<Value, String> {
    console_command("ping", None, state)
}

#[tauri::command]
fn evaluate_console(source: String, state: State<'_, AppState>) -> Result<Value, String> {
    if source.trim().is_empty() {
        return Err("Enter a BioLang expression to evaluate".into());
    }
    console_command("evaluate", Some(&source), state)
}

#[tauri::command]
fn inspect_console(state: State<'_, AppState>) -> Result<Value, String> {
    console_command("inspect", None, state)
}

#[tauri::command]
fn reset_console(state: State<'_, AppState>) -> Result<Value, String> {
    console_command("reset", None, state)
}

#[tauri::command]
fn stop_console(state: State<'_, AppState>) -> Result<(), String> {
    let process = state
        .console
        .lock()
        .map_err(|_| "Console state is unavailable")?
        .take();
    if let Some(process) = process {
        process
            .child
            .lock()
            .map_err(|_| "Console process is unavailable")?
            .kill()
            .map_err(|error| format!("Cannot stop BioLang Console: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
fn close_console(state: State<'_, AppState>) -> Result<(), String> {
    stop_console(state)
}

const MAX_STUDIO_NATIVE_DATA_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const MAX_STUDIO_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudioAttachedFile {
    path: String,
    contents: String,
    #[allow(dead_code)]
    size: u64,
    sha256: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioNativeFile {
    path: String,
    size: u64,
    sha256: String,
    media_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudioRemoteRequest {
    url: String,
    path: String,
    media_type: String,
    expected_bytes: Option<u64>,
    expected_sha256: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioRemoteResult {
    path: String,
    size: u64,
    sha256: String,
    media_type: String,
    source_bytes: u64,
    source_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioExportResult {
    path: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioDocument {
    path: String,
    filename: String,
    contents: String,
    size: u64,
    sha256: String,
    modified_ms: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudioDocumentSaveRequest {
    kind: String,
    path: Option<String>,
    suggested_name: String,
    contents: String,
    expected_sha256: Option<String>,
    overwrite: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioDocumentSaveResult {
    status: &'static str,
    path: String,
    document: Option<StudioDocument>,
    current_sha256: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioDocumentStatus {
    exists: bool,
    changed: bool,
    current_sha256: Option<String>,
    modified_ms: Option<u64>,
}

fn valid_studio_namespace(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn studio_kernel_target(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err("Studio data path must be a safe relative path".into());
    }
    let target = root.join(path);
    let parent = target
        .parent()
        .ok_or_else(|| "Studio data path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create Studio data directory: {error}"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("Cannot access Studio data directory: {error}"))?;
    if !canonical_parent.starts_with(root) {
        return Err("Studio data path escapes its private notebook directory".into());
    }
    if target.exists() {
        let canonical_target = target
            .canonicalize()
            .map_err(|error| format!("Cannot inspect Studio data path: {error}"))?;
        if !canonical_target.starts_with(root) {
            return Err("Studio data path resolves outside its private notebook directory".into());
        }
    }
    Ok(target)
}

fn studio_kernel_console(
    state: &State<'_, AppState>,
    namespace: &str,
) -> Result<(Arc<ConsoleProcess>, PathBuf), String> {
    let kernel = state
        .studio_kernel
        .lock()
        .map_err(|_| "Studio kernel state is unavailable")?;
    let kernel = kernel
        .as_ref()
        .ok_or_else(|| "Studio native kernel is not initialized".to_string())?;
    if kernel.namespace != namespace {
        return Err("This notebook's native kernel is no longer active".into());
    }
    Ok((kernel.console.clone(), kernel.root.clone()))
}

fn studio_file(
    path: &Path,
    relative: &str,
    media_type: Option<&str>,
) -> Result<StudioNativeFile, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
    Ok(StudioNativeFile {
        path: relative_display(Path::new(relative)),
        size: metadata.len(),
        sha256: sha256_file(path)?,
        media_type: media_type
            .map(str::to_string)
            .or_else(|| desktop_media_type(path).map(str::to_string))
            .unwrap_or_else(|| "application/octet-stream".to_string()),
    })
}

fn atomic_copy_into(source: &Path, destination: &Path) -> Result<(), String> {
    let filename = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let temporary = destination.with_file_name(format!(
        ".{filename}.part-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        fs::copy(source, &temporary)
            .map_err(|error| format!("Cannot copy {}: {error}", source.display()))?;
        if destination.exists() {
            if sha256_file(&temporary)? == sha256_file(destination)? {
                fs::remove_file(&temporary).map_err(|error| error.to_string())?;
                return Ok(());
            }
            return Err(format!(
                "{} already contains different data",
                destination.display()
            ));
        }
        fs::rename(&temporary, destination)
            .map_err(|error| format!("Cannot finish {}: {error}", destination.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn studio_document_filter(kind: &str) -> Result<(&'static str, &'static [&'static str]), String> {
    match kind {
        "notebook" => Ok(("BioLang notebook", &["bln", "md"])),
        "workspace" => Ok(("BioLang workspace", &["blw"])),
        _ => Err(format!("Unsupported Studio document kind '{kind}'")),
    }
}

fn validate_studio_document_path(path: &Path, kind: &str) -> Result<(), String> {
    let (_, extensions) = studio_document_filter(kind)?;
    if !path.is_absolute() {
        return Err("Studio document paths must be absolute".into());
    }
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Studio document name is not valid UTF-8".to_string())?;
    if !extensions.iter().any(|extension| {
        filename
            .to_ascii_lowercase()
            .ends_with(&format!(".{extension}"))
    }) {
        return Err(format!("{filename} is not a supported {kind} file"));
    }
    Ok(())
}

fn studio_modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

fn read_studio_document(path: &Path, kind: &str) -> Result<StudioDocument, String> {
    validate_studio_document_path(path, kind)?;
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    if metadata.len() > MAX_STUDIO_DOCUMENT_BYTES {
        return Err("Studio notebooks and workspaces are limited to 64 MB".into());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let contents =
        String::from_utf8(bytes).map_err(|_| format!("{} is not valid UTF-8", path.display()))?;
    Ok(StudioDocument {
        path: path.display().to_string(),
        filename: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("document")
            .to_string(),
        size: metadata.len(),
        sha256: sha256_file(path)?,
        modified_ms: studio_modified_ms(&metadata),
        contents,
    })
}

fn atomic_write_studio_document(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Studio document has no parent directory".to_string())?;
    if !parent.is_dir() {
        return Err(format!("{} does not exist", parent.display()));
    }
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let temporary = parent.join(format!(
        ".{filename}.part-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Cannot create temporary document: {error}"))?;
        output
            .write_all(contents.as_bytes())
            .and_then(|_| output.sync_all())
            .map_err(|error| format!("Cannot write temporary document: {error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("Cannot atomically replace {}: {error}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn save_studio_document_to_path(
    request: StudioDocumentSaveRequest,
    selected: PathBuf,
) -> Result<StudioDocumentSaveResult, String> {
    validate_studio_document_path(&selected, &request.kind)?;
    let current_sha256 = if selected.is_file() {
        Some(sha256_file(&selected)?)
    } else {
        None
    };
    let changed = match request.expected_sha256.as_deref() {
        Some(expected) => match current_sha256.as_deref() {
            Some(current) => !current.eq_ignore_ascii_case(expected),
            None => true,
        },
        None => current_sha256.is_some(),
    };
    if changed && !request.overwrite {
        return Ok(StudioDocumentSaveResult {
            status: "conflict",
            path: selected.display().to_string(),
            document: None,
            current_sha256,
        });
    }
    atomic_write_studio_document(&selected, &request.contents)?;
    Ok(StudioDocumentSaveResult {
        status: "saved",
        path: selected.display().to_string(),
        document: Some(read_studio_document(&selected, &request.kind)?),
        current_sha256: None,
    })
}

#[tauri::command]
async fn studio_open_document(
    kind: String,
    path: Option<String>,
) -> Result<Option<StudioDocument>, String> {
    let (title, extensions) = studio_document_filter(&kind)?;
    let selected = if let Some(path) = path {
        Some(PathBuf::from(path))
    } else {
        rfd::FileDialog::new()
            .add_filter(title, extensions)
            .pick_file()
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    let document =
        tauri::async_runtime::spawn_blocking(move || read_studio_document(&selected, &kind))
            .await
            .map_err(|error| format!("Studio document worker failed: {error}"))??;
    Ok(Some(document))
}

#[tauri::command]
async fn studio_save_document(
    request: StudioDocumentSaveRequest,
) -> Result<Option<StudioDocumentSaveResult>, String> {
    if request.contents.len() as u64 > MAX_STUDIO_DOCUMENT_BYTES {
        return Err("Studio notebooks and workspaces are limited to 64 MB".into());
    }
    let (title, extensions) = studio_document_filter(&request.kind)?;
    let selected = if let Some(path) = request.path.as_deref() {
        Some(PathBuf::from(path))
    } else {
        rfd::FileDialog::new()
            .add_filter(title, extensions)
            .set_file_name(&request.suggested_name)
            .save_file()
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        save_studio_document_to_path(request, selected)
    })
    .await
    .map_err(|error| format!("Studio save worker failed: {error}"))??;
    Ok(Some(result))
}

#[tauri::command]
async fn studio_document_status(
    path: String,
    expected_sha256: String,
) -> Result<StudioDocumentStatus, String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("Studio document paths must be absolute".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if !path.is_file() {
            return Ok(StudioDocumentStatus {
                exists: false,
                changed: true,
                current_sha256: None,
                modified_ms: None,
            });
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
        let current_sha256 = sha256_file(&path)?;
        Ok(StudioDocumentStatus {
            exists: true,
            changed: !current_sha256.eq_ignore_ascii_case(&expected_sha256),
            current_sha256: Some(current_sha256),
            modified_ms: Some(studio_modified_ms(&metadata)),
        })
    })
    .await
    .map_err(|error| format!("Studio document-status worker failed: {error}"))?
}

#[tauri::command]
fn kernel_initialize(
    namespace: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    if !valid_studio_namespace(&namespace) {
        return Err("Studio notebook identity is invalid".into());
    }
    let mut active = state
        .studio_kernel
        .lock()
        .map_err(|_| "Studio kernel state is unavailable")?;
    if let Some(previous) = active.take() {
        if let Ok(mut child) = previous.console.child.lock() {
            let _ = child.kill();
        }
    }
    let base = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Cannot locate Studio cache: {error}"))?
        .join("studio-kernels");
    fs::create_dir_all(&base).map_err(|error| format!("Cannot create Studio cache: {error}"))?;
    let root = base.join(&namespace);
    fs::create_dir_all(&root)
        .map_err(|error| format!("Cannot create private notebook directory: {error}"))?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("Cannot access private notebook directory: {error}"))?;
    let environment_root = state
        .workspace
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let bl = find_binary(&environment_root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let console = spawn_console_with_binary(&root, &environment_root, &bl)?;
    let response = send_console_request(&console, state.id(), "ping", None)?;
    *active = Some(StudioKernelProcess {
        namespace,
        console,
        root,
        environment_root,
        bl,
    });
    Ok(response)
}

#[tauri::command]
async fn kernel_execute(
    namespace: String,
    source: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    if source.trim().is_empty() {
        return Err("Enter BioLang code to run".into());
    }
    let (console, _) = studio_kernel_console(&state, &namespace)?;
    let id = state.id();
    tauri::async_runtime::spawn_blocking(move || {
        send_console_request(&console, id, "evaluate", Some(&source))
    })
    .await
    .map_err(|error| format!("Studio execution worker failed: {error}"))?
}

#[tauri::command]
async fn kernel_reset(namespace: String, state: State<'_, AppState>) -> Result<Value, String> {
    let (console, _) = studio_kernel_console(&state, &namespace)?;
    let id = state.id();
    tauri::async_runtime::spawn_blocking(move || send_console_request(&console, id, "reset", None))
        .await
        .map_err(|error| format!("Studio reset worker failed: {error}"))?
}

#[tauri::command]
async fn kernel_variables(namespace: String, state: State<'_, AppState>) -> Result<Value, String> {
    let (console, _) = studio_kernel_console(&state, &namespace)?;
    let id = state.id();
    tauri::async_runtime::spawn_blocking(move || {
        send_console_request(&console, id, "inspect", None)
    })
    .await
    .map_err(|error| format!("Studio inspection worker failed: {error}"))?
}

#[tauri::command]
fn kernel_attach(
    namespace: String,
    file: StudioAttachedFile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (_, root) = studio_kernel_console(&state, &namespace)?;
    let destination = studio_kernel_target(&root, &file.path)?;
    let bytes = file.contents.as_bytes();
    let actual = format!("{:x}", Sha256::digest(bytes));
    if file
        .sha256
        .as_deref()
        .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual))
    {
        return Err(format!("{} failed its SHA-256 check", file.path));
    }
    if destination.exists() && sha256_file(&destination)? != actual {
        return Err(format!("{} already contains different data", file.path));
    }
    if !destination.exists() {
        let filename = destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("data");
        let temporary = destination.with_file_name(format!(
            ".{filename}.part-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::write(&temporary, bytes)
            .map_err(|error| format!("Cannot attach {}: {error}", file.path))?;
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot finish attaching {}: {error}", file.path));
        }
    }
    Ok(())
}

#[tauri::command]
fn kernel_clear_files(namespace: String, state: State<'_, AppState>) -> Result<(), String> {
    let (_, root) = studio_kernel_console(&state, &namespace)?;
    for entry in fs::read_dir(&root)
        .map_err(|error| format!("Cannot inspect private notebook data: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("Cannot inspect private notebook data: {error}"))?
            .path();
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Cannot verify private notebook data: {error}"))?;
        if !canonical.starts_with(&root) {
            return Err("Refusing to clear a path outside the private notebook directory".into());
        }
        if canonical.is_dir() {
            fs::remove_dir_all(&canonical)
                .map_err(|error| format!("Cannot clear native notebook data: {error}"))?;
        } else {
            fs::remove_file(&canonical)
                .map_err(|error| format!("Cannot clear native notebook data: {error}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn kernel_import_files(
    namespace: String,
    state: State<'_, AppState>,
) -> Result<Vec<StudioNativeFile>, String> {
    let (_, root) = studio_kernel_console(&state, &namespace)?;
    let selected = rfd::FileDialog::new().pick_files().unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        let mut imported = Vec::new();
        for source in selected {
            if !source.is_file() {
                continue;
            }
            let name = source
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "Selected file name is not valid UTF-8".to_string())?;
            let destination = studio_kernel_target(&root, name)?;
            atomic_copy_into(&source, &destination)?;
            imported.push(studio_file(&destination, name, None)?);
        }
        Ok(imported)
    })
    .await
    .map_err(|error| format!("Studio file-import worker failed: {error}"))?
}

fn ensure_studio_attachment(root: PathBuf, path: String, sha256: String) -> Result<bool, String> {
    let target = studio_kernel_target(&root, &path)?;
    if target.is_file() {
        return Ok(sha256_file(&target)?.eq_ignore_ascii_case(&sha256));
    }

    // A workspace-scoped attachment may already exist in another notebook's
    // private directory. The UI only asks about attachments in scope for this
    // notebook, so reuse the exact checksum-pinned file without exposing any
    // unrelated path. Hard links avoid duplicating large data where supported.
    let base = root
        .parent()
        .ok_or_else(|| "Studio kernel directory has no shared cache root".to_string())?
        .canonicalize()
        .map_err(|error| format!("Cannot inspect Studio kernel cache: {error}"))?;
    for entry in fs::read_dir(&base)
        .map_err(|error| format!("Cannot inspect Studio kernel cache: {error}"))?
    {
        let directory = entry
            .map_err(|error| format!("Cannot inspect Studio kernel cache: {error}"))?
            .path();
        if directory == root || !directory.is_dir() {
            continue;
        }
        let canonical_directory = match directory.canonicalize() {
            Ok(directory) if directory.starts_with(&base) => directory,
            _ => continue,
        };
        let candidate = canonical_directory.join(&path);
        if !candidate.is_file() {
            continue;
        }
        let canonical_candidate = match candidate.canonicalize() {
            Ok(candidate) if candidate.starts_with(&canonical_directory) => candidate,
            _ => continue,
        };
        if !sha256_file(&canonical_candidate)?.eq_ignore_ascii_case(&sha256) {
            continue;
        }
        if fs::hard_link(&canonical_candidate, &target).is_err() {
            atomic_copy_into(&canonical_candidate, &target)?;
        }
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
async fn kernel_has_attachment(
    namespace: String,
    path: String,
    sha256: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let (_, root) = studio_kernel_console(&state, &namespace)?;
    tauri::async_runtime::spawn_blocking(move || ensure_studio_attachment(root, path, sha256))
        .await
        .map_err(|error| format!("Studio attachment worker failed: {error}"))?
}

fn download_studio_url(
    request: StudioRemoteRequest,
    root: PathBuf,
) -> Result<StudioRemoteResult, String> {
    let parsed = reqwest::Url::parse(request.url.trim())
        .map_err(|error| format!("Enter a valid data URL: {error}"))?;
    if parsed.scheme() != "https" {
        return Err("Native Studio data URLs must use HTTPS".into());
    }
    let destination = studio_kernel_target(&root, &request.path)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60 * 60))
        .user_agent("BioLang-Studio/0.1")
        .build()
        .map_err(|error| format!("Cannot initialize native downloader: {error}"))?;
    let mut response = client
        .get(parsed)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Cannot download {}: {error}", request.url))?;
    if response.url().scheme() != "https" {
        return Err("The source redirected to a non-HTTPS location".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_STUDIO_NATIVE_DATA_BYTES)
    {
        return Err("The source exceeds Studio Desktop's 20 GB safety limit".into());
    }
    let filename = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let temporary = destination.with_file_name(format!(
        ".{filename}.part-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Cannot create native download: {error}"))?;
        let mut digest = Sha256::new();
        let mut received = 0_u64;
        let mut buffer = [0_u8; 256 * 1024];
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| format!("Cannot read native download: {error}"))?;
            if read == 0 {
                break;
            }
            received = received.saturating_add(read as u64);
            if received > MAX_STUDIO_NATIVE_DATA_BYTES {
                return Err("Download exceeded Studio Desktop's 20 GB safety limit".into());
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| format!("Cannot write native download: {error}"))?;
            digest.update(&buffer[..read]);
        }
        output.sync_all().map_err(|error| error.to_string())?;
        if request
            .expected_bytes
            .is_some_and(|expected| expected != received)
        {
            return Err(format!(
                "Expected {} bytes, but the source returned {received}",
                request.expected_bytes.unwrap_or_default()
            ));
        }
        let actual = format!("{:x}", digest.finalize());
        if request
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual))
        {
            return Err("Native download failed its expected SHA-256 check".into());
        }
        if destination.exists() {
            if sha256_file(&destination)? == actual {
                fs::remove_file(&temporary).map_err(|error| error.to_string())?;
            } else {
                return Err(format!("{} already contains different data", request.path));
            }
        } else {
            fs::rename(&temporary, &destination)
                .map_err(|error| format!("Cannot finish native download: {error}"))?;
        }
        Ok(StudioRemoteResult {
            path: relative_display(Path::new(&request.path)),
            size: received,
            sha256: actual.clone(),
            media_type: request.media_type,
            source_bytes: received,
            source_sha256: actual,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[tauri::command]
async fn kernel_fetch_url(
    namespace: String,
    request: StudioRemoteRequest,
    state: State<'_, AppState>,
) -> Result<StudioRemoteResult, String> {
    let (_, root) = studio_kernel_console(&state, &namespace)?;
    tauri::async_runtime::spawn_blocking(move || download_studio_url(request, root))
        .await
        .map_err(|error| format!("Studio download worker failed: {error}"))?
}

#[tauri::command]
async fn kernel_export_variable(
    namespace: String,
    name: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<Option<StudioExportResult>, String> {
    let (console, root) = studio_kernel_console(&state, &namespace)?;
    let extension = match format.as_str() {
        "json" | "csv" | "tsv" => format.as_str(),
        "text" => "txt",
        _ => return Err(format!("Unsupported variable export format '{format}'")),
    };
    let safe_name = Path::new(&name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("value");
    let selected = rfd::FileDialog::new()
        .set_directory(&root)
        .set_file_name(format!("{safe_name}.{extension}"))
        .save_file();
    let Some(destination) = selected else {
        return Ok(None);
    };
    let id = state.id();
    let export_destination = destination.clone();
    let response = tauri::async_runtime::spawn_blocking(move || {
        send_console_payload(
            &console,
            id,
            serde_json::json!({
                "id": id,
                "command": "export",
                "name": name,
                "path": export_destination.display().to_string(),
                "format": format,
            }),
        )
    })
    .await
    .map_err(|error| format!("Studio export worker failed: {error}"))??;
    if response.get("status").and_then(Value::as_str) != Some("ok") {
        return Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("BioLang variable export failed")
            .to_string());
    }
    let bytes = fs::metadata(&destination)
        .map_err(|error| format!("Cannot inspect exported file: {error}"))?
        .len();
    Ok(Some(StudioExportResult {
        path: destination.display().to_string(),
        bytes,
    }))
}

#[tauri::command]
async fn kernel_publish_variable(
    namespace: String,
    name: String,
    format: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<StudioNativeFile, String> {
    let (console, root) = studio_kernel_console(&state, &namespace)?;
    let media_type = match format.as_str() {
        "json" => "application/json",
        "csv" => "text/csv",
        "tsv" => "text/tab-separated-values",
        "text" => "text/plain",
        _ => return Err(format!("Unsupported variable export format '{format}'")),
    };
    let destination = studio_kernel_target(&root, &path)?;
    let filename = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let temporary = destination.with_file_name(format!(
        ".{filename}.publish-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let export_destination = temporary.clone();
    let id = state.id();
    let response = tauri::async_runtime::spawn_blocking(move || {
        send_console_payload(
            &console,
            id,
            serde_json::json!({
                "id": id,
                "command": "export",
                "name": name,
                "path": export_destination.display().to_string(),
                "format": format,
            }),
        )
    })
    .await
    .map_err(|error| format!("Studio output worker failed: {error}"))??;
    if response.get("status").and_then(Value::as_str) != Some("ok") {
        let _ = fs::remove_file(&temporary);
        return Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("BioLang output publication failed")
            .to_string());
    }

    let result = (|| {
        let actual = sha256_file(&temporary)?;
        if destination.exists() {
            if sha256_file(&destination)?.eq_ignore_ascii_case(&actual) {
                fs::remove_file(&temporary).map_err(|error| error.to_string())?;
            } else {
                return Err(format!(
                    "{} already contains a different published output",
                    relative_display(Path::new(&path))
                ));
            }
        } else {
            fs::rename(&temporary, &destination)
                .map_err(|error| format!("Cannot publish {}: {error}", relative_display(Path::new(&path))))?;
        }
        studio_file(&destination, &path, Some(media_type))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[tauri::command]
fn kernel_cancel(namespace: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state
        .studio_kernel
        .lock()
        .map_err(|_| "Studio kernel state is unavailable")?;
    if active
        .as_ref()
        .is_some_and(|process| process.namespace != namespace)
    {
        return Err("This notebook's native kernel is no longer active".into());
    }
    if let Some(process) = active.take() {
        process
            .console
            .child
            .lock()
            .map_err(|_| "Studio kernel process is unavailable")?
            .kill()
            .map_err(|error| format!("Cannot stop Studio kernel: {error}"))?;
        let console =
            spawn_console_with_binary(&process.root, &process.environment_root, &process.bl)?;
        *active = Some(StudioKernelProcess { console, ..process });
    }
    Ok(())
}

#[tauri::command]
fn kernel_dispose(namespace: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state
        .studio_kernel
        .lock()
        .map_err(|_| "Studio kernel state is unavailable")?;
    if active
        .as_ref()
        .is_some_and(|process| process.namespace != namespace)
    {
        return Ok(());
    }
    if let Some(process) = active.take() {
        process
            .console
            .child
            .lock()
            .map_err(|_| "Studio kernel process is unavailable")?
            .kill()
            .map_err(|error| format!("Cannot close Studio kernel: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
fn start_terminal(
    cols: u16,
    rows: u16,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    require_trusted_workspace(&state)?;
    let root = workspace_root(&state)?;
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(10),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Cannot open terminal: {error}"))?;

    let shell = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    };
    let mut command = CommandBuilder::new(shell);
    command.cwd(root);
    if cfg!(windows) && std::env::var("COMSPEC").is_err() {
        command.arg("-NoLogo");
    }
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Cannot start shell: {error}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Cannot read terminal: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Cannot write terminal: {error}"))?;

    let session_id = state.id();
    let session = Arc::new(TerminalSession {
        writer: Mutex::new(writer),
        master: Mutex::new(pair.master),
        child: Mutex::new(child),
    });
    state
        .terminals
        .lock()
        .map_err(|_| "Terminal state is unavailable")?
        .insert(session_id, session);

    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    let _ = app.emit(
                        "terminal-output",
                        TerminalOutput {
                            session_id,
                            data: String::from_utf8_lossy(&buffer[..read]).to_string(),
                        },
                    );
                }
            }
        }
    });
    Ok(session_id)
}

#[tauri::command]
fn terminal_write(session_id: u64, data: String, state: State<'_, AppState>) -> Result<(), String> {
    require_trusted_workspace(&state)?;
    let session = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state is unavailable")?
        .get(&session_id)
        .cloned()
        .ok_or_else(|| "Terminal session is closed".to_string())?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "Terminal writer is unavailable")?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Cannot write to terminal: {error}"))
}

#[tauri::command]
fn terminal_resize(
    session_id: u64,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    require_trusted_workspace(&state)?;
    let session = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state is unavailable")?
        .get(&session_id)
        .cloned()
        .ok_or_else(|| "Terminal session is closed".to_string())?;
    let result = session
        .master
        .lock()
        .map_err(|_| "Terminal is unavailable")?
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(10),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Cannot resize terminal: {error}"));
    result
}

#[tauri::command]
fn close_terminal(session_id: u64, state: State<'_, AppState>) -> Result<(), String> {
    let session = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state is unavailable")?
        .remove(&session_id)
        .ok_or_else(|| "Terminal session is already closed".to_string())?;
    let result = session
        .child
        .lock()
        .map_err(|_| "Terminal process is unavailable")?
        .kill()
        .map_err(|error| format!("Cannot close terminal: {error}"));
    result
}

fn read_lsp_messages(reader: impl Read + Send + 'static, app: AppHandle) {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut content_length = None;
            loop {
                let mut header = String::new();
                match reader.read_line(&mut header) {
                    Ok(0) | Err(_) => return,
                    Ok(_) if header == "\r\n" || header == "\n" => break,
                    Ok(_) => {
                        if let Some(value) = header.strip_prefix("Content-Length:") {
                            content_length = value.trim().parse::<usize>().ok();
                        }
                    }
                }
            }
            let Some(content_length) = content_length else {
                continue;
            };
            let mut body = vec![0_u8; content_length];
            if reader.read_exact(&mut body).is_err() {
                return;
            }
            if let Ok(message) = serde_json::from_slice::<Value>(&body) {
                let _ = app.emit("lsp-message", message);
            }
        }
    });
}

#[tauri::command]
fn start_lsp(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    require_trusted_workspace(&state)?;
    let mut lsp = state.lsp.lock().map_err(|_| "LSP state is unavailable")?;
    if lsp.is_some() {
        return Ok(true);
    }
    let root = workspace_root(&state)?;
    let direct = find_binary(&root, "bl-lsp");
    let (executable, arguments) = if let Some(path) = direct {
        (path, Vec::<String>::new())
    } else {
        let bl = find_binary(&root, "bl")
            .ok_or_else(|| "BioLang LSP executable not found".to_string())?;
        (bl, vec!["lsp".into()])
    };
    let mut command = Command::new(&executable);
    configure_biolang_command(&mut command, &root, &executable);
    let mut child = command
        .args(arguments)
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot start BioLang LSP: {error}"))?;
    let stdin = Arc::new(Mutex::new(
        child.stdin.take().ok_or("LSP stdin is unavailable")?,
    ));
    let stdout = child.stdout.take().ok_or("LSP stdout is unavailable")?;
    let child = Arc::new(Mutex::new(child));
    read_lsp_messages(stdout, app);
    *lsp = Some(LspProcess { stdin, child });
    Ok(true)
}

#[tauri::command]
fn send_lsp(message: Value, state: State<'_, AppState>) -> Result<(), String> {
    let stdin = state
        .lsp
        .lock()
        .map_err(|_| "LSP state is unavailable")?
        .as_ref()
        .map(|process| process.stdin.clone())
        .ok_or_else(|| "BioLang LSP is not running".to_string())?;
    let body = serde_json::to_vec(&message).map_err(|error| error.to_string())?;
    let mut stdin = stdin.lock().map_err(|_| "LSP stdin is unavailable")?;
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len())
        .and_then(|_| stdin.write_all(&body))
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("Cannot send LSP message: {error}"))
}

fn shutdown_processes(state: &AppState) {
    if let Ok(mut jobs) = state.jobs.lock() {
        for process in jobs.values() {
            if let Ok(mut child) = process.lock() {
                let _ = child.kill();
            }
        }
        jobs.clear();
    }
    if let Ok(mut terminals) = state.terminals.lock() {
        for session in terminals.values() {
            if let Ok(mut child) = session.child.lock() {
                let _ = child.kill();
            }
        }
        terminals.clear();
    }
    if let Ok(mut console) = state.console.lock() {
        if let Some(process) = console.take() {
            if let Ok(mut child) = process.child.lock() {
                let _ = child.kill();
            }
        }
    }
    if let Ok(mut studio) = state.studio_kernel.lock() {
        if let Some(process) = studio.take() {
            if let Ok(mut child) = process.console.child.lock() {
                let _ = child.kill();
            }
        }
    }
    if let Ok(mut lsp) = state.lsp.lock() {
        if let Some(process) = lsp.take() {
            if let Ok(mut child) = process.child.lock() {
                let _ = child.kill();
            }
        }
    }
    if let Ok(mut tunnels) = state.somer_tunnels.lock() {
        for tunnel in tunnels.values() {
            if let Ok(mut child) = tunnel.child.lock() {
                let _ = child.kill();
            }
        }
        tunnels.clear();
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            select_workspace,
            pick_path,
            open_workspace,
            close_workspace,
            set_workspace_trust,
            workspace_snapshot,
            git_status,
            get_somer_secret,
            list_credentials,
            compare_run_environment,
            restore_run_environment,
            list_reference_builds,
            save_reference_build,
            delete_reference_build,
            run_workspace_tests,
            git_stage,
            git_unstage,
            git_commit,
            git_diff,
            set_credential,
            delete_credential,
            set_somer_secret,
            delete_somer_secret,
            start_somer_tunnel,
            stop_somer_tunnel,
            create_entry,
            rename_entry,
            move_entry,
            write_new_file,
            delete_entry,
            duplicate_entry,
            reveal_entry,
            open_external,
            search_workspace,
            replace_in_workspace,
            preview_file,
            import_files,
            checksum_workspace_files,
            import_code,
            import_code_url,
            validate_import_code,
            export_preview,
            export_text,
            export_binary,
            load_run_history,
            save_run_history,
            delete_run_history,
            read_file,
            read_workspace_binary,
            read_workspace_binary_range,
            read_jsonl_page,
            write_file,
            save_file_as,
            write_clipboard,
            get_environment,
            run_file,
            run_source,
            run_notebook,
            run_notebook_source,
            run_workflow,
            stop_job,
            list_packages,
            install_packages,
            start_console,
            evaluate_console,
            inspect_console,
            reset_console,
            stop_console,
            close_console,
            studio_open_document,
            studio_save_document,
            studio_document_status,
            kernel_initialize,
            kernel_execute,
            kernel_reset,
            kernel_variables,
            kernel_attach,
            kernel_clear_files,
            kernel_import_files,
            kernel_has_attachment,
            kernel_fetch_url,
            kernel_export_variable,
            kernel_publish_variable,
            kernel_cancel,
            kernel_dispose,
            start_terminal,
            terminal_write,
            terminal_resize,
            close_terminal,
            start_lsp,
            send_lsp,
        ])
        .build(tauri::generate_context!())
        .expect("error while building BioLang Desktop")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                shutdown_processes(app.state::<AppState>().inner());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_copy_into, ensure_studio_attachment, preview_delimited, preview_fasta,
        save_studio_document_to_path, sha256_file, studio_kernel_target, valid_studio_namespace,
        workflow_source,
        StudioDocumentSaveRequest, WorkflowDocument, WorkflowEdge, WorkflowNode,
    };

    fn studio_test_root(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "biolang-studio-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    #[test]
    fn studio_namespaces_and_paths_cannot_escape_the_private_root() {
        assert!(valid_studio_namespace("notebook-123_ab"));
        assert!(!valid_studio_namespace("../notebook"));
        assert!(!valid_studio_namespace("notebook/child"));

        let root = std::env::temp_dir().join(format!(
            "biolang-studio-path-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        assert_eq!(
            studio_kernel_target(&root, "data/counts.csv").unwrap(),
            root.join("data/counts.csv")
        );
        assert!(studio_kernel_target(&root, "../outside.csv").is_err());
        let absolute = root.parent().unwrap().join("outside.csv");
        assert!(studio_kernel_target(&root, absolute.to_str().unwrap()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn studio_document_save_detects_external_changes_before_atomic_overwrite() {
        let root = studio_test_root("document-save");
        let destination = root.join("analysis.bln");
        std::fs::write(&destination, "let value = 1\n").unwrap();
        let expected = sha256_file(&destination).unwrap();
        std::fs::write(&destination, "let value = 2\n").unwrap();

        let request = |overwrite| StudioDocumentSaveRequest {
            kind: "notebook".into(),
            path: Some(destination.display().to_string()),
            suggested_name: "analysis.bln".into(),
            contents: "let value = 3\n".into(),
            expected_sha256: Some(expected.clone()),
            overwrite,
        };
        let conflict = save_studio_document_to_path(request(false), destination.clone()).unwrap();
        assert_eq!(conflict.status, "conflict");
        assert_eq!(
            std::fs::read_to_string(&destination).unwrap(),
            "let value = 2\n"
        );

        let saved = save_studio_document_to_path(request(true), destination.clone()).unwrap();
        assert_eq!(saved.status, "saved");
        assert_eq!(
            std::fs::read_to_string(&destination).unwrap(),
            "let value = 3\n"
        );
        assert!(std::fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".part-")
        }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verified_native_attachment_is_reused_offline_across_notebooks() {
        let base = studio_test_root("shared-data");
        let first = base.join("notebook-one");
        let second = base.join("notebook-two");
        std::fs::create_dir_all(first.join("data")).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let source = first.join("data/counts.tsv");
        std::fs::write(&source, "gene\tcount\nTP53\t7\n").unwrap();
        let digest = sha256_file(&source).unwrap();

        assert!(!ensure_studio_attachment(
            second.clone(),
            "data/counts.tsv".into(),
            "0".repeat(64)
        )
        .unwrap());
        assert!(
            ensure_studio_attachment(second.clone(), "data/counts.tsv".into(), digest).unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(second.join("data/counts.tsv")).unwrap(),
            "gene\tcount\nTP53\t7\n"
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    #[ignore = "1 GiB native I/O stress test"]
    fn studio_one_gib_copy_and_hash_remains_streamed() {
        let root = studio_test_root("one-gib-streaming");
        let source = root.join("source.bin");
        let destination = root.join("destination.bin");
        let result = (|| -> Result<(), String> {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&source)
                .map_err(|error| error.to_string())?;
            file.set_len(1024 * 1024 * 1024)
                .map_err(|error| error.to_string())?;

            let started = std::time::Instant::now();
            atomic_copy_into(&source, &destination)?;
            let source_digest = sha256_file(&source)?;
            let destination_digest = sha256_file(&destination)?;
            if source_digest != destination_digest {
                return Err("streamed copy digest does not match its source".into());
            }
            if std::fs::metadata(&destination)
                .map_err(|error| error.to_string())?
                .len()
                != 1024 * 1024 * 1024
            {
                return Err("streamed copy has the wrong length".into());
            }
            eprintln!("one_gib_elapsed_ms={}", started.elapsed().as_millis());
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(&root);
        result.unwrap();
    }

    #[test]
    fn fasta_preview_lists_records_and_keeps_first_sequence() {
        let input = ">ori_candidate\nTAAACGTGAG\nAGAAACGTGC\n>control\nCCAGATC\n";
        let (rows, sequence, sequences) = preview_fasta(input);

        assert_eq!(
            rows,
            vec![
                vec!["ori_candidate".to_string(), "20".to_string()],
                vec!["control".to_string(), "7".to_string()],
            ]
        );
        assert_eq!(sequence.as_deref(), Some("TAAACGTGAGAGAAACGTGC"));
        assert_eq!(sequences[1].name, "control");
        assert_eq!(sequences[1].sequence, "CCAGATC");
    }

    #[test]
    fn csv_preview_handles_quoted_delimiters() {
        let input = "gene,note,value\nBRCA1,\"DNA repair, breast cancer\",12.4\n";
        let (columns, rows) = preview_delimited(input, b',').expect("CSV should parse");

        assert_eq!(columns, vec!["gene", "note", "value"]);
        assert_eq!(rows[0][1], "DNA repair, breast cancer");
        assert_eq!(rows[0][2], "12.4");
    }

    #[test]
    fn workflow_generation_connects_nodes_with_pipes() {
        let workflow = WorkflowDocument {
            schema_version: 1,
            name: "QC".into(),
            nodes: vec![
                WorkflowNode {
                    id: "input".into(),
                    operation: "read_fasta".into(),
                    arguments: vec!["\"reads.fa\"".into()],
                    parameters: Vec::new(),
                    strategy: None,
                },
                WorkflowNode {
                    id: "first ten".into(),
                    operation: "take".into(),
                    arguments: vec!["10".into()],
                    parameters: Vec::new(),
                    strategy: None,
                },
            ],
            edges: vec![WorkflowEdge {
                from: "input".into(),
                to: "first ten".into(),
            }],
        };
        let source = workflow_source(&workflow).expect("workflow should generate");
        assert!(source.contains("let input = read_fasta(\"reads.fa\")"));
        assert!(source.contains("let first_ten = input |> take(10)"));
        assert!(source.contains("println(first_ten)"));
    }

    #[test]
    fn workflow_generation_sorts_and_merges_dag_inputs() {
        let node = |id: &str, operation: &str, strategy: Option<&str>| WorkflowNode {
            id: id.into(),
            operation: operation.into(),
            arguments: Vec::new(),
            parameters: Vec::new(),
            strategy: strategy.map(str::to_string),
        };
        let workflow = WorkflowDocument {
            schema_version: 1,
            name: "DAG".into(),
            nodes: vec![
                node("merge", "combine", Some("gather")),
                node("left", "read_fasta", None),
                node("right", "read_fasta", None),
            ],
            edges: vec![
                WorkflowEdge {
                    from: "left".into(),
                    to: "merge".into(),
                },
                WorkflowEdge {
                    from: "right".into(),
                    to: "merge".into(),
                },
            ],
        };

        let source = workflow_source(&workflow).expect("DAG should generate");
        let left = source.find("let left").expect("left source");
        let right = source.find("let right").expect("right source");
        let merge = source.find("let merge").expect("merge node");
        assert!(left < merge && right < merge);
        assert!(source.contains("let merge = [left, right] |> combine()"));
    }

    #[test]
    fn workflow_generation_rejects_cycles_and_identifier_collisions() {
        let node = |id: &str| WorkflowNode {
            id: id.into(),
            operation: "identity".into(),
            arguments: Vec::new(),
            parameters: Vec::new(),
            strategy: None,
        };
        let cycle = WorkflowDocument {
            schema_version: 1,
            name: "Cycle".into(),
            nodes: vec![node("a"), node("b")],
            edges: vec![
                WorkflowEdge {
                    from: "a".into(),
                    to: "b".into(),
                },
                WorkflowEdge {
                    from: "b".into(),
                    to: "a".into(),
                },
            ],
        };
        assert!(workflow_source(&cycle)
            .expect_err("cycle should fail")
            .contains("cycle"));

        let collision = WorkflowDocument {
            schema_version: 1,
            name: "Collision".into(),
            nodes: vec![node("a-b"), node("a b")],
            edges: Vec::new(),
        };
        assert!(workflow_source(&collision)
            .expect_err("identifier collision should fail")
            .contains("same BioLang variable"));
    }
}
