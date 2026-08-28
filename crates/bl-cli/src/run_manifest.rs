use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: &str = "biolang.run/v1";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunOptions {
    pub verbose: bool,
    pub events: bool,
    pub print_result: bool,
    pub seed: Option<u64>,
    /// Kept here for recorder construction, but serialized once at the
    /// manifest's top-level `parameters` field.
    #[serde(skip_serializing)]
    pub parameters: BTreeMap<String, JsonValue>,
    pub plot_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Artifact {
    path: String,
    canonical_path: Option<String>,
    kind: &'static str,
    exists: bool,
    bytes: u64,
    file_count: u64,
    sha256: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    biolang_version: &'static str,
    executable: Option<String>,
    executable_sha256: Option<String>,
    os: &'static str,
    architecture: &'static str,
    process_id: u32,
    available_threads: usize,
}

pub struct RunRecorder {
    destination: PathBuf,
    started_at_unix_ms: u128,
    script: Artifact,
    inputs: Vec<Artifact>,
    declared_outputs: Vec<PathBuf>,
    options: RunOptions,
    compute_backend: String,
    working_directory: String,
    package_manifest: Option<Artifact>,
    provenance_start: usize,
    preflight_errors: Vec<String>,
    safe_destination: bool,
}

impl RunRecorder {
    pub fn new(
        destination: PathBuf,
        script: &Path,
        inputs: &[PathBuf],
        outputs: &[PathBuf],
        options: RunOptions,
        compute_backend: String,
    ) -> Self {
        let working_directory = std::env::current_dir().unwrap_or_default();
        let script_artifact = inspect_artifact(script);
        let input_artifacts = inputs
            .iter()
            .map(|path| inspect_artifact(path))
            .collect::<Vec<_>>();
        let mut preflight_errors = Vec::new();
        if let Some(error) = &script_artifact.error {
            preflight_errors.push(format!("script '{}': {error}", script.display()));
        }
        for input in &input_artifacts {
            if let Some(error) = &input.error {
                preflight_errors.push(format!("input '{}': {error}", input.path));
            }
        }
        let normalized_destination = normalized_absolute(&destination, &working_directory);
        let normalized_script = normalized_absolute(script, &working_directory);
        let mut safe_destination = true;
        if same_path(&normalized_destination, &normalized_script) {
            preflight_errors.push("run record destination cannot overwrite the script".to_string());
            safe_destination = false;
        }
        for (label, paths) in [("input", inputs), ("output", outputs)] {
            for path in paths {
                let normalized = normalized_absolute(path, &working_directory);
                if path_contains(&normalized, &normalized_destination) {
                    preflight_errors.push(format!(
                        "run record '{}' cannot be inside declared {label} '{}' because it would change that artifact's hash",
                        destination.display(),
                        path.display()
                    ));
                    safe_destination = false;
                }
            }
        }
        let package_manifest = nearest_package_manifest(script).map(|path| inspect_artifact(&path));
        let provenance_start = read_provenance(&working_directory).len();

        Self {
            destination,
            started_at_unix_ms: now_unix_ms(),
            script: script_artifact,
            inputs: input_artifacts,
            declared_outputs: outputs.to_vec(),
            options,
            compute_backend,
            working_directory: slash_path(&working_directory),
            package_manifest,
            provenance_start,
            preflight_errors,
            safe_destination,
        }
    }

    pub fn preflight_error(&self) -> Option<String> {
        (!self.preflight_errors.is_empty()).then(|| self.preflight_errors.join("\n"))
    }

    pub fn can_write_safely(&self) -> bool {
        self.safe_destination
    }

    pub fn postflight_error(&self) -> Option<String> {
        let problems = self
            .declared_outputs
            .iter()
            .filter_map(|path| {
                fs::symlink_metadata(path)
                    .and_then(|metadata| {
                        if metadata.file_type().is_symlink() {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "symbolic links must be declared by their resolved target",
                            ))
                        } else if metadata.is_file() || metadata.is_dir() {
                            Ok(())
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "path is neither a regular file nor a directory",
                            ))
                        }
                    })
                    .err()
                    .map(|error| format!("output '{}': {error}", path.display()))
            })
            .collect::<Vec<_>>();
        (!problems.is_empty()).then(|| problems.join("\n"))
    }

    pub fn finish(
        &self,
        status: &'static str,
        duration_ms: u128,
        error: Option<&str>,
    ) -> Result<(), String> {
        self.finish_with_dependencies(status, duration_ms, error, &[])
    }

    pub fn finish_with_dependencies(
        &self,
        status: &'static str,
        duration_ms: u128,
        error: Option<&str>,
        dependencies: &[PathBuf],
    ) -> Result<(), String> {
        let outputs = self
            .declared_outputs
            .iter()
            .map(|path| inspect_artifact(path))
            .collect::<Vec<_>>();
        let mut warnings = Vec::new();
        for output in &outputs {
            if let Some(problem) = &output.error {
                warnings.push(format!("declared output '{}': {problem}", output.path));
            }
        }
        let provenance = read_provenance(Path::new(&self.working_directory));
        let decisions = provenance
            .into_iter()
            .skip(self.provenance_start)
            .filter(|decision| {
                decision.get("process_id").and_then(JsonValue::as_u64)
                    == Some(std::process::id() as u64)
            })
            .collect::<Vec<_>>();
        for decision in &decisions {
            if let Some(items) = decision.get("warnings").and_then(JsonValue::as_array) {
                warnings.extend(
                    items
                        .iter()
                        .filter_map(JsonValue::as_str)
                        .map(str::to_owned),
                );
            }
        }
        let modules = dependencies
            .iter()
            .map(|path| inspect_artifact(path))
            .collect::<Vec<_>>();
        let executable_path = std::env::current_exe().ok();
        let executable_sha256 = executable_path
            .as_ref()
            .and_then(|path| hash_artifact(path).ok().map(|artifact| artifact.3));

        let document = json!({
            "schema": SCHEMA,
            "status": status,
            "startedAtUnixMs": self.started_at_unix_ms,
            "finishedAtUnixMs": now_unix_ms(),
            "durationMs": duration_ms,
            "workingDirectory": self.working_directory,
            "script": self.script,
            "inputs": self.inputs,
            "inputTracking": "declared",
            "outputs": outputs,
            "parameters": self.options.parameters,
            "options": self.options,
            "compute": {
                "backend": self.compute_backend,
                "gpuPolicy": std::env::var("BIOLANG_GPU").unwrap_or_else(|_| "auto".to_string()),
                "backendDecisions": decisions,
            },
            "runtime": RuntimeInfo {
                biolang_version: env!("CARGO_PKG_VERSION"),
                executable: executable_path.as_ref().map(|path| slash_path(path)),
                executable_sha256,
                os: std::env::consts::OS,
                architecture: std::env::consts::ARCH,
                process_id: std::process::id(),
                available_threads: std::thread::available_parallelism().map(usize::from).unwrap_or(1),
            },
            "resources": {
                "peakResidentBytes": peak_resident_bytes(),
                "measurement": "operating-system process high-water mark",
            },
            "packageManifest": self.package_manifest,
            "loadedModules": modules,
            "warnings": warnings,
            "warningCapture": "CLI, declared artifacts, and backend decisions",
            "error": error,
        });
        write_atomic_json(&self.destination, &document)
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

pub fn parse_parameters(
    raw: &[String],
) -> Result<
    (
        BTreeMap<String, JsonValue>,
        std::collections::HashMap<String, bl_core::value::Value>,
    ),
    String,
> {
    let mut json_values = BTreeMap::new();
    let mut runtime_values = std::collections::HashMap::new();
    for item in raw {
        let Some((key, text)) = item.split_once('=') else {
            return Err(format!("invalid --param '{item}'; expected NAME=VALUE"));
        };
        let key = key.trim();
        if key.is_empty()
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            return Err(format!(
                "invalid --param name '{key}'; use letters, numbers, '.', '_' or '-'"
            ));
        }
        let parsed = serde_json::from_str::<JsonValue>(text)
            .unwrap_or_else(|_| JsonValue::String(text.to_string()));
        if json_values.contains_key(key) {
            return Err(format!("duplicate --param name '{key}'"));
        }
        runtime_values.insert(key.to_string(), json_to_biolang(&parsed));
        json_values.insert(key.to_string(), parsed);
    }
    Ok((json_values, runtime_values))
}

fn json_to_biolang(value: &JsonValue) -> bl_core::value::Value {
    use bl_core::value::Value;
    match value {
        JsonValue::Null => Value::Nil,
        JsonValue::Bool(value) => Value::Bool(*value),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Int)
            .unwrap_or_else(|| Value::Float(value.as_f64().unwrap_or(f64::NAN))),
        JsonValue::String(value) => Value::Str(value.clone()),
        JsonValue::Array(values) => Value::List(
            values
                .iter()
                .map(json_to_biolang)
                .collect::<Vec<_>>()
                .into(),
        ),
        JsonValue::Object(values) => Value::Record(
            values
                .iter()
                .map(|(key, value)| (key.clone(), json_to_biolang(value)))
                .collect::<std::collections::HashMap<_, _>>()
                .into(),
        ),
    }
}

fn inspect_artifact(path: &Path) -> Artifact {
    match hash_artifact(path) {
        Ok((kind, bytes, file_count, digest)) => Artifact {
            path: slash_path(path),
            canonical_path: fs::canonicalize(path).ok().map(|path| slash_path(&path)),
            kind,
            exists: true,
            bytes,
            file_count,
            sha256: Some(digest),
            error: None,
        },
        Err(error) => Artifact {
            path: slash_path(path),
            canonical_path: None,
            kind: "missing",
            exists: false,
            bytes: 0,
            file_count: 0,
            sha256: None,
            error: Some(error.to_string()),
        },
    }
}

fn hash_artifact(path: &Path) -> io::Result<(&'static str, u64, u64, String)> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic links must be declared by their resolved target",
        ));
    }
    if metadata.is_file() {
        let mut hasher = Sha256::new();
        let bytes = hash_file(path, &mut hasher)?;
        return Ok(("file", bytes, 1, hex_digest(hasher)));
    }
    if metadata.is_dir() {
        let mut files = Vec::new();
        collect_directory_files(path, path, &mut files)?;
        files.sort_by(|left, right| left.0.cmp(&right.0));
        let mut hasher = Sha256::new();
        hasher.update(b"biolang-directory/v2\0");
        hasher.update((files.len() as u64).to_le_bytes());
        let mut total = 0u64;
        for (relative, file) in &files {
            let name = relative.as_bytes();
            hasher.update((name.len() as u64).to_le_bytes());
            hasher.update(name);
            total = total.saturating_add(hash_framed_file(file, &mut hasher)?);
        }
        return Ok(("directory", total, files.len() as u64, hex_digest(hasher)));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "path is neither a regular file nor a directory",
    ))
}

fn collect_directory_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("directory contains symbolic link '{}'", path.display()),
            ));
        }
        if kind.is_dir() {
            collect_directory_files(root, &path, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, path));
        }
    }
    Ok(())
}

fn hash_file(path: &Path, hasher: &mut Sha256) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        total = total.saturating_add(count as u64);
    }
    Ok(total)
}

/// Hash one directory entry with an unambiguous content boundary.
///
/// The declared length is checked after reading so a file modified during the
/// provenance scan cannot silently produce a malformed framing.
fn hash_framed_file(path: &Path, hasher: &mut Sha256) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let declared = file.metadata()?.len();
    hasher.update(declared.to_le_bytes());
    let mut buffer = [0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        total = total.saturating_add(count as u64);
    }
    if total != declared {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("file changed while hashing: expected {declared} bytes, read {total}"),
        ));
    }
    Ok(total)
}

fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn nearest_package_manifest(script: &Path) -> Option<PathBuf> {
    let mut directory = fs::canonicalize(script).ok()?.parent()?.to_path_buf();
    loop {
        let candidate = directory.join("biolang.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !directory.pop() {
            return None;
        }
    }
}

fn read_provenance(root: &Path) -> Vec<JsonValue> {
    fs::read_to_string(root.join(".biolang").join("provenance.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_atomic_json(destination: &Path, document: &JsonValue) -> Result<(), String> {
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create manifest directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let temporary = destination.with_extension(format!("json.tmp-{}", std::process::id()));
    let text = serde_json::to_string_pretty(document).map_err(|error| error.to_string())?;
    fs::write(&temporary, format!("{text}\n"))
        .map_err(|error| format!("cannot write '{}': {error}", temporary.display()))?;
    match fs::rename(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let cleanup = fs::remove_file(&temporary);
            let cleanup_note = cleanup
                .err()
                .map(|cleanup_error| {
                    format!("; temporary file cleanup also failed: {cleanup_error}")
                })
                .unwrap_or_default();
            Err(format!(
                "cannot finalize '{}': {error}{cleanup_note}",
                destination.display()
            ))
        }
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalized_absolute(path: &Path, working_directory: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_directory.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        slash_path(left).eq_ignore_ascii_case(&slash_path(right))
    } else {
        left == right
    }
}

fn path_contains(scope: &Path, candidate: &Path) -> bool {
    if cfg!(windows) {
        let scope = slash_path(scope).to_ascii_lowercase();
        let candidate = slash_path(candidate).to_ascii_lowercase();
        candidate == scope
            || candidate
                .strip_prefix(&scope)
                .is_some_and(|remainder| remainder.starts_with('/'))
    } else {
        candidate == scope || candidate.starts_with(scope)
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(windows)]
fn peak_resident_bytes() -> Option<u64> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let mut counters = unsafe { std::mem::zeroed::<PROCESS_MEMORY_COUNTERS>() };
    counters.cb = size;
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
    (ok != 0).then_some(counters.PeakWorkingSetSize as u64)
}

#[cfg(unix)]
fn peak_resident_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let ok = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if ok != 0 {
        return None;
    }
    let high_water = unsafe { usage.assume_init().ru_maxrss as u64 };
    #[cfg(target_os = "macos")]
    {
        Some(high_water)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(high_water.saturating_mul(1024))
    }
}

#[cfg(not(any(windows, unix)))]
fn peak_resident_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::{hash_artifact, parse_parameters, write_atomic_json};
    use serde_json::json;

    #[test]
    fn parameters_preserve_scalar_types_and_plain_strings() {
        let (json_values, runtime_values) = parse_parameters(&[
            "count=3".into(),
            "enabled=true".into(),
            "label=treated".into(),
        ])
        .unwrap();
        assert_eq!(json_values["count"], json!(3));
        assert_eq!(json_values["enabled"], json!(true));
        assert_eq!(runtime_values["label"].to_string(), "treated");
        assert!(parse_parameters(&["count=1".into(), "count=2".into()]).is_err());
    }

    #[test]
    fn directory_hash_is_stable_and_content_sensitive() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("a.txt"), "A").unwrap();
        std::fs::create_dir(directory.path().join("nested")).unwrap();
        std::fs::write(directory.path().join("nested").join("b.txt"), "B").unwrap();
        let first = hash_artifact(directory.path()).unwrap();
        let second = hash_artifact(directory.path()).unwrap();
        assert_eq!(first.3, second.3);
        assert_eq!(first.2, 2);
        std::fs::write(directory.path().join("nested").join("b.txt"), "changed").unwrap();
        assert_ne!(first.3, hash_artifact(directory.path()).unwrap().3);
    }

    #[test]
    fn directory_hash_frames_file_contents_and_file_count() {
        let root = tempfile::tempdir().unwrap();
        let one = root.path().join("one");
        let two = root.path().join("two");
        std::fs::create_dir(&one).unwrap();
        std::fs::create_dir(&two).unwrap();
        let mut ambiguous_content = b"A".to_vec();
        ambiguous_content.extend_from_slice(&5u64.to_le_bytes());
        ambiguous_content.extend_from_slice(b"b.txtB");
        std::fs::write(one.join("a"), ambiguous_content).unwrap();
        std::fs::write(two.join("a"), "A").unwrap();
        std::fs::write(two.join("b.txt"), "B").unwrap();

        assert_ne!(
            hash_artifact(&one).unwrap().3,
            hash_artifact(&two).unwrap().3
        );
    }

    #[test]
    fn atomic_json_replaces_an_existing_record_without_a_delete_step() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("run.json");
        write_atomic_json(&destination, &json!({"generation": 1})).unwrap();
        write_atomic_json(&destination, &json!({"generation": 2})).unwrap();

        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
        assert_eq!(document["generation"], 2);
        assert!(!destination
            .with_extension(format!("json.tmp-{}", std::process::id()))
            .exists());
    }
}
