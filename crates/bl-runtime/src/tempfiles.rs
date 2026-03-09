//! Global temp file registry for BioLang.
//!
//! Any operation that creates temporary files on disk (e.g. disk-backed kmer counting)
//! registers them here. Files are cleaned up when:
//! - The operation completes normally (via `Drop` or explicit `unregister + delete`)
//! - The process exits normally (`cleanup_all` called from REPL/CLI shutdown)
//!
//! For hard kills (SIGKILL, power loss), files live in the OS temp dir and are
//! cleaned up by the OS or on next BioLang startup via `cleanup_stale`.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static REGISTRY: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashSet<PathBuf>) -> R,
{
    let mut guard = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let set = guard.get_or_insert_with(HashSet::new);
    f(set)
}

/// Register a temp file path. It will be cleaned up on process exit if not unregistered first.
pub fn register(path: PathBuf) {
    with_registry(|set| {
        set.insert(path);
    });
}

/// Unregister a temp file (call after you've already deleted it yourself).
pub fn unregister(path: &std::path::Path) {
    with_registry(|set| {
        set.remove(path);
    });
}

/// Generate a unique temp file path for BioLang operations.
/// Format: `<temp_dir>/biolang_<tag>_<pid>_<counter>.db`
pub fn temp_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("biolang_{tag}_{}_{n}.db", std::process::id()))
}

/// Delete all registered temp files. Call from REPL/CLI shutdown.
/// Safe to call multiple times.
pub fn cleanup_all() {
    let paths: Vec<PathBuf> = with_registry(|set| set.drain().collect());
    for path in &paths {
        let _ = std::fs::remove_file(path);
    }
    if !paths.is_empty() {
        eprintln!("Cleaned up {} temporary file(s).", paths.len());
    }
}

/// Remove stale temp files from previous BioLang sessions that didn't clean up
/// (e.g. due to crash or SIGKILL). Call once at startup.
pub fn cleanup_stale() {
    let tmp = std::env::temp_dir();
    let pid = std::process::id();
    if let Ok(entries) = std::fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("biolang_") && name.ends_with(".db") {
                // Don't delete files from the current process
                let is_current = name.contains(&format!("_{pid}_"));
                if !is_current {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}
