use base64::Engine;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
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

    for ancestor in root.ancestors().take(6) {
        for profile in ["debug", "release"] {
            let candidate = ancestor.join("target").join(profile).join(&executable);
            if candidate.is_file() {
                return Some(candidate);
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

fn search_directory(root: &Path, directory: &Path, query: &str, hits: &mut Vec<SearchHit>) {
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
            search_directory(root, &path, query, hits);
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
            let lowercase = line.to_lowercase();
            if let Some(column) = lowercase.find(query) {
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
fn search_workspace(query: String, state: State<'_, AppState>) -> Result<Vec<SearchHit>, String> {
    let root = workspace_root(&state)?;
    let query = query.trim().to_lowercase();
    if query.len() < 2 {
        return Ok(Vec::new());
    }
    let mut hits = Vec::new();
    search_directory(&root, &root, &query, &mut hits);
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
    find_binary(&root, "bl")
        .ok_or_else(|| "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string())
}

/// Parse a `bl import --json` invocation's captured output into an `ImportResult`.
fn parse_import_output(status: std::process::ExitStatus, stdout: &[u8], stderr: &[u8]) -> Result<ImportResult, String> {
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
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
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
    });
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
    let child = Command::new(bl)
        .arg(command)
        .arg(&script)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            if let Some(path) = cleanup_path {
                let _ = fs::remove_file(path);
            }
            return Err(format!("Cannot start BioLang: {error}"));
        }
    };

    let job_id = state.id();
    if let Some(stdout) = child.stdout.take() {
        stream_reader(stdout, app.clone(), job_id, "stdout");
    }
    if let Some(stderr) = child.stderr.take() {
        stream_reader(stderr, app.clone(), job_id, "stderr");
    }

    let child = Arc::new(Mutex::new(child));
    state
        .jobs
        .lock()
        .map_err(|_| "Job state is unavailable")?
        .insert(job_id, child.clone());

    let handle = app.clone();
    thread::spawn(move || {
        let started = Instant::now();
        loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut child| child.try_wait().ok())
                .flatten();
            if let Some(status) = status {
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

#[tauri::command]
fn run_file(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    start_biolang_job(path, "run", &[".bl"], app, state)
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
    let output = Command::new(bl)
        .arg("install")
        .current_dir(root)
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

fn spawn_console(root: &Path) -> Result<Arc<ConsoleProcess>, String> {
    let bl = find_binary(root, "bl").ok_or_else(|| {
        "BioLang executable not found. Build BioLang or set BIOLANG_BIN.".to_string()
    })?;
    let mut child = Command::new(bl)
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
    let _request = process
        .io
        .lock()
        .map_err(|_| "Console request state is unavailable")?;
    let request = serde_json::json!({
        "id": id,
        "command": command,
        "source": source,
    });
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
    let mut child = Command::new(executable)
        .args(arguments)
        .current_dir(root)
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
            open_workspace,
            close_workspace,
            set_workspace_trust,
            workspace_snapshot,
            git_status,
            get_somer_secret,
            set_somer_secret,
            delete_somer_secret,
            start_somer_tunnel,
            stop_somer_tunnel,
            create_entry,
            rename_entry,
            delete_entry,
            duplicate_entry,
            reveal_entry,
            open_external,
            search_workspace,
            preview_file,
            import_files,
            import_code,
            import_code_url,
            validate_import_code,
            export_preview,
            export_text,
            read_file,
            write_file,
            save_file_as,
            get_environment,
            run_file,
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
        preview_delimited, preview_fasta, workflow_source, WorkflowDocument, WorkflowEdge,
        WorkflowNode,
    };

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
