use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const CACHED_MANIFEST_FILE: &str = ".biolang-dataset-manifest.json";

pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/oriclabs/biolang-registry/main/registry/v1/index.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegistryIndex {
    pub schema: u8,
    pub entries: Vec<RegistryEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub schema: u8,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub title: String,
    pub summary: String,
    pub publisher: String,
    pub version: String,
    pub status: String,
    pub verified: bool,
    pub manifest: String,
    pub manifest_sha256: String,
    pub published_at: String,
    pub compatibility: RegistryCompatibility,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub source_repository: String,
    pub licence: String,
    pub validation: String,
    pub dataset: Option<DatasetDiscovery>,
    pub provider: Option<ProviderDiscovery>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegistryCompatibility {
    pub biolang: Option<String>,
    pub studio: Option<String>,
    pub runtimes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetDiscovery {
    pub provider: String,
    pub access: String,
    pub formats: Vec<String>,
    pub modalities: Vec<String>,
    pub organisms: Vec<String>,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiscovery {
    pub adapter: String,
    pub authentication: String,
    pub capabilities: Vec<String>,
    pub api_documentation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatasetManifest {
    pub schema: u8,
    pub kind: String,
    pub id: String,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub modalities: Vec<String>,
    pub organisms: Vec<String>,
    pub provider: String,
    pub access: DatasetAccess,
    pub source: DatasetSource,
    pub files: Vec<DatasetFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetAccess {
    pub kind: String,
    pub requires_acceptance: bool,
    pub terms_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSource {
    pub accession: Option<String>,
    pub landing_page: String,
    pub citation: String,
    pub licence: String,
    pub rights: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetFile {
    pub id: String,
    pub title: String,
    pub path: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub media_type: String,
    pub format: String,
    pub compression: Option<String>,
    pub role: String,
    pub reader: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchResult {
    id: String,
    version: String,
    provider: String,
    files: Vec<CachedFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedFile {
    id: String,
    path: String,
    bytes: u64,
    sha256: String,
    reader: String,
    reused: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedManifest {
    schema: u8,
    manifest_sha256: String,
    manifest_json: String,
}

struct VerifiedManifest {
    manifest: DatasetManifest,
    manifest_sha256: String,
    manifest_json: String,
}

fn registry_url(override_url: Option<&str>) -> String {
    override_url
        .map(str::to_owned)
        .or_else(|| env::var("BIOLANG_REGISTRY_URL").ok())
        .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_owned())
}

fn http_get(url: &str) -> Result<ureq::Response, String> {
    if !url.starts_with("https://")
        && !url.starts_with("http://127.0.0.1:")
        && !url.starts_with("http://localhost:")
    {
        return Err(format!("registry downloads require HTTPS: {url}"));
    }
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .redirects(0)
        .user_agent(concat!("BioLang/", env!("CARGO_PKG_VERSION")))
        .build()
        .get(url)
        .call()
        .map_err(|error| format!("GET {url} failed: {error}"))
        .and_then(|response| {
            if (300..400).contains(&response.status()) {
                Err(format!(
                    "GET {url} refused an unverified redirect (HTTP {})",
                    response.status()
                ))
            } else {
                Ok(response)
            }
        })
}

fn response_bytes(response: ureq::Response, limit: u64) -> Result<Vec<u8>, String> {
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| format!("download failed: {error}"))?;
        if count == 0 {
            break;
        }
        if bytes.len() as u64 + count as u64 > limit {
            return Err(format!("response exceeds the {limit}-byte safety limit"));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

fn digest_hex(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fetch_index(override_url: Option<&str>) -> Result<RegistryIndex, String> {
    let url = registry_url(override_url);
    let bytes = response_bytes(http_get(&url)?, 32 * 1024 * 1024)?;
    decode_index(&bytes)
}

fn decode_index(bytes: &[u8]) -> Result<RegistryIndex, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid registry JSON: {error}"))?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "invalid registry JSON: missing integer field 'schema'".to_owned())?;
    if schema != 1 {
        return Err(format!("unsupported registry schema {schema}"));
    }
    let raw_entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "invalid registry JSON: field 'entries' must be an array".to_owned())?;
    let mut entries = Vec::with_capacity(raw_entries.len());
    for (index, raw) in raw_entries.iter().enumerate() {
        let decoded = serde_json::from_value::<RegistryEntry>(raw.clone())
            .map_err(|error| format!("invalid entry JSON: {error}"))
            .and_then(|entry| {
                validate_entry(&entry)?;
                Ok(entry)
            });
        match decoded {
            Ok(entry) => entries.push(entry),
            Err(error) => eprintln!("Warning: skipped registry entry {}: {error}", index + 1),
        }
    }
    let index = RegistryIndex { schema: 1, entries };
    validate_index(&index)?;
    Ok(index)
}

fn validate_index(index: &RegistryIndex) -> Result<(), String> {
    if index.schema != 1 {
        return Err(format!("unsupported registry schema {}", index.schema));
    }
    for entry in &index.entries {
        validate_entry(entry)?;
    }
    Ok(())
}

fn validate_entry(entry: &RegistryEntry) -> Result<(), String> {
    if entry.schema != 1
        || !matches!(
            entry.kind.as_str(),
            "lesson" | "package" | "workflow" | "tool" | "dataset" | "provider"
        )
        || !valid_id(&entry.id)
        || !valid_piece(&entry.publisher)
        || !valid_piece(&entry.name)
        || !valid_version(&entry.version)
        || entry.id != format!("{}/{}", entry.publisher, entry.name)
        || !entry.manifest.starts_with("https://")
        || !valid_sha256(&entry.manifest_sha256)
        || entry.compatibility.runtimes.is_empty()
        || entry.categories.is_empty()
    {
        return Err(format!("registry entry '{}' is invalid", entry.id));
    }
    if entry.kind == "dataset" && entry.dataset.is_none() {
        return Err(format!("dataset '{}' lacks discovery metadata", entry.id));
    }
    if entry.kind == "provider" && entry.provider.is_none() {
        return Err(format!("provider '{}' lacks discovery metadata", entry.id));
    }
    Ok(())
}

fn valid_piece(piece: &str) -> bool {
    let mut bytes = piece.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn valid_id(value: &str) -> bool {
    let mut pieces = value.split('/');
    matches!((pieces.next(), pieces.next(), pieces.next()), (Some(a), Some(b), None) if valid_piece(a) && valid_piece(b))
}

fn valid_version(value: &str) -> bool {
    let (base, suffix_is_valid) = match value.split_once('-') {
        Some((base, suffix)) => (
            base,
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-'),
        ),
        None => (value, true),
    };
    let parts = base.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u64>().is_ok())
        && suffix_is_valid
        && !value.contains('/')
        && !value.contains('\\')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn split_spec(spec: &str) -> Result<(&str, Option<&str>), String> {
    let (id, version) = spec
        .rsplit_once('@')
        .map(|(id, version)| (id, Some(version)))
        .unwrap_or((spec, None));
    if !valid_id(id) || version.is_some_and(|value| !valid_version(value)) {
        return Err(format!(
            "invalid dataset id '{spec}'; expected publisher/name or publisher/name@version"
        ));
    }
    Ok((id, version))
}

fn version_key(value: &str) -> (u64, u64, u64, bool, &str) {
    let (base, suffix) = value.split_once('-').unwrap_or((value, ""));
    let mut parts = base.split('.').map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        suffix.is_empty(),
        suffix,
    )
}

fn select_entry<'a>(
    index: &'a RegistryIndex,
    spec: &str,
    expected_kind: &str,
) -> Result<&'a RegistryEntry, String> {
    let (id, version) = split_spec(spec)?;
    index
        .entries
        .iter()
        .filter(|entry| {
            entry.id == id
                && entry.kind == expected_kind
                && entry.status != "withdrawn"
                && version.map_or(true, |wanted| entry.version == wanted)
        })
        .max_by_key(|entry| version_key(&entry.version))
        .ok_or_else(|| format!("{expected_kind} '{spec}' was not found in the registry"))
}

fn fetch_manifest(entry: &RegistryEntry) -> Result<VerifiedManifest, String> {
    let bytes = response_bytes(http_get(&entry.manifest)?, 8 * 1024 * 1024)?;
    let actual = sha256(&bytes);
    if actual != entry.manifest_sha256 {
        return Err(format!(
            "dataset manifest failed SHA-256 verification: expected {}, received {actual}",
            entry.manifest_sha256
        ));
    }
    let manifest: DatasetManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid dataset manifest JSON: {error}"))?;
    validate_manifest(entry, &manifest)?;
    let manifest_json = String::from_utf8(bytes)
        .map_err(|error| format!("dataset manifest is not UTF-8 JSON: {error}"))?;
    Ok(VerifiedManifest {
        manifest,
        manifest_sha256: actual,
        manifest_json,
    })
}

fn validate_manifest(entry: &RegistryEntry, manifest: &DatasetManifest) -> Result<(), String> {
    if manifest.schema != 1
        || manifest.kind != "dataset"
        || manifest.id != entry.id
        || manifest.version != entry.version
    {
        return Err(format!(
            "dataset manifest identity does not match {}@{}",
            entry.id, entry.version
        ));
    }
    let discovery = entry
        .dataset
        .as_ref()
        .ok_or_else(|| "registry entry lacks dataset metadata".to_owned())?;
    let total = manifest
        .files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.bytes))
        .ok_or_else(|| "dataset size overflows u64".to_owned())?;
    if manifest.provider != discovery.provider
        || manifest.access.kind != discovery.access
        || manifest.files.len() as u64 != discovery.file_count
        || total != discovery.total_bytes
    {
        return Err("dataset manifest does not match registry discovery metadata".to_owned());
    }
    validate_manifest_contents(manifest)
}

fn validate_manifest_contents(manifest: &DatasetManifest) -> Result<(), String> {
    if manifest.schema != 1
        || manifest.kind != "dataset"
        || !valid_id(&manifest.id)
        || !valid_version(&manifest.version)
        || !valid_id(&manifest.provider)
        || manifest.files.is_empty()
    {
        return Err("dataset manifest has an invalid identity or version".to_owned());
    }
    let mut ids = std::collections::HashSet::new();
    let mut paths = std::collections::HashSet::new();
    for file in &manifest.files {
        if !ids.insert(&file.id)
            || !paths.insert(&file.path)
            || !safe_relative_path(&file.path)
            || !file.url.starts_with("https://")
            || !valid_sha256(&file.sha256)
            || file.bytes == 0
        {
            return Err(format!("dataset file '{}' is unsafe or invalid", file.id));
        }
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn data_home() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("BIOLANG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".biolang").join("data"))
        .ok_or_else(|| "cannot determine data cache; set BIOLANG_DATA_HOME".to_owned())
}

fn manifest_root(manifest: &DatasetManifest) -> Result<PathBuf, String> {
    if !valid_id(&manifest.id) || !valid_version(&manifest.version) {
        return Err("invalid dataset cache identity or version".to_owned());
    }
    let (publisher, name) = manifest
        .id
        .split_once('/')
        .ok_or_else(|| "invalid dataset id".to_owned())?;
    Ok(data_home()?
        .join(publisher)
        .join(name)
        .join(&manifest.version))
}

fn cache_base(id: &str) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err("invalid dataset cache identity".to_owned());
    }
    let (publisher, name) = id
        .split_once('/')
        .ok_or_else(|| "invalid dataset id".to_owned())?;
    Ok(data_home()?.join(publisher).join(name))
}

fn temporary_path(destination: &Path) -> Result<PathBuf, String> {
    let mut name = destination
        .file_name()
        .ok_or_else(|| format!("invalid destination '{}'", destination.display()))?
        .to_os_string();
    name.push(format!(".part-{}", std::process::id()));
    Ok(destination.with_file_name(name))
}

fn cache_verified_manifest(verified: &VerifiedManifest) -> Result<(), String> {
    let root = manifest_root(&verified.manifest)?;
    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot create '{}': {error}", root.display()))?;
    let destination = root.join(CACHED_MANIFEST_FILE);
    let cached = CachedManifest {
        schema: 1,
        manifest_sha256: verified.manifest_sha256.clone(),
        manifest_json: verified.manifest_json.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&cached)
        .map_err(|error| format!("cannot encode cached dataset manifest: {error}"))?;

    if destination.exists() {
        let existing = load_cached_manifest_at(
            &destination,
            &verified.manifest.id,
            &verified.manifest.version,
        )?;
        let existing_json = serde_json::to_vec(&existing)
            .map_err(|error| format!("cannot compare cached dataset manifest: {error}"))?;
        let wanted_json = serde_json::to_vec(&verified.manifest)
            .map_err(|error| format!("cannot compare dataset manifest: {error}"))?;
        if existing_json == wanted_json {
            return Ok(());
        }
        return Err(format!(
            "cached manifest '{}' conflicts with the verified registry manifest",
            destination.display()
        ));
    }

    let temporary = temporary_path(&destination)?;
    let result = (|| {
        let mut output = File::create(&temporary)
            .map_err(|error| format!("cannot create '{}': {error}", temporary.display()))?;
        output
            .write_all(&bytes)
            .map_err(|error| format!("cannot write '{}': {error}", temporary.display()))?;
        output
            .sync_all()
            .map_err(|error| format!("cannot sync '{}': {error}", temporary.display()))?;
        fs::rename(&temporary, &destination)
            .map_err(|error| format!("cannot activate '{}': {error}", destination.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn load_cached_manifest_at(
    path: &Path,
    expected_id: &str,
    expected_version: &str,
) -> Result<DatasetManifest, String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "cannot inspect cached manifest '{}': {error}",
            path.display()
        )
    })?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err(format!("cached manifest '{}' is too large", path.display()));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read cached manifest '{}': {error}", path.display()))?;
    let cached: CachedManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid cached manifest '{}': {error}", path.display()))?;
    if cached.schema != 1 || !valid_sha256(&cached.manifest_sha256) {
        return Err(format!(
            "cached manifest '{}' has an unsupported format",
            path.display()
        ));
    }
    let actual = sha256(cached.manifest_json.as_bytes());
    if actual != cached.manifest_sha256 {
        return Err(format!(
            "cached manifest '{}' failed SHA-256 verification",
            path.display()
        ));
    }
    let manifest: DatasetManifest = serde_json::from_str(&cached.manifest_json)
        .map_err(|error| format!("invalid dataset manifest in '{}': {error}", path.display()))?;
    validate_manifest_contents(&manifest)?;
    if manifest.id != expected_id || manifest.version != expected_version {
        return Err(format!(
            "cached manifest '{}' has the wrong identity",
            path.display()
        ));
    }
    Ok(manifest)
}

fn load_cached_manifest(spec: &str) -> Result<Option<DatasetManifest>, String> {
    let (id, wanted_version) = split_spec(spec)?;
    let base = cache_base(id)?;
    if let Some(version) = wanted_version {
        let path = base.join(version).join(CACHED_MANIFEST_FILE);
        return if path.exists() {
            load_cached_manifest_at(&path, id, version).map(Some)
        } else {
            Ok(None)
        };
    }
    if !base.is_dir() {
        return Ok(None);
    }
    let mut manifests = Vec::new();
    for entry in fs::read_dir(&base)
        .map_err(|error| format!("cannot inspect data cache '{}': {error}", base.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot inspect data cache: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect cache entry: {error}"))?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let version = entry.file_name().to_string_lossy().into_owned();
        if !valid_version(&version) {
            continue;
        }
        let path = entry.path().join(CACHED_MANIFEST_FILE);
        if !path.is_file() {
            continue;
        }
        match load_cached_manifest_at(&path, id, &version) {
            Ok(manifest) => manifests.push(manifest),
            Err(error) => eprintln!("Warning: skipped {error}"),
        }
    }
    Ok(manifests
        .into_iter()
        .max_by(|left, right| version_key(&left.version).cmp(&version_key(&right.version))))
}

fn resolve_manifest(spec: &str, override_url: Option<&str>) -> Result<DatasetManifest, String> {
    if let Some(manifest) = load_cached_manifest(spec)? {
        return Ok(manifest);
    }
    let index = fetch_index(override_url)?;
    let entry = select_entry(&index, spec, "dataset")?;
    let verified = fetch_manifest(entry)?;
    cache_verified_manifest(&verified)?;
    Ok(verified.manifest)
}

fn digest_file(path: &Path) -> Result<(u64, String), String> {
    let mut file =
        File::open(path).map_err(|error| format!("cannot open '{}': {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    let mut bytes = 0u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read '{}': {error}", path.display()))?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        hasher.update(&buffer[..count]);
    }
    Ok((bytes, digest_hex(hasher.finalize())))
}

fn download_file(root: &Path, file: &DatasetFile, force: bool) -> Result<CachedFile, String> {
    let destination = root.join(&file.path);
    if destination.exists() && !force {
        let (bytes, digest) = digest_file(&destination)?;
        if bytes == file.bytes && digest == file.sha256 {
            return Ok(CachedFile {
                id: file.id.clone(),
                path: destination.to_string_lossy().into_owned(),
                bytes,
                sha256: digest,
                reader: file.reader.clone(),
                reused: true,
            });
        }
    }
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid cache path '{}',", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create '{}': {error}", parent.display()))?;
    let temporary = temporary_path(&destination)?;
    let result = (|| {
        let response = http_get(&file.url)?;
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
        let mut input = response.into_reader();
        let mut output = File::create(&temporary)
            .map_err(|error| format!("cannot create '{}': {error}", temporary.display()))?;
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
        if digest != file.sha256 {
            return Err(format!(
                "{} failed SHA-256 verification: expected {}, received {digest}",
                file.title, file.sha256
            ));
        }
        fs::rename(&temporary, &destination)
            .map_err(|error| format!("cannot activate '{}': {error}", destination.display()))?;
        Ok(CachedFile {
            id: file.id.clone(),
            path: destination.to_string_lossy().into_owned(),
            bytes,
            sha256: digest,
            reader: file.reader.clone(),
            reused: false,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn selected_files<'a>(
    manifest: &'a DatasetManifest,
    file_id: Option<&str>,
) -> Result<Vec<&'a DatasetFile>, String> {
    match file_id {
        Some(id) => manifest
            .files
            .iter()
            .find(|file| file.id == id)
            .map(|file| vec![file])
            .ok_or_else(|| format!("dataset has no file named '{id}'")),
        None => Ok(manifest.files.iter().collect()),
    }
}

fn sort_registry_entries(entries: &mut [RegistryEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| version_key(&right.version).cmp(&version_key(&left.version)))
    });
}

pub fn search(
    query: Option<&str>,
    category: Option<&str>,
    kind: &str,
    json: bool,
    override_url: Option<&str>,
) -> Result<(), String> {
    let index = fetch_index(override_url)?;
    let terms = query
        .unwrap_or("")
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut entries = index
        .entries
        .into_iter()
        .filter(|entry| entry.status != "withdrawn")
        .filter(|entry| kind == "all" || entry.kind == kind)
        .filter(|entry| {
            category.map_or(true, |wanted| {
                entry.categories.iter().any(|value| value == wanted)
            })
        })
        .filter(|entry| {
            let dataset_text = entry
                .dataset
                .as_ref()
                .map(|data| {
                    format!(
                        "{} {} {} {}",
                        data.provider,
                        data.formats.join(" "),
                        data.modalities.join(" "),
                        data.organisms.join(" ")
                    )
                })
                .unwrap_or_default();
            let provider_text = entry
                .provider
                .as_ref()
                .map(|provider| {
                    format!(
                        "{} {} {}",
                        provider.adapter,
                        provider.authentication,
                        provider.capabilities.join(" ")
                    )
                })
                .unwrap_or_default();
            let haystack = format!(
                "{} {} {} {} {} {} {}",
                entry.id,
                entry.title,
                entry.summary,
                entry.categories.join(" "),
                entry.tags.join(" "),
                dataset_text,
                provider_text
            )
            .to_lowercase();
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect::<Vec<_>>();
    sort_registry_entries(&mut entries);
    if json {
        println!("{}", serde_json::to_string_pretty(&entries).unwrap());
    } else if entries.is_empty() {
        println!("No registry entries matched.");
    } else {
        for entry in entries {
            let detail = entry
                .dataset
                .as_ref()
                .map(|data| {
                    format!(
                        "{} · {} · {}",
                        display_bytes(data.total_bytes),
                        data.formats.join(","),
                        data.access
                    )
                })
                .or_else(|| {
                    entry.provider.as_ref().map(|provider| {
                        format!("adapter {} · {}", provider.adapter, provider.authentication)
                    })
                })
                .unwrap_or_else(|| entry.licence.clone());
            println!(
                "{}@{}  [{} / {}]",
                entry.id, entry.version, entry.kind, entry.status
            );
            println!("  {}", entry.title);
            println!("  {}", entry.summary);
            println!("  {detail}\n");
        }
    }
    Ok(())
}

pub fn info(spec: &str, json: bool, override_url: Option<&str>) -> Result<(), String> {
    let manifest = resolve_manifest(spec, override_url)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
        return Ok(());
    }
    println!("{}@{}", manifest.id, manifest.version);
    println!("{}\n", manifest.title);
    println!("{}\n", manifest.description);
    println!("Provider: {}", manifest.provider);
    println!("Access:   {}", manifest.access.kind);
    println!("Licence:  {}", manifest.source.licence);
    println!("Source:   {}", manifest.source.landing_page);
    println!("Citation: {}\n", manifest.source.citation);
    for file in &manifest.files {
        println!(
            "{}  {}  {}",
            file.id,
            display_bytes(file.bytes),
            file.format
        );
        println!("  {}", file.title);
        println!("  after fetch: {}(\"{}\")", file.reader, file.path);
    }
    Ok(())
}

pub fn fetch(
    spec: &str,
    file_id: Option<&str>,
    accept_terms: bool,
    force: bool,
    json: bool,
    override_url: Option<&str>,
) -> Result<(), String> {
    let manifest = resolve_manifest(spec, override_url)?;
    if manifest.access.requires_acceptance && !accept_terms {
        return Err(format!(
            "{} requires acceptance of its source terms{}; review them and rerun with --accept-terms",
            manifest.id,
            manifest
                .access
                .terms_url
                .as_deref()
                .map(|url| format!(" at {url}"))
                .unwrap_or_default()
        ));
    }
    if manifest.provider != "oriclabs/direct-https" {
        return Err(format!(
            "provider '{}' needs a BioLang adapter that is not installed",
            manifest.provider
        ));
    }
    let root = manifest_root(&manifest)?;
    let mut cached = Vec::new();
    for file in selected_files(&manifest, file_id)? {
        if !json {
            eprintln!("Fetching {} ({})...", file.title, display_bytes(file.bytes));
        }
        cached.push(download_file(&root, file, force)?);
    }
    let result = FetchResult {
        id: manifest.id,
        version: manifest.version,
        provider: manifest.provider,
        files: cached,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        for file in &result.files {
            println!(
                "{} {}\n  {}\n  BioLang: {}(\"{}\")",
                if file.reused { "Reused" } else { "Verified" },
                file.id,
                file.path,
                file.reader,
                file.path.replace('\\', "\\\\")
            );
        }
    }
    Ok(())
}

pub fn path(spec: &str, file_id: Option<&str>, json: bool) -> Result<(), String> {
    let manifest = load_cached_manifest(spec)?.ok_or_else(|| {
        format!("'{spec}' has no cached manifest; run `bl data fetch {spec}` while online")
    })?;
    let root = manifest_root(&manifest)?;
    let mut cached = Vec::new();
    for file in selected_files(&manifest, file_id)? {
        let destination = root.join(&file.path);
        if !destination.exists() {
            return Err(format!(
                "'{}' is not cached; run `bl data fetch {} --file {}`",
                file.id, spec, file.id
            ));
        }
        let (bytes, digest) = digest_file(&destination)?;
        if bytes != file.bytes || digest != file.sha256 {
            return Err(format!(
                "cached file '{}' failed verification; run `bl data fetch {} --file {} --force`",
                file.id, spec, file.id
            ));
        }
        cached.push(CachedFile {
            id: file.id.clone(),
            path: destination.to_string_lossy().into_owned(),
            bytes,
            sha256: digest,
            reader: file.reader.clone(),
            reused: true,
        });
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&cached).unwrap());
    } else {
        for file in cached {
            println!("{}", file.path);
        }
    }
    Ok(())
}

fn display_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cache_verified_manifest, decode_index, digest_file, download_file, load_cached_manifest_at,
        manifest_root, path as cached_path, safe_relative_path, select_entry, sha256,
        sort_registry_entries, split_spec, temporary_path, validate_index, CachedManifest,
        DatasetAccess, DatasetDiscovery, DatasetFile, DatasetManifest, DatasetSource,
        RegistryEntry, RegistryIndex, VerifiedManifest, CACHED_MANIFEST_FILE,
    };
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::thread;

    static DATA_HOME_LOCK: Mutex<()> = Mutex::new(());

    fn dataset(version: &str, status: &str) -> RegistryEntry {
        RegistryEntry {
            schema: 1,
            kind: "dataset".into(),
            id: "test/cells".into(),
            name: "cells".into(),
            title: "Cells".into(),
            summary: "Test cells".into(),
            publisher: "test".into(),
            version: version.into(),
            status: status.into(),
            verified: true,
            manifest: "https://example.test/dataset.json".into(),
            manifest_sha256: "a".repeat(64),
            published_at: "2026-08-28".into(),
            compatibility: super::RegistryCompatibility {
                biolang: Some(">=1.5.0".into()),
                studio: Some(">=0.1.0".into()),
                runtimes: vec!["cli".into()],
            },
            categories: vec!["single-cell".into()],
            tags: vec!["RNA".into()],
            source_repository: "https://example.test/source".into(),
            licence: "CC0".into(),
            validation: "registry-verified".into(),
            dataset: Some(DatasetDiscovery {
                provider: "test/direct".into(),
                access: "public".into(),
                formats: vec!["csv".into()],
                modalities: vec!["RNA".into()],
                organisms: vec!["Homo sapiens".into()],
                file_count: 1,
                total_bytes: 10,
            }),
            provider: None,
        }
    }

    fn manifest(version: &str) -> DatasetManifest {
        DatasetManifest {
            schema: 1,
            kind: "dataset".into(),
            id: "test/cells".into(),
            version: version.into(),
            title: "Cells".into(),
            summary: "Test cells".into(),
            description: "A test dataset".into(),
            categories: vec!["single-cell".into()],
            tags: vec!["RNA".into()],
            modalities: vec!["RNA".into()],
            organisms: vec!["Homo sapiens".into()],
            provider: "test/direct".into(),
            access: DatasetAccess {
                kind: "public".into(),
                requires_acceptance: false,
                terms_url: None,
            },
            source: DatasetSource {
                accession: None,
                landing_page: "https://example.test/data".into(),
                citation: "Test data".into(),
                licence: "CC0".into(),
                rights: "Open".into(),
            },
            files: vec![DatasetFile {
                id: "matrix".into(),
                title: "Matrix".into(),
                path: "matrix.csv".into(),
                url: "https://example.test/matrix.csv".into(),
                bytes: 5,
                sha256: "a".repeat(64),
                media_type: "text/csv".into(),
                format: "csv".into(),
                compression: None,
                role: "primary".into(),
                reader: "read_csv".into(),
            }],
        }
    }

    #[test]
    fn rejects_paths_that_can_escape_the_dataset_cache() {
        assert!(safe_relative_path("matrix/data.csv"));
        assert!(!safe_relative_path("../data.csv"));
        assert!(!safe_relative_path("/data.csv"));
        assert!(!safe_relative_path("matrix/../data.csv"));
        assert!(split_spec("oriclabs/cells@1.0.0").is_ok());
        assert!(split_spec("../../evil@1.0.0").is_err());
        assert!(split_spec("../evil@1.0.0").is_err());
        assert!(split_spec("oriclabs/cells@../../evil").is_err());
        assert!(split_spec("oriclabs/cells@1.2.3-").is_err());
        assert!(split_spec("oriclabs/cells@18446744073709551616.0.0").is_err());
        let mut unsafe_manifest = manifest("1.0.0");
        unsafe_manifest.version = "../../evil".into();
        assert!(manifest_root(&unsafe_manifest).is_err());
    }

    #[test]
    fn temporary_names_append_instead_of_replacing_extensions() {
        let gzip = temporary_path(std::path::Path::new("data.tar.gz")).unwrap();
        let zstd = temporary_path(std::path::Path::new("data.tar.zst")).unwrap();
        assert_ne!(gzip, zstd);
        assert!(gzip
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("data.tar.gz.part-"));
    }

    #[test]
    fn resolves_latest_non_withdrawn_dataset_or_exact_version() {
        let index = RegistryIndex {
            schema: 1,
            entries: vec![
                dataset("1.0.0", "stable"),
                dataset("1.2.0", "stable"),
                dataset("2.0.0", "withdrawn"),
            ],
        };
        assert_eq!(
            select_entry(&index, "test/cells", "dataset")
                .unwrap()
                .version,
            "1.2.0"
        );
        assert_eq!(
            select_entry(&index, "test/cells@1.0.0", "dataset")
                .unwrap()
                .version,
            "1.0.0"
        );
    }

    #[test]
    fn validates_dataset_discovery_metadata() {
        let index = RegistryIndex {
            schema: 1,
            entries: vec![dataset("1.0.0", "stable")],
        };
        validate_index(&index).unwrap();
        let mut broken = index.clone();
        broken.entries[0].manifest_sha256 = "bad".into();
        assert!(validate_index(&broken).is_err());
        let mut unsafe_version = index;
        unsafe_version.entries[0].version = "../../evil".into();
        assert!(validate_index(&unsafe_version).is_err());
    }

    #[test]
    fn checks_schema_before_decoding_entries_and_skips_bad_entries() {
        let future = serde_json::json!({ "schema": 2 });
        let error = decode_index(&serde_json::to_vec(&future).unwrap()).unwrap_err();
        assert_eq!(error, "unsupported registry schema 2");

        let mixed = serde_json::json!({
            "schema": 1,
            "entries": [dataset("1.0.0", "stable"), { "schema": 1, "kind": "dataset" }]
        });
        let decoded = decode_index(&serde_json::to_vec(&mixed).unwrap()).unwrap();
        assert_eq!(decoded.entries.len(), 1);
    }

    #[test]
    fn sorts_versions_the_same_way_as_unversioned_resolution() {
        let mut entries = vec![dataset("1.9.0", "stable"), dataset("1.10.0", "stable")];
        sort_registry_entries(&mut entries);
        assert_eq!(entries[0].version, "1.10.0");
        let index = RegistryIndex { schema: 1, entries };
        assert_eq!(
            select_entry(&index, "test/cells", "dataset")
                .unwrap()
                .version,
            "1.10.0"
        );
    }

    #[test]
    fn cached_manifest_is_self_verifying() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("manifest.json");
        let expected = manifest("1.0.0");
        let manifest_json = serde_json::to_string(&expected).unwrap();
        let cached = CachedManifest {
            schema: 1,
            manifest_sha256: sha256(manifest_json.as_bytes()),
            manifest_json,
        };
        fs::write(&path, serde_json::to_vec(&cached).unwrap()).unwrap();
        let loaded = load_cached_manifest_at(&path, "test/cells", "1.0.0").unwrap();
        assert_eq!(loaded.id, "test/cells");

        let mut damaged: CachedManifest =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        damaged.manifest_json.push(' ');
        fs::write(&path, serde_json::to_vec(&damaged).unwrap()).unwrap();
        assert!(load_cached_manifest_at(&path, "test/cells", "1.0.0").is_err());
    }

    #[test]
    fn path_uses_the_verified_manifest_cache_without_a_registry_request() {
        let _guard = DATA_HOME_LOCK.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("BIOLANG_DATA_HOME");
        std::env::set_var("BIOLANG_DATA_HOME", directory.path());

        let contents = b"hello";
        let mut expected = manifest("1.0.0");
        expected.files[0].sha256 = sha256(contents);
        let manifest_json = serde_json::to_string(&expected).unwrap();
        let verified = VerifiedManifest {
            manifest: expected,
            manifest_sha256: sha256(manifest_json.as_bytes()),
            manifest_json,
        };
        cache_verified_manifest(&verified).unwrap();
        let root = directory.path().join("test").join("cells").join("1.0.0");
        fs::write(root.join("matrix.csv"), contents).unwrap();

        cached_path("test/cells@1.0.0", Some("matrix"), false).unwrap();
        assert!(root.join(CACHED_MANIFEST_FILE).is_file());

        match previous {
            Some(value) => std::env::set_var("BIOLANG_DATA_HOME", value),
            None => std::env::remove_var("BIOLANG_DATA_HOME"),
        }
    }

    fn serve_once(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        (format!("http://{address}/data"), handle)
    }

    #[test]
    fn download_is_verified_reused_and_does_not_replace_good_data_on_failure() {
        let root = tempfile::tempdir().unwrap();
        let expected = b"hello".to_vec();
        let (url, server) = serve_once(expected.clone());
        let mut file = DatasetFile {
            id: "matrix".into(),
            title: "Matrix".into(),
            path: "matrix.csv".into(),
            url,
            bytes: expected.len() as u64,
            sha256: sha256(&expected),
            media_type: "text/csv".into(),
            format: "csv".into(),
            compression: Some("none".into()),
            role: "primary".into(),
            reader: "read_csv".into(),
        };
        let first = download_file(root.path(), &file, false).unwrap();
        server.join().unwrap();
        assert!(!first.reused);
        assert!(download_file(root.path(), &file, false).unwrap().reused);

        let (bad_url, bad_server) = serve_once(b"bad!!".to_vec());
        file.url = bad_url;
        assert!(download_file(root.path(), &file, true).is_err());
        bad_server.join().unwrap();
        let (bytes, digest) = digest_file(&root.path().join("matrix.csv")).unwrap();
        assert_eq!(bytes, expected.len() as u64);
        assert_eq!(digest, file.sha256);
        assert!(!root
            .path()
            .join(format!("matrix.csv.part-{}", std::process::id()))
            .exists());
    }
}
