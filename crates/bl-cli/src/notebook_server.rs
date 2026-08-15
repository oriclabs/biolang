//! Loopback-only native kernel for the BioLang notebook UI.
//!
//! The server deliberately has two protocol layers:
//! - a small SOMER-compatible `/v1/jobs` subset for stateless clients; and
//! - `/v1/notebook-sessions`, whose interpreter survives between cell runs.
//!
//! This is not a scheduler or a multi-user service. It binds only to loopback,
//! uses a launch-scoped bearer token, exposes one notebook file, and exits with
//! the `bl` process.

use crate::events;
use bl_core::value::Value;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVENTS_PER_SESSION: usize = 10_000;

#[derive(Debug)]
struct NotebookDocument {
    source: String,
    revision: u64,
}

#[derive(Debug)]
struct ExecutionState {
    id: String,
    cell_id: Option<String>,
    status: String,
    created_at: u128,
    started_at: Option<u128>,
    duration_ms: Option<u128>,
    stdout: String,
    stderr: String,
    results: Vec<JsonValue>,
    events: Vec<JsonValue>,
}

impl ExecutionState {
    fn new(id: String, cell_id: Option<String>) -> Self {
        Self {
            id,
            cell_id,
            status: "queued".into(),
            created_at: now_ms(),
            started_at: None,
            duration_ms: None,
            stdout: String::new(),
            stderr: String::new(),
            results: Vec::new(),
            events: Vec::new(),
        }
    }

    fn snapshot(&self) -> JsonValue {
        json!({
            "id": self.id,
            "cellId": self.cell_id,
            "status": self.status,
            "createdAt": self.created_at,
            "startedAt": self.started_at,
            "durationMs": self.duration_ms,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "results": self.results,
            "events": self.events,
        })
    }

    fn terminal(&self) -> bool {
        matches!(self.status.as_str(), "succeeded" | "failed" | "cancelled")
    }
}

#[derive(Debug, Default)]
struct SessionEvents {
    next_sequence: AtomicU64,
    entries: Mutex<Vec<JsonValue>>,
}

enum SessionCommand {
    Execute {
        source: String,
        execution: Arc<Mutex<ExecutionState>>,
    },
    Shutdown,
}

struct NotebookSession {
    id: String,
    created_at: u128,
    sender: mpsc::Sender<SessionCommand>,
    executions: Mutex<HashMap<String, Arc<Mutex<ExecutionState>>>>,
    events: Arc<SessionEvents>,
}

impl NotebookSession {
    fn start(id: String, notebook_path: PathBuf) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel();
        let events = Arc::new(SessionEvents::default());
        let session = Arc::new(Self {
            id,
            created_at: now_ms(),
            sender,
            executions: Mutex::new(HashMap::new()),
            events: events.clone(),
        });
        std::thread::Builder::new()
            .name(format!(
                "bl-notebook-{}",
                &session.id[..session.id.len().min(20)]
            ))
            .stack_size(256 * 1024 * 1024)
            .spawn(move || session_worker(receiver, events, notebook_path))
            .expect("cannot start notebook interpreter thread");
        session
    }

    fn enqueue(&self, source: String, cell_id: Option<String>) -> Result<String, String> {
        let id = random_id("exec");
        let execution = Arc::new(Mutex::new(ExecutionState::new(id.clone(), cell_id)));
        emit_execution_event(&execution, &self.events, json!({ "type": "queued" }));
        self.executions
            .lock()
            .map_err(|_| "session execution store is unavailable".to_string())?
            .insert(id.clone(), execution.clone());
        self.sender
            .send(SessionCommand::Execute { source, execution })
            .map_err(|_| "notebook session has stopped".to_string())?;
        Ok(id)
    }

    fn execution(&self, id: &str) -> Option<Arc<Mutex<ExecutionState>>> {
        self.executions.lock().ok()?.get(id).cloned()
    }

    fn snapshot(&self) -> JsonValue {
        let executions = self
            .executions
            .lock()
            .map(|items| {
                items
                    .values()
                    .filter_map(|item| item.lock().ok().map(|e| e.snapshot()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({
            "id": self.id,
            "createdAt": self.created_at,
            "status": "ready",
            "executions": executions,
        })
    }
}

impl Drop for NotebookSession {
    fn drop(&mut self) {
        let _ = self.sender.send(SessionCommand::Shutdown);
    }
}

#[derive(Clone)]
struct JobRecord {
    id: String,
    name: String,
    entrypoint: String,
    source: String,
    session_id: String,
    execution_id: String,
}

struct ServerState {
    token: String,
    authority: String,
    notebook_path: PathBuf,
    root: PathBuf,
    document: Mutex<NotebookDocument>,
    sessions: Mutex<HashMap<String, Arc<NotebookSession>>>,
    jobs: Mutex<HashMap<String, JobRecord>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionRequest {
    source: String,
    #[serde(default)]
    cell_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveNotebookRequest {
    source: String,
    expected_revision: u64,
}

#[derive(Deserialize)]
struct RenderMarkdownRequest {
    source: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobRequest {
    source: String,
    #[serde(default = "default_job_name")]
    name: String,
    #[serde(default = "default_entrypoint")]
    entrypoint: String,
}

fn default_job_name() -> String {
    "BioLang notebook run".into()
}

fn default_entrypoint() -> String {
    "notebook.generated.bl".into()
}

pub fn serve(path: &str, bind: &str, port: u16, root: Option<&str>, open: bool) {
    let ip = match bind.parse::<IpAddr>() {
        Ok(ip) if ip.is_loopback() => ip,
        _ => {
            eprintln!(
                "Error: notebook serve is local-only; --bind must be a loopback address such as 127.0.0.1"
            );
            std::process::exit(2);
        }
    };
    let notebook_path = match std::fs::canonicalize(path) {
        Ok(path) if path.is_file() => path,
        Ok(_) => {
            eprintln!("Error: notebook path is not a file: {path}");
            std::process::exit(2);
        }
        Err(error) => {
            eprintln!("Error reading notebook '{path}': {error}");
            std::process::exit(2);
        }
    };
    let root = match resolve_root(root, &notebook_path) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(2);
        }
    };
    if !notebook_path.starts_with(&root) {
        eprintln!(
            "Error: notebook '{}' is outside --root '{}'",
            notebook_path.display(),
            root.display()
        );
        std::process::exit(2);
    }
    let source = match std::fs::read_to_string(&notebook_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error reading '{}': {error}", notebook_path.display());
            std::process::exit(2);
        }
    };
    let server = match Server::http(SocketAddr::new(ip, port)) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("Error starting notebook server: {error}");
            std::process::exit(1);
        }
    };
    let authority = server.server_addr().to_string();
    let token = random_id("blnt");
    let state = Arc::new(ServerState {
        token,
        authority: authority.clone(),
        notebook_path,
        root,
        document: Mutex::new(NotebookDocument {
            source,
            revision: 1,
        }),
        sessions: Mutex::new(HashMap::new()),
        jobs: Mutex::new(HashMap::new()),
    });
    let url = format!("http://{authority}/");
    eprintln!("BioLang notebook server: {url}");
    eprintln!("Notebook: {}", state.notebook_path.display());
    eprintln!("Root: {}", state.root.display());
    eprintln!("Compute backend: {}", bl_runtime::gpu::execution_summary());
    eprintln!("Press Ctrl+C to stop.");
    if open {
        if let Err(error) = open_browser(&url) {
            eprintln!("Could not open the browser automatically: {error}");
        }
    }
    for request in server.incoming_requests() {
        let state = state.clone();
        std::thread::spawn(move || handle_request(request, state));
    }
}

fn resolve_root(root: Option<&str>, notebook_path: &Path) -> Result<PathBuf, String> {
    let candidate = root
        .map(PathBuf::from)
        .or_else(|| notebook_path.parent().map(Path::to_path_buf))
        .ok_or_else(|| "cannot determine the notebook directory".to_string())?;
    std::fs::canonicalize(&candidate)
        .map_err(|error| format!("cannot resolve root '{}': {error}", candidate.display()))
}

fn handle_request(request: Request, state: Arc<ServerState>) {
    if !valid_host(&request, &state.authority) {
        return json_response(
            request,
            421,
            json!({ "error": "invalid_host", "message": "Host is not this loopback notebook server" }),
        );
    }
    let url = request.url().to_string();
    let (path, query) = split_url(&url);
    if path == "/" && request.method() == &Method::Get {
        return html_response(request, notebook_page(&state));
    }
    if path == "/favicon.ico" && request.method() == &Method::Get {
        return empty_response(request, 204);
    }
    if path == "/v1/service-info" && request.method() == &Method::Get {
        return json_response(request, 200, service_info());
    }
    if !authorized(&request, &state.token) {
        return json_response(
            request,
            401,
            json!({ "error": "unauthorized", "message": "A valid launch token is required" }),
        );
    }
    if !valid_origin(&request, &state.authority) {
        return json_response(
            request,
            403,
            json!({ "error": "invalid_origin", "message": "Browser origin is not this notebook server" }),
        );
    }
    if path == "/v1/me" && request.method() == &Method::Get {
        return json_response(
            request,
            200,
            json!({ "id": "local", "displayName": "Local BioLang user", "roles": ["owner"] }),
        );
    }
    if path == "/v1/resource-profiles" && request.method() == &Method::Get {
        let cpus = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1);
        let gpu = if bl_runtime::gpu::execution_summary().starts_with("GPU:") {
            1
        } else {
            0
        };
        return json_response(
            request,
            200,
            json!([{
                "id": "local",
                "name": "Local machine",
                "description": "The machine running bl notebook serve",
                "cpu": cpus,
                "memoryGb": 0,
                "gpu": gpu,
            }]),
        );
    }
    if path == "/v1/notebook" {
        return handle_notebook(request, &state);
    }
    if path == "/v1/render-markdown" {
        if request.method() != &Method::Post {
            return method_not_allowed(request);
        }
        let mut request = request;
        let markdown: RenderMarkdownRequest = match read_json(&mut request) {
            Ok(markdown) => markdown,
            Err(error) => return api_error(request, 400, "invalid_request", error),
        };
        return json_response(
            request,
            200,
            json!({ "html": crate::notebook::markdown_to_html(&markdown.source) }),
        );
    }
    if path == "/v1/notebook-sessions" {
        return match *request.method() {
            Method::Post => match create_session(&state) {
                Ok(session) => json_response(request, 201, session.snapshot()),
                Err(error) => api_error(request, 500, "session_failed", error),
            },
            _ => method_not_allowed(request),
        };
    }
    if path.starts_with("/v1/notebook-sessions/") {
        return handle_session_route(request, &state, path, query);
    }
    if path == "/v1/jobs" {
        return handle_jobs_collection(request, &state);
    }
    if path.starts_with("/v1/jobs/") {
        return handle_job_route(request, &state, path);
    }
    api_error(request, 404, "not_found", "No such notebook endpoint")
}

fn service_info() -> JsonValue {
    json!({
        "name": "BioLang local notebook kernel",
        "version": env!("CARGO_PKG_VERSION"),
        "apiVersion": "v1",
        "executionMode": "integrated",
        "compatibility": "somer-jobs-subset",
        "capabilities": [
            "jobs",
            "job-cancellation-when-queued",
            "structured-results",
            "notebook-sessions",
            "persistent-interpreter",
            "execution-events",
            "notebook-save",
            "markdown-preview"
        ],
        "localOnly": true,
    })
}

fn handle_notebook(mut request: Request, state: &Arc<ServerState>) {
    match *request.method() {
        Method::Get => {
            let document = match state.document.lock() {
                Ok(document) => document,
                Err(_) => {
                    return api_error(
                        request,
                        500,
                        "document_unavailable",
                        "Notebook state is unavailable",
                    )
                }
            };
            json_response(
                request,
                200,
                json!({
                    "name": state.notebook_path.file_name().and_then(|name| name.to_str()).unwrap_or("notebook.bln"),
                    "source": document.source,
                    "revision": document.revision,
                    "backend": bl_runtime::gpu::execution_summary(),
                }),
            )
        }
        Method::Put => {
            let update: SaveNotebookRequest = match read_json(&mut request) {
                Ok(update) => update,
                Err(error) => return api_error(request, 400, "invalid_request", error),
            };
            let mut document = match state.document.lock() {
                Ok(document) => document,
                Err(_) => {
                    return api_error(
                        request,
                        500,
                        "document_unavailable",
                        "Notebook state is unavailable",
                    )
                }
            };
            if update.expected_revision != document.revision {
                return json_response(
                    request,
                    409,
                    json!({
                        "error": "revision_conflict",
                        "message": "Notebook changed since it was opened",
                        "revision": document.revision,
                    }),
                );
            }
            match std::fs::read_to_string(&state.notebook_path) {
                Ok(disk_source) if disk_source != document.source => {
                    return json_response(
                        request,
                        409,
                        json!({
                            "error": "external_change",
                            "message": "Notebook changed on disk; reload it before saving",
                            "revision": document.revision,
                        }),
                    );
                }
                Err(error) => {
                    return api_error(request, 500, "save_failed", error.to_string());
                }
                _ => {}
            }
            if let Err(error) = std::fs::write(&state.notebook_path, update.source.as_bytes()) {
                return api_error(request, 500, "save_failed", error.to_string());
            }
            document.source = update.source;
            document.revision += 1;
            json_response(
                request,
                200,
                json!({ "saved": true, "revision": document.revision }),
            )
        }
        _ => method_not_allowed(request),
    }
}

fn create_session(state: &Arc<ServerState>) -> Result<Arc<NotebookSession>, String> {
    let id = random_id("session");
    let session = NotebookSession::start(id.clone(), state.notebook_path.clone());
    state
        .sessions
        .lock()
        .map_err(|_| "session store is unavailable".to_string())?
        .insert(id, session.clone());
    Ok(session)
}

fn get_session(state: &Arc<ServerState>, id: &str) -> Option<Arc<NotebookSession>> {
    state.sessions.lock().ok()?.get(id).cloned()
}

fn handle_session_route(
    mut request: Request,
    state: &Arc<ServerState>,
    path: &str,
    query: Option<&str>,
) {
    let parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    let Some(session_id) = parts.get(2).copied() else {
        return api_error(request, 404, "not_found", "Session was not specified");
    };
    let Some(session) = get_session(state, session_id) else {
        return api_error(
            request,
            404,
            "session_not_found",
            "Notebook session does not exist",
        );
    };
    match parts.as_slice() {
        ["v1", "notebook-sessions", _] => match *request.method() {
            Method::Get => json_response(request, 200, session.snapshot()),
            Method::Delete => {
                if let Ok(mut sessions) = state.sessions.lock() {
                    sessions.remove(session_id);
                }
                json_response(
                    request,
                    200,
                    json!({ "id": session_id, "status": "closed" }),
                )
            }
            _ => method_not_allowed(request),
        },
        ["v1", "notebook-sessions", _, "executions"] => match *request.method() {
            Method::Post => {
                let execution: ExecutionRequest = match read_json(&mut request) {
                    Ok(execution) => execution,
                    Err(error) => return api_error(request, 400, "invalid_request", error),
                };
                match session.enqueue(execution.source, execution.cell_id) {
                    Ok(id) => {
                        let snapshot = session
                            .execution(&id)
                            .and_then(|execution| execution.lock().ok().map(|item| item.snapshot()))
                            .unwrap_or_else(|| json!({ "id": id, "status": "queued" }));
                        json_response(request, 202, snapshot)
                    }
                    Err(error) => api_error(request, 500, "execution_failed", error),
                }
            }
            _ => method_not_allowed(request),
        },
        ["v1", "notebook-sessions", _, "executions", execution_id] => {
            if request.method() != &Method::Get {
                return method_not_allowed(request);
            }
            let Some(execution) = session.execution(execution_id) else {
                return api_error(
                    request,
                    404,
                    "execution_not_found",
                    "Execution does not exist",
                );
            };
            let snapshot = execution
                .lock()
                .map(|execution| execution.snapshot())
                .unwrap_or_else(|_| json!({ "error": "execution_unavailable" }));
            json_response(request, 200, snapshot)
        }
        ["v1", "notebook-sessions", _, "executions", execution_id, "cancel"] => {
            if request.method() != &Method::Post {
                return method_not_allowed(request);
            }
            cancel_execution(request, &session, execution_id)
        }
        ["v1", "notebook-sessions", _, "events"] => {
            if request.method() != &Method::Get {
                return method_not_allowed(request);
            }
            let after = query_value(query, "after")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            let entries = session
                .events
                .entries
                .lock()
                .map(|items| {
                    items
                        .iter()
                        .filter(|event| {
                            event
                                .get("sequence")
                                .and_then(JsonValue::as_u64)
                                .unwrap_or(0)
                                > after
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let next = entries
                .last()
                .and_then(|event| event.get("sequence"))
                .and_then(JsonValue::as_u64)
                .unwrap_or(after);
            json_response(
                request,
                200,
                json!({ "events": entries, "nextCursor": next }),
            )
        }
        _ => api_error(request, 404, "not_found", "No such session endpoint"),
    }
}

fn cancel_execution(request: Request, session: &Arc<NotebookSession>, id: &str) {
    let Some(execution) = session.execution(id) else {
        return api_error(
            request,
            404,
            "execution_not_found",
            "Execution does not exist",
        );
    };
    let mut item = match execution.lock() {
        Ok(item) => item,
        Err(_) => {
            return api_error(
                request,
                500,
                "execution_unavailable",
                "Execution is unavailable",
            )
        }
    };
    match item.status.as_str() {
        "queued" => {
            item.status = "cancelled".into();
            item.duration_ms = Some(0);
            drop(item);
            emit_execution_event(
                &execution,
                &session.events,
                json!({ "type": "completed", "status": "cancelled" }),
            );
            let snapshot = execution
                .lock()
                .map(|item| item.snapshot())
                .unwrap_or_default();
            json_response(request, 200, snapshot)
        }
        "running" => api_error(
            request,
            409,
            "interrupt_unavailable",
            "This native execution has already started and is not cooperatively interruptible yet",
        ),
        _ => json_response(request, 200, item.snapshot()),
    }
}

fn handle_jobs_collection(mut request: Request, state: &Arc<ServerState>) {
    match *request.method() {
        Method::Post => {
            let job: JobRequest = match read_json(&mut request) {
                Ok(job) => job,
                Err(error) => return api_error(request, 400, "invalid_request", error),
            };
            match submit_job(state, job) {
                Ok(snapshot) => json_response(request, 202, snapshot),
                Err(error) => api_error(request, 500, "job_failed", error),
            }
        }
        Method::Get => {
            let jobs = state
                .jobs
                .lock()
                .map(|jobs| {
                    jobs.values()
                        .filter_map(|job| job_snapshot(state, job))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json_response(request, 200, json!({ "jobs": jobs }))
        }
        _ => method_not_allowed(request),
    }
}

fn submit_job(state: &Arc<ServerState>, request: JobRequest) -> Result<JsonValue, String> {
    let session = create_session(state)?;
    let execution_id = session.enqueue(request.source.clone(), None)?;
    let id = random_id("job");
    let job = JobRecord {
        id: id.clone(),
        name: request.name,
        entrypoint: request.entrypoint,
        source: request.source,
        session_id: session.id.clone(),
        execution_id,
    };
    state
        .jobs
        .lock()
        .map_err(|_| "job store is unavailable".to_string())?
        .insert(id, job.clone());
    job_snapshot(state, &job).ok_or_else(|| "job execution disappeared".to_string())
}

fn handle_job_route(request: Request, state: &Arc<ServerState>, path: &str) {
    let parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    let Some(job_id) = parts.get(2).copied() else {
        return api_error(request, 404, "job_not_found", "Job was not specified");
    };
    let job = state
        .jobs
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(job_id).cloned());
    let Some(job) = job else {
        return api_error(request, 404, "job_not_found", "Job does not exist");
    };
    match parts.as_slice() {
        ["v1", "jobs", _] if request.method() == &Method::Get => match job_snapshot(state, &job) {
            Some(snapshot) => json_response(request, 200, snapshot),
            None => api_error(
                request,
                500,
                "job_unavailable",
                "Job execution is unavailable",
            ),
        },
        ["v1", "jobs", _, "cancel"] if request.method() == &Method::Post => {
            let Some(session) = get_session(state, &job.session_id) else {
                return api_error(
                    request,
                    404,
                    "session_not_found",
                    "Job session does not exist",
                );
            };
            cancel_execution(request, &session, &job.execution_id)
        }
        ["v1", "jobs", _, "retry"] if request.method() == &Method::Post => {
            match submit_job(
                state,
                JobRequest {
                    source: job.source,
                    name: job.name,
                    entrypoint: job.entrypoint,
                },
            ) {
                Ok(snapshot) => json_response(request, 202, snapshot),
                Err(error) => api_error(request, 500, "job_failed", error),
            }
        }
        ["v1", "jobs", _, "artifacts"] if request.method() == &Method::Get => {
            json_response(request, 200, json!({ "artifacts": [] }))
        }
        _ => method_not_allowed(request),
    }
}

fn job_snapshot(state: &Arc<ServerState>, job: &JobRecord) -> Option<JsonValue> {
    let session = get_session(state, &job.session_id)?;
    let execution = session.execution(&job.execution_id)?;
    let execution = execution.lock().ok()?;
    let exit_code = match execution.status.as_str() {
        "succeeded" => Some(0),
        "failed" => Some(1),
        _ => None,
    };
    Some(json!({
        "id": job.id,
        "name": job.name,
        "executor": "biolang",
        "entrypoint": job.entrypoint,
        "runtimeVersion": env!("CARGO_PKG_VERSION"),
        "resources": { "profile": "local" },
        "priority": 0,
        "tags": {},
        "dependsOn": [],
        "inputFiles": [],
        "inputsReady": true,
        "retryPolicy": {},
        "attempt": 1,
        "status": execution.status,
        "createdAt": rfc3339(execution.created_at),
        "startedAt": execution.started_at.map(rfc3339),
        "finishedAt": execution.terminal().then(|| rfc3339(execution.started_at.unwrap_or(execution.created_at) + execution.duration_ms.unwrap_or(0))),
        "durationMs": execution.duration_ms,
        "exitCode": exit_code,
        "stdout": execution.stdout,
        "stderr": execution.stderr,
        "stdoutOffset": 0,
        "stderrOffset": 0,
        "results": execution.results,
    }))
}

fn session_worker(
    receiver: mpsc::Receiver<SessionCommand>,
    events: Arc<SessionEvents>,
    notebook_path: PathBuf,
) {
    let mut interpreter = bl_runtime::Interpreter::new();
    interpreter.set_current_file(Some(notebook_path));
    while let Ok(command) = receiver.recv() {
        match command {
            SessionCommand::Shutdown => break,
            SessionCommand::Execute { source, execution } => {
                let cancelled = execution.lock().map(|item| item.terminal()).unwrap_or(true);
                if cancelled {
                    continue;
                }
                let started = Instant::now();
                if let Ok(mut item) = execution.lock() {
                    item.status = "running".into();
                    item.started_at = Some(now_ms());
                }
                emit_execution_event(&execution, &events, json!({ "type": "started" }));

                let output_execution = execution.clone();
                let output_events = events.clone();
                bl_runtime::builtins::set_output_sink(Some(Arc::new(move |text| {
                    if let Ok(mut item) = output_execution.lock() {
                        item.stdout.push_str(text);
                    }
                    emit_execution_event(
                        &output_execution,
                        &output_events,
                        json!({ "type": "stdout", "data": text }),
                    );
                })));
                let display_execution = execution.clone();
                let display_events = events.clone();
                bl_runtime::builtins::set_display_sink(Some(Arc::new(move |value, _| {
                    if !events::is_structured_result(value) {
                        return;
                    }
                    let result = events::value_to_json(value);
                    if let Ok(mut item) = display_execution.lock() {
                        if item.results.last() != Some(&result) {
                            item.results.push(result.clone());
                        }
                    }
                    emit_execution_event(
                        &display_execution,
                        &display_events,
                        json!({ "type": "result", "result": result }),
                    );
                })));

                let prepared = prepare_cell_source(&source);
                let result = if prepared.skip {
                    Ok(Value::Nil)
                } else if prepared.chat {
                    bl_runtime::llm::call_llm_builtin("chat", vec![Value::Str(prepared.source)])
                        .map_err(|error| error.to_string())
                } else {
                    evaluate_source(&prepared.source, &mut interpreter)
                };
                bl_runtime::builtins::flush_trailing_newline();
                bl_runtime::builtins::set_output_sink(None);
                bl_runtime::builtins::set_display_sink(None);

                match result {
                    Ok(value) => {
                        if !matches!(value, Value::Nil) {
                            let result = events::value_to_json(&value);
                            let mut new_result = false;
                            if let Ok(mut item) = execution.lock() {
                                if item.results.last() != Some(&result) {
                                    item.results.push(result.clone());
                                    new_result = true;
                                }
                            }
                            if new_result {
                                emit_execution_event(
                                    &execution,
                                    &events,
                                    json!({ "type": "result", "result": result }),
                                );
                            }
                        }
                        if let Ok(mut item) = execution.lock() {
                            item.status = "succeeded".into();
                            item.duration_ms = Some(started.elapsed().as_millis());
                        }
                        emit_execution_event(
                            &execution,
                            &events,
                            json!({ "type": "completed", "status": "succeeded", "durationMs": started.elapsed().as_millis() }),
                        );
                    }
                    Err(error) => {
                        if let Ok(mut item) = execution.lock() {
                            item.status = "failed".into();
                            item.stderr.push_str(&error);
                            if !error.ends_with('\n') {
                                item.stderr.push('\n');
                            }
                            item.duration_ms = Some(started.elapsed().as_millis());
                        }
                        emit_execution_event(
                            &execution,
                            &events,
                            json!({ "type": "stderr", "data": error }),
                        );
                        emit_execution_event(
                            &execution,
                            &events,
                            json!({ "type": "completed", "status": "failed", "durationMs": started.elapsed().as_millis() }),
                        );
                    }
                }
            }
        }
    }
}

struct PreparedCell {
    source: String,
    skip: bool,
    chat: bool,
}

/// Remove notebook-only directives before native evaluation.
///
/// They are comments to the language, but removing the header also gives
/// `@chat` its notebook meaning and keeps source locations consistent with the
/// terminal notebook runner.
fn prepare_cell_source(source: &str) -> PreparedCell {
    let mut skip = false;
    let mut chat = false;
    let mut body = Vec::new();
    let mut scanning = true;
    for line in source.lines() {
        if scanning {
            match line.trim() {
                "# @hide" | "# @hide-code" | "# @echo" | "# @hide-output" => continue,
                "# @skip" => {
                    skip = true;
                    continue;
                }
                "# @chat" => {
                    chat = true;
                    continue;
                }
                _ => scanning = false,
            }
        }
        body.push(line);
    }
    PreparedCell {
        source: body.join("\n"),
        skip,
        chat,
    }
}

fn evaluate_source(
    source: &str,
    interpreter: &mut bl_runtime::Interpreter,
) -> Result<Value, String> {
    let tokens = bl_lexer::Lexer::new(source)
        .tokenize()
        .map_err(|error| error.format_with_source(source))?;
    let parsed = bl_parser::Parser::new(tokens)
        .parse()
        .map_err(|error| error.format_with_source(source))?;
    if parsed.has_errors() {
        return Err(parsed
            .errors
            .iter()
            .map(|error| error.format_with_source(source))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    interpreter
        .run(&parsed.program)
        .map_err(|error| error.format_with_source(source))
}

fn emit_execution_event(
    execution: &Arc<Mutex<ExecutionState>>,
    session_events: &Arc<SessionEvents>,
    mut event: JsonValue,
) {
    let sequence = session_events.next_sequence.fetch_add(1, Ordering::Relaxed) + 1;
    let execution_id = execution
        .lock()
        .map(|item| item.id.clone())
        .unwrap_or_default();
    if let Some(object) = event.as_object_mut() {
        object.insert("sequence".into(), json!(sequence));
        object.insert("executionId".into(), json!(execution_id));
        object.insert("timestamp".into(), json!(now_ms()));
    }
    if let Ok(mut item) = execution.lock() {
        item.events.push(event.clone());
    }
    if let Ok(mut entries) = session_events.entries.lock() {
        entries.push(event);
        if entries.len() > MAX_EVENTS_PER_SESSION {
            let remove = entries.len() - MAX_EVENTS_PER_SESSION;
            entries.drain(0..remove);
        }
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(request: &mut Request) -> Result<T, String> {
    if request.body_length().unwrap_or(0) > MAX_REQUEST_BYTES {
        return Err(format!("request exceeds {MAX_REQUEST_BYTES} bytes"));
    }
    let mut body = Vec::new();
    request
        .as_reader()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| format!("cannot read request body: {error}"))?;
    if body.len() > MAX_REQUEST_BYTES {
        return Err(format!("request exceeds {MAX_REQUEST_BYTES} bytes"));
    }
    serde_json::from_slice(&body).map_err(|error| format!("invalid JSON: {error}"))
}

fn split_url(url: &str) -> (&str, Option<&str>) {
    match url.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (url, None),
    }
}

fn query_value<'a>(query: Option<&'a str>, key: &str) -> Option<&'a str> {
    query?.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then_some(value)
    })
}

fn request_header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.as_str())
}

fn valid_host(request: &Request, authority: &str) -> bool {
    let Some(host) = request_header(request, "Host") else {
        return false;
    };
    host == authority
}

fn valid_origin(request: &Request, authority: &str) -> bool {
    request_header(request, "Origin")
        .map(|origin| origin == format!("http://{authority}"))
        .unwrap_or(true)
}

fn authorized(request: &Request, token: &str) -> bool {
    let Some(value) = request_header(request, "Authorization") else {
        return false;
    };
    let Some(provided) = value.strip_prefix("Bearer ") else {
        return false;
    };
    constant_time_equal(provided.as_bytes(), token.as_bytes())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn random_id(prefix: &str) -> String {
    let bytes: [u8; 16] = rand::random();
    let encoded = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}_{encoded}")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

/// UTC RFC 3339 without pulling a date-time framework into the CLI.
fn rfc3339(milliseconds: u128) -> String {
    let seconds = (milliseconds / 1000) as i64;
    let millis = (milliseconds % 1000) as u32;
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);

    // Howard Hinnant's civil_from_days, with day zero at 1970-01-01.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_piece = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_piece + 2) / 5 + 1;
    let month = month_piece + if month_piece < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);

    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid response header")
}

fn standard_response(
    body: Vec<u8>,
    status: u16,
    content_type: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", content_type))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("X-Frame-Options", "DENY"))
        .with_header(header("Referrer-Policy", "no-referrer"))
}

fn json_response(request: Request, status: u16, value: JsonValue) {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
    let _ = request.respond(standard_response(
        body,
        status,
        "application/json; charset=utf-8",
    ));
}

fn html_response(request: Request, html: String) {
    let response = standard_response(html.into_bytes(), 200, "text/html; charset=utf-8")
        .with_header(header(
            "Content-Security-Policy",
            "default-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ));
    let _ = request.respond(response);
}

fn empty_response(request: Request, status: u16) {
    let _ = request.respond(standard_response(Vec::new(), status, "text/plain"));
}

fn api_error(request: Request, status: u16, code: &str, message: impl ToString) {
    json_response(
        request,
        status,
        json!({ "error": code, "message": message.to_string() }),
    );
}

fn method_not_allowed(request: Request) {
    api_error(
        request,
        405,
        "method_not_allowed",
        "Method is not supported for this endpoint",
    );
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn notebook_page(state: &ServerState) -> String {
    let token = serde_json::to_string(&state.token).unwrap_or_else(|_| "\"\"".into());
    let title = state
        .notebook_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("BioLang notebook");
    let title = html_escape(title);
    let figure_runtime = include_str!("../../../website/js/figure-fallback.js");
    NOTEBOOK_PAGE
        .replace("{title}", &title)
        .replace("{token}", &token)
        .replace("{figure_runtime}", figure_runtime)
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const NOTEBOOK_PAGE: &str = r###"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — BioLang Notebook</title>
<style>
:root{color-scheme:light dark;--bg:#0b1020;--panel:#111827;--cell:#172033;--border:#334155;--text:#e5e7eb;--muted:#94a3b8;--accent:#8b5cf6;--ok:#34d399;--bad:#fb7185}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:14px/1.55 system-ui,sans-serif}.bar{position:sticky;top:0;z-index:10;display:flex;gap:.65rem;align-items:center;padding:.65rem 1rem;background:#111827f2;border-bottom:1px solid var(--border);backdrop-filter:blur(8px)}.bar strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.bar .spacer{flex:1}.bar button,.cell button,.markdown-cell button{border:1px solid var(--border);border-radius:6px;padding:.42rem .72rem;background:#1e293b;color:var(--text);cursor:pointer}.bar button.primary,.cell button.primary{background:var(--accent);border-color:var(--accent);color:white}.bar button:disabled,.cell button:disabled{opacity:.55;cursor:wait}.status{color:var(--muted);font-size:.83rem}.shell{max-width:1040px;margin:1.5rem auto;padding:0 1rem 5rem}.notice{padding:.7rem .9rem;border:1px solid var(--border);border-radius:8px;background:var(--panel);color:var(--muted);margin-bottom:1rem}.markdown-cell{position:relative;margin:1rem 0}.markdown-head{display:flex;justify-content:flex-end;min-height:1.8rem}.markdown-head button{padding:.2rem .5rem;color:var(--muted);font-size:.75rem}.markdown-preview{color:#dbeafe}.markdown-preview h1,.markdown-preview h2,.markdown-preview h3,.markdown-preview h4{color:#f8fafc;margin:1.25rem 0 .55rem}.markdown-preview p{margin:.65rem 0}.markdown-preview code{padding:.1rem .3rem;border-radius:4px;background:#1e293b}.markdown-preview ul{padding-left:1.5rem}.markdown-preview blockquote{margin:.7rem 0;padding:.35rem .8rem;border-left:3px solid var(--accent);color:var(--muted)}.markdown-table-wrap{overflow-x:auto;margin:.8rem 0}.markdown-preview table{width:100%;border-collapse:collapse;background:var(--panel)}.markdown-preview th,.markdown-preview td{padding:.48rem .62rem;border:1px solid var(--border);text-align:left;vertical-align:top}.markdown-preview th{background:#1e293b;color:#f8fafc}.cell{margin:1rem 0 1.4rem;border:1px solid var(--border);border-radius:9px;background:var(--cell);overflow:hidden}.cell.running{border-color:var(--accent)}.cell-head{display:flex;align-items:center;gap:.6rem;padding:.45rem .65rem;border-bottom:1px solid var(--border);color:var(--muted);font-size:.8rem}.cell-head .spacer{flex:1}.editor{display:block;width:100%;min-height:110px;padding:.8rem;border:0;resize:vertical;background:#0f172a;color:#e2e8f0;font:13px/1.55 ui-monospace,SFMono-Regular,Consolas,monospace;tab-size:2;outline:none}.editor[hidden]{display:none}.output{padding:.75rem;border-top:1px solid var(--border);background:#0b1220}.output:empty{display:none}.output pre{margin:.35rem 0;white-space:pre-wrap;overflow-wrap:anywhere}.error{color:var(--bad)}.result{margin:.75rem 0;overflow:auto}.result table{border-collapse:collapse;min-width:50%}.result th,.result td{padding:.35rem .55rem;border:1px solid var(--border);text-align:left}.result svg{display:block;max-width:100%;height:auto;background:white;border-radius:6px}.report-frame{display:block;width:100%;height:620px;border:1px solid var(--border);border-radius:7px;background:white}.report-tools{display:flex;gap:.45rem;flex-wrap:wrap;margin:.55rem 0}.report-actions{border:1px solid var(--border);border-radius:7px;padding:.55rem .7rem}.report-action{padding:.5rem 0;border-bottom:1px solid var(--border)}.report-action:last-child{border-bottom:0}.report-action small{display:block;color:var(--muted)}.report-action code{display:block;margin:.35rem 0;white-space:pre-wrap}.figure-fallback-toggle{margin-top:.4rem}.dirty{color:#fbbf24}@media(max-width:640px){.bar{flex-wrap:wrap}.status{width:100%}.shell{padding:0 .55rem}.editor{font-size:12px}.report-frame{height:500px}}
</style></head><body>
<header class="bar"><strong id="title">{title}</strong><span id="dirty" class="dirty" hidden>unsaved</span><span class="spacer"></span><button id="save">Save</button><button id="run-all" class="primary">Run all</button><span id="status" class="status">Connecting…</span></header>
<main class="shell"><div class="notice">Native local kernel. Code runs with your user permissions and can use local BioLang packages, files, memory and the selected compute backend.</div><div id="notebook"></div></main>
<script>{figure_runtime}</script>
<script>
const TOKEN={token};let source="",revision=0,sessionId="",dirty=false,executedThrough=0,sessionStale=false;
const terminal=new Set(["succeeded","failed","cancelled"]);
const $=id=>document.getElementById(id);const escapeHtml=s=>s.replace(/[&<>\"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[c]));
async function api(path,options={}){const response=await fetch(path,{...options,headers:{"authorization":"Bearer "+TOKEN,"content-type":"application/json",...(options.headers||{})}});const body=await response.json().catch(()=>({}));if(!response.ok)throw new Error(body.message||response.statusText);return body}
function linesWithOffsets(text){const lines=[];let start=0;for(const match of text.matchAll(/.*(?:\r\n|\n|$)/g)){if(!match[0])continue;lines.push({start,end:start+match[0].length,text:match[0],value:match[0].replace(/\r?\n$/,"")});start+=match[0].length}return lines}
function parseNotebook(text){const blocks=[];const lines=linesWithOffsets(text);let proseStart=0,codeStart=0,inCode=false,legacy=false,otherFence=false,codeIndex=0,frontMatterEnd=0;const pushProse=end=>{if(end>proseStart&&text.slice(proseStart,end).trim())blocks.push({type:"markdown",start:proseStart,end,content:text.slice(proseStart,end)})};if(lines[0]?.value.trim()==="---"){for(let i=1;i<lines.length;i++){if(lines[i].value.trim()!=="---")continue;const fields=lines.slice(1,i).filter(line=>line.value.trim());if(fields.length&&fields.every(line=>/^[A-Za-z][\w-]*\s*:/.test(line.value))){frontMatterEnd=lines[i].end}break}}for(const line of lines){if(line.start<frontMatterEnd)continue;const t=line.value.trim().toLowerCase();const bio=t==="```"||t==="```biolang"||t==="```bl";if(otherFence){if(t==="```")otherFence=false;continue}if(!inCode&&t.startsWith("```")&&!bio){otherFence=true;continue}if(!inCode&&bio){pushProse(line.start);inCode=true;legacy=false;codeStart=line.end;continue}if(inCode&&!legacy&&t==="```"){blocks.push({type:"code",start:codeStart,end:line.start,content:text.slice(codeStart,line.start),index:codeIndex++});inCode=false;proseStart=line.end;continue}if(t==="---"){if(!inCode){pushProse(line.start);inCode=true;legacy=true;codeStart=line.end}else if(legacy){blocks.push({type:"code",start:codeStart,end:line.start,content:text.slice(codeStart,line.start),index:codeIndex++});inCode=false;legacy=false;proseStart=line.end}}}if(inCode)blocks.push({type:"code",start:codeStart,end:text.length,content:text.slice(codeStart),index:codeIndex++});else pushProse(text.length);return blocks}
function replaceBlock(type,index,value){const blocks=parseNotebook(source).filter(block=>block.type===type);const block=blocks[index];if(!block)return;source=source.slice(0,block.start)+value+source.slice(block.end);dirty=true;$("dirty").hidden=false}
async function renderMarkdown(preview,value){preview.dataset.source=value;const rendered=await api("/v1/render-markdown",{method:"POST",body:JSON.stringify({source:value})});if(preview.dataset.source===value)preview.innerHTML=rendered.html}
function scalar(value){if(value&&typeof value==="object"&&"kind" in value&&"value" in value)return value.value;return value}
function listItems(value){return value?.kind==="list"?(value.items||[]):(Array.isArray(value)?value:[])}
function addCodeToCurrentCell(container,code){const editor=container.closest(".cell")?.querySelector(".editor");if(!editor)return;editor.value=editor.value.replace(/\s*$/,"")+"\n"+code+"\n";editor.dispatchEvent(new Event("input",{bubbles:true}));editor.focus();editor.setSelectionRange(editor.value.length,editor.value.length)}
function renderStatsReport(result,container){const value=scalar(result),content=scalar(value.content)||"",format=scalar(value.format)||"html",title=scalar(value.title)||"biolang-statistics-report";const wrap=document.createElement("div");wrap.className="result stats-report";const tools=document.createElement("div");tools.className="report-tools";const download=document.createElement("button");download.textContent="Download report";download.addEventListener("click",()=>{const blob=new Blob([content],{type:format==="html"?"text/html":"text/markdown"}),url=URL.createObjectURL(blob),link=document.createElement("a");link.href=url;link.download=title.replace(/[^a-z0-9_-]+/gi,"-").replace(/^-|-$/g,"")+(format==="html"?".html":".md");link.click();setTimeout(()=>URL.revokeObjectURL(url),1000)});tools.append(download);wrap.append(tools);if(format==="html"){const frame=document.createElement("iframe");frame.className="report-frame";frame.setAttribute("sandbox","");frame.srcdoc=content;frame.title=title;wrap.append(frame)}else{const pre=document.createElement("pre");pre.textContent=content;wrap.append(pre)}const scan=scalar(value.scan)||{},recommendations=listItems(scan.recommendations);if(recommendations.length){const details=document.createElement("details");details.className="report-actions";const summary=document.createElement("summary");summary.textContent="Copy or insert suggested BioLang commands";details.append(summary);for(const encoded of recommendations){const item=scalar(encoded)||{},next=scalar(item.next_step)||"Review this evidence",evidence=scalar(item.evidence)||"",code=scalar(item.example)||"";if(!code)continue;const row=document.createElement("div");row.className="report-action";const label=document.createElement("strong");label.textContent=next;const note=document.createElement("small");note.textContent="Evidence: "+evidence;const snippet=document.createElement("code");snippet.textContent=code;const copy=document.createElement("button");copy.textContent="Copy";copy.addEventListener("click",async()=>{await navigator.clipboard.writeText(code);copy.textContent="Copied";setTimeout(()=>copy.textContent="Copy",1000)});const insert=document.createElement("button");insert.textContent="Insert in cell";insert.addEventListener("click",()=>addCodeToCurrentCell(container,code));row.append(label,note,snippet,copy,insert);details.append(row)}wrap.append(details)}container.append(wrap)}
function renderResult(result,container){const value=scalar(result);if(result.kind==="record"&&scalar(value?.schema)==="biolang.stats.report/v1"){renderStatsReport(result,container);return}const wrap=document.createElement("div");wrap.className="result";if(result.kind==="plot"&&result.format==="svg"){wrap.className="result cell-figure";wrap.innerHTML=result.data}else if(result.kind==="table"){const table=document.createElement("table"),head=document.createElement("thead"),tr=document.createElement("tr");for(const column of result.columns||[]){const th=document.createElement("th");th.textContent=column;tr.append(th)}head.append(tr);table.append(head);const body=document.createElement("tbody");for(const row of result.rows||[]){const line=document.createElement("tr");for(const cell of row){const td=document.createElement("td");td.textContent=String(scalar(cell)??"");line.append(td)}body.append(line)}table.append(body);wrap.append(table)}else{const pre=document.createElement("pre");pre.textContent=JSON.stringify(value,null,2);wrap.append(pre)}container.append(wrap)}
function renderExecution(execution,output){output.replaceChildren();if(execution.stdout){const pre=document.createElement("pre");pre.textContent=execution.stdout;output.append(pre)}if(execution.stderr){const pre=document.createElement("pre");pre.className="error";pre.textContent=execution.stderr;output.append(pre)}for(const result of execution.results||[])renderResult(result,output);document.dispatchEvent(new CustomEvent("bl:figures-updated"))}
async function waitExecution(id,output,cell,hideOutput=false){while(true){const execution=await api(`/v1/notebook-sessions/${sessionId}/executions/${id}`);if(!hideOutput)renderExecution(execution,output);if(terminal.has(execution.status)){cell.classList.remove("running");if(hideOutput)output.textContent="Output hidden by @hide-output";return execution}await new Promise(resolve=>setTimeout(resolve,100))}}
async function executeCell(index,cell,output){const block=parseNotebook(source).filter(item=>item.type==="code")[index];if(!block)return {status:"succeeded"};if(/^\s*#\s*@skip\b/m.test(block.content)){output.textContent="Skipped by @skip";return {status:"succeeded"}}const hideOutput=/^\s*#\s*@hide-output\b/m.test(block.content);cell.classList.add("running");const submitted=await api(`/v1/notebook-sessions/${sessionId}/executions`,{method:"POST",body:JSON.stringify({cellId:`cell-${index+1}`,source:block.content})});return waitExecution(submitted.id,output,cell,hideOutput)}
async function runThrough(index){const cells=[...document.querySelectorAll(".cell")];if(sessionStale||index<executedThrough)await newSession();for(let current=executedThrough;current<=index;current++){const execution=await executeCell(current,cells[current],cells[current].querySelector(".output"));if(execution.status!=="succeeded")return false;executedThrough=current+1}return true}
function render(){const host=$("notebook");host.replaceChildren();let markdownIndex=0;for(const block of parseNotebook(source)){if(block.type==="markdown"){const shell=document.createElement("section"),head=document.createElement("div"),toggle=document.createElement("button"),preview=document.createElement("div"),area=document.createElement("textarea");shell.className="markdown-cell";head.className="markdown-head";toggle.textContent="Edit";preview.className="markdown-preview";area.className="editor";area.value=block.content;area.hidden=true;const index=markdownIndex++;renderMarkdown(preview,area.value).catch(error=>{preview.textContent=String(error)});area.addEventListener("input",()=>{replaceBlock("markdown",index,area.value);renderMarkdown(preview,area.value).catch(error=>{preview.textContent=String(error)})});toggle.addEventListener("click",()=>{const editing=area.hidden;area.hidden=!editing;preview.hidden=editing;toggle.textContent=editing?"Preview":"Edit";if(editing)area.focus()});head.append(toggle);shell.append(head,preview,area);host.append(shell);continue}const cell=document.createElement("section");cell.className="cell";const head=document.createElement("div");head.className="cell-head";head.innerHTML=`<span>In [${block.index+1}]</span><span class="spacer"></span>`;const run=document.createElement("button");run.className="primary";run.textContent="Run";run.title="Run any required earlier cells, then this cell";head.append(run);const editor=document.createElement("textarea");editor.className="editor";editor.value=block.content;editor.addEventListener("input",()=>{if(block.index<executedThrough)sessionStale=true;executedThrough=Math.min(executedThrough,block.index);replaceBlock("code",block.index,editor.value)});editor.addEventListener("keydown",event=>{if((event.ctrlKey||event.metaKey)&&event.key==="Enter"){event.preventDefault();run.click()}});const output=document.createElement("div");output.className="output";run.addEventListener("click",()=>runThrough(block.index).catch(error=>{cell.classList.remove("running");output.innerHTML=`<pre class="error">${escapeHtml(String(error))}</pre>`}));cell.append(head,editor,output);host.append(cell)}}
async function newSession(){if(sessionId)await api(`/v1/notebook-sessions/${sessionId}`,{method:"DELETE"}).catch(()=>{});const session=await api("/v1/notebook-sessions",{method:"POST",body:"{}"});sessionId=session.id;executedThrough=0;sessionStale=false}
async function load(){const notebook=await api("/v1/notebook");source=notebook.source;revision=notebook.revision;$("title").textContent=notebook.name;$("status").textContent=notebook.backend;await newSession();render()}
$("save").addEventListener("click",async()=>{const button=$("save");button.disabled=true;try{const saved=await api("/v1/notebook",{method:"PUT",body:JSON.stringify({source,expectedRevision:revision})});revision=saved.revision;dirty=false;$("dirty").hidden=true;$("status").textContent="Saved"}catch(error){$("status").textContent=String(error)}finally{button.disabled=false}});
$("run-all").addEventListener("click",async()=>{const button=$("run-all");button.disabled=true;try{await newSession();const cells=[...document.querySelectorAll(".cell")];const ok=!cells.length||await runThrough(cells.length-1);$("status").textContent=ok?"Run complete":"Run stopped at the first error"}catch(error){$("status").textContent=String(error)}finally{button.disabled=false}});
window.addEventListener("beforeunload",event=>{if(dirty){event.preventDefault();event.returnValue=""}});load().catch(error=>{$("status").textContent=String(error)});
</script></body></html>"###;

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(session: &Arc<NotebookSession>, id: &str) -> JsonValue {
        for _ in 0..200 {
            if let Some(execution) = session.execution(id) {
                if let Ok(item) = execution.lock() {
                    if item.terminal() {
                        return item.snapshot();
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("execution did not finish");
    }

    #[test]
    fn session_preserves_interpreter_context_between_cells() {
        let session = NotebookSession::start(random_id("test"), PathBuf::from("test.bln"));
        let first = session
            .enqueue("let x = 41".into(), Some("one".into()))
            .unwrap();
        assert_eq!(wait(&session, &first)["status"], "succeeded");
        let second = session.enqueue("x + 1".into(), Some("two".into())).unwrap();
        let result = wait(&session, &second);
        assert_eq!(result["status"], "succeeded");
        assert_eq!(result["results"][0]["kind"], "integer");
        assert_eq!(result["results"][0]["value"], 42);
    }

    #[test]
    fn session_reports_structured_plot_results() {
        let session = NotebookSession::start(random_id("test"), PathBuf::from("test.bln"));
        let id = session
            .enqueue(
                "histogram([1, 2, 2, 3], {title: \"Counts\"})".into(),
                Some("plot".into()),
            )
            .unwrap();
        let result = wait(&session, &id);
        assert_eq!(result["status"], "succeeded");
        assert_eq!(result["results"][0]["kind"], "plot");
        assert!(result["results"][0]["data"]
            .as_str()
            .unwrap()
            .contains("aria-label=\"Counts\""));
    }

    #[test]
    fn event_feed_has_monotone_session_sequences() {
        let session = NotebookSession::start(random_id("test"), PathBuf::from("test.bln"));
        let id = session.enqueue("println(\"hello\")".into(), None).unwrap();
        wait(&session, &id);
        let entries = session.events.entries.lock().unwrap();
        let sequences = entries
            .iter()
            .map(|event| event["sequence"].as_u64().unwrap())
            .collect::<Vec<_>>();
        assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(entries.iter().any(|event| event["type"] == "stdout"));
        assert!(entries.iter().any(|event| event["type"] == "completed"));
    }

    #[test]
    fn bearer_comparison_rejects_different_lengths_and_values() {
        assert!(constant_time_equal(b"secret", b"secret"));
        assert!(!constant_time_equal(b"secret", b"secrex"));
        assert!(!constant_time_equal(b"secret", b"secret-long"));
    }

    #[test]
    fn notebook_directives_are_removed_before_execution() {
        let prepared = prepare_cell_source("# @hide\n# @echo\nlet x = 3");
        assert_eq!(prepared.source, "let x = 3");
        assert!(!prepared.skip);
        assert!(!prepared.chat);

        let skipped = prepare_cell_source("# @skip\nprintln(\"no\")");
        assert!(skipped.skip);
        assert_eq!(skipped.source, "println(\"no\")");

        let chat = prepare_cell_source("# @chat\nExplain this table");
        assert!(chat.chat);
        assert_eq!(chat.source, "Explain this table");
    }

    #[test]
    fn generated_page_contains_no_token_placeholder() {
        let state = ServerState {
            token: "test-token".into(),
            authority: "127.0.0.1:8765".into(),
            notebook_path: PathBuf::from("lesson.bln"),
            root: PathBuf::from("."),
            document: Mutex::new(NotebookDocument {
                source: String::new(),
                revision: 1,
            }),
            sessions: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
        };
        let page = notebook_page(&state);
        assert!(page.contains("const TOKEN=\"test-token\""));
        assert!(!page.contains("{token}"));
        assert!(page.contains("/v1/notebook-sessions"));
        assert!(page.contains("/v1/render-markdown"));
        assert!(page.contains("markdown-preview"));
        assert!(page.contains("bl:figures-updated"));
        assert!(page.contains("biolang.stats.report/v1"));
        assert!(page.contains("Download report"));
        assert!(page.contains("Insert in cell"));
        assert!(page.contains("setAttribute(\"sandbox\",\"\")"));
    }

    #[test]
    fn timestamps_match_the_somer_rfc3339_shape() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(1_723_162_896_123), "2024-08-09T00:21:36.123Z");
    }
}
