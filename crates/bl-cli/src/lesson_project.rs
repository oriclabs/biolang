use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LOCK_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonDataLock {
    schema: u8,
    kind: String,
    project: LessonProject,
    files: Vec<LessonFile>,
}

#[derive(Debug, Deserialize)]
struct LessonProject {
    notebook: String,
    script: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonFile {
    id: String,
    title: String,
    path: String,
    url: String,
    bytes: u64,
    sha256: String,
    media_type: String,
    source: String,
    citation: String,
    rights: String,
    role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedFile {
    id: String,
    path: String,
    bytes: u64,
    sha256: String,
    media_type: String,
    reused: bool,
}

pub struct PreparedProject {
    pub root: PathBuf,
    pub script: PathBuf,
    pub inputs: Vec<PathBuf>,
}

fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_file(path: &Path) -> Result<(u64, String), String> {
    let mut input =
        File::open(path).map_err(|error| format!("cannot open '{}': {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| format!("cannot read '{}': {error}", path.display()))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        hasher.update(&buffer[..count]);
    }
    Ok((total, digest_hex(hasher.finalize())))
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_url(value: &str) -> bool {
    value.starts_with("https://")
        || value.starts_with("http://127.0.0.1:")
        || value.starts_with("http://localhost:")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn load_lock(path: &Path) -> Result<(LessonDataLock, PathBuf), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot open lesson data lock '{}': {error}", path.display()))?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        format!(
            "cannot inspect lesson data lock '{}': {error}",
            canonical.display()
        )
    })?;
    if !metadata.is_file() || metadata.len() > MAX_LOCK_BYTES {
        return Err("lesson-data.json must be a regular JSON file no larger than 16 MB".to_owned());
    }
    let lock: LessonDataLock = serde_json::from_slice(
        &fs::read(&canonical)
            .map_err(|error| format!("cannot read '{}': {error}", canonical.display()))?,
    )
    .map_err(|error| {
        format!(
            "invalid lesson data lock '{}': {error}",
            canonical.display()
        )
    })?;
    if lock.schema != 1
        || lock.kind != "biolang-lesson-data-lock"
        || !safe_relative_path(&lock.project.notebook)
        || !lock.project.notebook.ends_with(".bln")
        || !safe_relative_path(&lock.project.script)
        || !lock.project.script.ends_with(".bl")
    {
        return Err(
            "lesson data lock has an unsupported schema or unsafe project paths".to_owned(),
        );
    }
    let mut ids = std::collections::HashSet::new();
    let mut paths = std::collections::HashSet::new();
    let reserved = [
        lock.project.notebook.to_ascii_lowercase(),
        lock.project.script.to_ascii_lowercase(),
        canonical
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase(),
        "readme.md".to_owned(),
        "provenance.md".to_owned(),
    ];
    for file in &lock.files {
        if file.id.is_empty()
            || !ids.insert(&file.id)
            || !paths.insert(&file.path)
            || !safe_relative_path(&file.path)
            || file.path.contains('\\')
            || reserved.contains(&file.path.replace('\\', "/").to_ascii_lowercase())
            || !valid_url(&file.url)
            || file.bytes == 0
            || file.bytes > MAX_FILE_BYTES
            || !valid_sha256(&file.sha256)
            || file.role != "input"
            || file.title.is_empty()
            || file.media_type.is_empty()
            || file.source.is_empty()
            || file.citation.is_empty()
            || file.rights.is_empty()
        {
            return Err(format!(
                "lesson input '{}' is unsafe or incomplete",
                file.id
            ));
        }
    }
    let root = canonical
        .parent()
        .ok_or_else(|| "lesson data lock has no project directory".to_owned())?
        .to_path_buf();
    Ok((lock, root))
}

fn confined_destination(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let destination = root.join(relative);
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid lesson path '{relative}'"))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create '{}': {error}", parent.display()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot inspect '{}': {error}", parent.display()))?;
    if !canonical_parent.starts_with(root) {
        return Err(format!(
            "lesson path '{relative}' escapes the project directory"
        ));
    }
    if let Ok(metadata) = fs::symlink_metadata(&destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!("lesson path '{relative}' is not a regular file"));
        }
        let canonical = fs::canonicalize(&destination)
            .map_err(|error| format!("cannot inspect '{}': {error}", destination.display()))?;
        if !canonical.starts_with(root) {
            return Err(format!(
                "lesson path '{relative}' escapes the project directory"
            ));
        }
    }
    Ok(destination)
}

fn temporary_file(destination: &Path) -> Result<(PathBuf, File), String> {
    let original = destination
        .file_name()
        .ok_or_else(|| "invalid lesson destination".to_owned())?
        .to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..100u8 {
        let path = destination.with_file_name(format!(
            "{original}.part-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("cannot create '{}': {error}", path.display())),
        }
    }
    Err("could not allocate a temporary lesson data file".to_owned())
}

fn download(destination: &Path, file: &LessonFile) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(120))
        .redirects(0)
        .user_agent(concat!("BioLang/", env!("CARGO_PKG_VERSION")))
        .build();
    let response = agent
        .get(&file.url)
        .call()
        .map_err(|error| format!("download of '{}' failed: {error}", file.title))?;
    if !(200..300).contains(&response.status()) {
        return Err(format!(
            "download of '{}' returned HTTP {}",
            file.title,
            response.status()
        ));
    }
    if let Some(length) = response.header("Content-Length") {
        if length
            .parse::<u64>()
            .ok()
            .is_some_and(|length| length != file.bytes)
        {
            return Err(format!(
                "{} declared {} bytes but the server announced {length}",
                file.title, file.bytes
            ));
        }
    }
    let (temporary, mut output) = temporary_file(destination)?;
    let result = (|| {
        let mut input = response.into_reader();
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        let mut buffer = [0u8; 1024 * 1024];
        loop {
            let count = input
                .read(&mut buffer)
                .map_err(|error| format!("download of '{}' failed: {error}", file.title))?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            if bytes > file.bytes {
                return Err(format!("{} exceeded its declared size", file.title));
            }
            hasher.update(&buffer[..count]);
            output
                .write_all(&buffer[..count])
                .map_err(|error| format!("cannot write '{}': {error}", temporary.display()))?;
        }
        output
            .sync_all()
            .map_err(|error| format!("cannot sync '{}': {error}", temporary.display()))?;
        let digest = digest_hex(hasher.finalize());
        if bytes != file.bytes {
            return Err(format!(
                "{} expected {} bytes, received {bytes}",
                file.title, file.bytes
            ));
        }
        if !digest.eq_ignore_ascii_case(&file.sha256) {
            return Err(format!("{} failed SHA-256 verification", file.title));
        }
        drop(output);
        fs::rename(&temporary, destination).map_err(|error| {
            format!(
                "cannot atomically activate '{}': {error}",
                destination.display()
            )
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn prepare(
    lock_path: &Path,
    force: bool,
    offline: bool,
    json: bool,
    quiet: bool,
) -> Result<PreparedProject, String> {
    let (lock, root) = load_lock(lock_path)?;
    let script = confined_destination(&root, &lock.project.script)?;
    if !script.is_file() {
        return Err(format!(
            "generated lesson script '{}' is missing",
            script.display()
        ));
    }
    let notebook = confined_destination(&root, &lock.project.notebook)?;
    if !notebook.is_file() {
        return Err(format!(
            "lesson notebook '{}' is missing",
            notebook.display()
        ));
    }
    let mut prepared = Vec::new();
    let mut inputs = Vec::new();
    for file in &lock.files {
        let destination = confined_destination(&root, &file.path)?;
        let existing = if destination.is_file() {
            Some(digest_file(&destination)?)
        } else {
            None
        };
        let reused = existing.as_ref().is_some_and(|(bytes, digest)| {
            *bytes == file.bytes && digest.eq_ignore_ascii_case(&file.sha256)
        });
        if !reused {
            if offline {
                return Err(format!(
                    "'{}' is missing or failed verification; rerun without --offline",
                    file.path
                ));
            }
            if existing.is_some() && !force {
                return Err(format!("'{}' already exists with different content; review it and rerun with --force to replace it", file.path));
            }
            if !json && !quiet {
                eprintln!("Fetching {} ({} bytes)...", file.title, file.bytes);
            }
            download(&destination, file)?;
        }
        prepared.push(PreparedFile {
            id: file.id.clone(),
            path: destination.to_string_lossy().into_owned(),
            bytes: file.bytes,
            sha256: file.sha256.to_lowercase(),
            media_type: file.media_type.clone(),
            reused,
        });
        inputs.push(destination);
    }
    if quiet {
        // A lesson run must not mix preparation chatter into either normal
        // program output or the JSON Lines execution protocol.
    } else if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&prepared).map_err(|error| error.to_string())?
        );
    } else if prepared.is_empty() {
        println!("This lesson declares no managed data files.");
    } else {
        for file in &prepared {
            println!(
                "{} {}\n  {}",
                if file.reused { "Verified" } else { "Prepared" },
                file.id,
                file.path
            );
        }
    }
    Ok(PreparedProject {
        root,
        script,
        inputs,
    })
}

#[cfg(test)]
mod tests {
    use super::{digest_hex, load_lock, prepare, safe_relative_path};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn rejects_paths_that_can_escape_a_lesson_project() {
        assert!(!safe_relative_path("../secret.csv"));
        assert!(!safe_relative_path("data/../../secret.csv"));
        assert!(!safe_relative_path("/absolute.csv"));
        assert!(safe_relative_path("data/example.csv"));
    }

    #[test]
    fn loads_a_safe_empty_lock() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lesson-data.json");
        fs::write(directory.path().join("lesson.bl"), "1 + 1\n").unwrap();
        fs::write(
            directory.path().join("lesson.bln"),
            "```biolang\n1 + 1\n```\n",
        )
        .unwrap();
        fs::write(&path, r#"{"schema":1,"kind":"biolang-lesson-data-lock","project":{"notebook":"lesson.bln","script":"lesson.bl"},"files":[]}"#).unwrap();
        let (lock, root) = load_lock(&path).unwrap();
        assert_eq!(lock.project.script, "lesson.bl");
        assert_eq!(root, fs::canonicalize(directory.path()).unwrap());
    }

    #[test]
    fn rejects_inputs_that_can_replace_project_code() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lesson-data.json");
        fs::write(
            &path,
            format!(
                r#"{{"schema":1,"kind":"biolang-lesson-data-lock","project":{{"notebook":"lesson.bln","script":"lesson.bl"}},"files":[{{"id":"code","title":"Code","path":"lesson.bl","url":"https://example.test/code","bytes":1,"sha256":"{}","mediaType":"text/plain","source":"Fixture","citation":"Fixture","rights":"CC0","role":"input"}}]}}"#,
                "a".repeat(64)
            ),
        )
        .unwrap();
        assert!(load_lock(&path).is_err());
    }

    #[test]
    fn downloads_verifies_and_reuses_a_declared_input_offline() {
        let bytes = b"name,value\nA,1\n";
        let digest = digest_hex(Sha256::digest(bytes));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(bytes).unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let lock_path = directory.path().join("lesson-data.json");
        fs::write(directory.path().join("lesson.bl"), "1 + 1\n").unwrap();
        fs::write(
            directory.path().join("lesson.bln"),
            "```biolang\n1 + 1\n```\n",
        )
        .unwrap();
        fs::write(
            &lock_path,
            format!(
                r#"{{"schema":1,"kind":"biolang-lesson-data-lock","project":{{"notebook":"lesson.bln","script":"lesson.bl"}},"files":[{{"id":"table","title":"Table","path":"data/table.csv","url":"http://127.0.0.1:{}/table.csv","bytes":{},"sha256":"{}","mediaType":"text/csv","source":"Fixture","citation":"Fixture citation","rights":"CC0","role":"input"}}]}}"#,
                address.port(), bytes.len(), digest
            ),
        )
        .unwrap();

        let first = prepare(&lock_path, false, false, false, true).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(&first.inputs[0]).unwrap(), bytes);
        let second = prepare(&lock_path, false, true, false, true).unwrap();
        assert_eq!(second.inputs, first.inputs);
    }

    #[test]
    fn force_replaces_a_changed_input_only_after_verification() {
        let bytes = b"verified replacement\n";
        let digest = digest_hex(Sha256::digest(bytes));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(bytes).unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let lock_path = directory.path().join("lesson-data.json");
        fs::write(directory.path().join("lesson.bl"), "1 + 1\n").unwrap();
        fs::write(
            directory.path().join("lesson.bln"),
            "```biolang\n1 + 1\n```\n",
        )
        .unwrap();
        fs::create_dir(directory.path().join("data")).unwrap();
        fs::write(
            directory.path().join("data/table.csv"),
            b"changed locally\n",
        )
        .unwrap();
        fs::write(
            &lock_path,
            format!(
                r#"{{"schema":1,"kind":"biolang-lesson-data-lock","project":{{"notebook":"lesson.bln","script":"lesson.bl"}},"files":[{{"id":"table","title":"Table","path":"data/table.csv","url":"http://127.0.0.1:{}/table.csv","bytes":{},"sha256":"{}","mediaType":"text/plain","source":"Fixture","citation":"Fixture citation","rights":"CC0","role":"input"}}]}}"#,
                address.port(), bytes.len(), digest
            ),
        )
        .unwrap();
        assert!(prepare(&lock_path, false, false, false, true).is_err());
        let prepared = prepare(&lock_path, true, false, false, true).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(&prepared.inputs[0]).unwrap(), bytes);
    }
}
