use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO_OWNER: &str = "oriclabs";
const GITHUB_REPO_NAME: &str = "biolang";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(serde::Serialize, Deserialize, Default)]
struct UpdateCache {
    last_check: u64,
    latest_version: String,
    download_url: String,
    release_url: String,
}

fn config_dir() -> Option<PathBuf> {
    dirs_path().map(|d| {
        let _ = fs::create_dir_all(&d);
        d
    })
}

fn dirs_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(|h| PathBuf::from(h).join(".biolang"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".biolang"))
    }
}

fn cache_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("update-check.json"))
}

fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(cache: &UpdateCache) {
    if let Some(path) = cache_path() {
        if let Ok(data) = serde_json::to_string_pretty(cache) {
            let _ = fs::write(path, data);
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_update_disabled() -> bool {
    std::env::var("BIOLANG_NO_UPDATE_CHECK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn platform_asset_name() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "biolang-linux-x86_64.tar.gz" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "biolang-macos-x86_64.tar.gz" }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "biolang-macos-aarch64.tar.gz" }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { "biolang-windows-x86_64.zip" }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    { "" }
}

fn fetch_latest_release() -> Result<GitHubRelease, String> {
    let url = format!(
        "https://api.github.com/repos/{GITHUB_REPO_OWNER}/{GITHUB_REPO_NAME}/releases/latest"
    );
    let resp = ureq::get(&url)
        .set("User-Agent", &format!("biolang-cli/{CURRENT_VERSION}"))
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|e| format!("network error: {e}"))?;

    let body = resp.into_string().map_err(|e| format!("read error: {e}"))?;
    serde_json::from_str::<GitHubRelease>(&body).map_err(|e| format!("parse error: {e}"))
}

/// Spawn a background thread to check for updates.
/// Prints a one-line warning to stderr if a newer version is available.
/// Does nothing if:
/// - BIOLANG_NO_UPDATE_CHECK=1
/// - Last check was within 24 hours
/// - Network is unavailable
pub fn check_for_updates_background() {
    if is_update_disabled() {
        return;
    }

    // Check cache first (no network needed)
    if let Some(cache) = read_cache() {
        let elapsed = now_secs().saturating_sub(cache.last_check);
        if elapsed < CHECK_INTERVAL.as_secs() {
            // Cache is fresh — show warning if we already know about a newer version
            if !cache.latest_version.is_empty() && is_newer(&cache.latest_version, CURRENT_VERSION)
            {
                print_update_warning(&cache.latest_version);
            }
            return;
        }
    }

    // Cache is stale or missing — check in background
    std::thread::spawn(|| {
        if let Ok(release) = fetch_latest_release() {
            let version = release.tag_name.strip_prefix('v')
                .unwrap_or(&release.tag_name)
                .to_string();

            let download_url = release
                .assets
                .iter()
                .find(|a| a.name == platform_asset_name())
                .map(|a| a.browser_download_url.clone())
                .unwrap_or_default();

            let cache = UpdateCache {
                last_check: now_secs(),
                latest_version: version.clone(),
                download_url,
                release_url: release.html_url,
            };
            write_cache(&cache);

            if is_newer(&version, CURRENT_VERSION) {
                print_update_warning(&version);
            }
        } else {
            // Network failed — update timestamp so we don't retry immediately
            let cache = UpdateCache {
                last_check: now_secs(),
                ..read_cache().unwrap_or_default()
            };
            write_cache(&cache);
        }
    });
}

fn print_update_warning(latest: &str) {
    eprintln!(
        "\x1b[33mA new version of BioLang is available: v{latest} (current: v{CURRENT_VERSION}). Run 'bl upgrade' to update.\x1b[0m"
    );
}

/// Show version info and check for updates (blocking).
pub fn cmd_version() {
    println!("BioLang v{CURRENT_VERSION}");
    println!();

    if is_update_disabled() {
        println!("Update checking is disabled (BIOLANG_NO_UPDATE_CHECK=1).");
        return;
    }

    eprint!("Checking for updates... ");
    match fetch_latest_release() {
        Ok(release) => {
            let version = release.tag_name.strip_prefix('v')
                .unwrap_or(&release.tag_name)
                .to_string();

            let download_url = release
                .assets
                .iter()
                .find(|a| a.name == platform_asset_name())
                .map(|a| a.browser_download_url.clone())
                .unwrap_or_default();

            let cache = UpdateCache {
                last_check: now_secs(),
                latest_version: version.clone(),
                download_url,
                release_url: release.html_url,
            };
            write_cache(&cache);

            if is_newer(&version, CURRENT_VERSION) {
                eprintln!("v{version} available!");
                println!("Run 'bl upgrade' to update.");
                println!("Release: {}", cache.release_url);
            } else {
                eprintln!("up to date.");
            }
        }
        Err(e) => {
            eprintln!("failed ({e})");
        }
    }
}

/// Download the latest release and replace the current binary.
pub fn cmd_upgrade() {
    println!("BioLang v{CURRENT_VERSION}");
    println!();

    eprint!("Checking for latest release... ");
    let release = match fetch_latest_release() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("failed");
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let latest = release.tag_name.strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    if !is_newer(&latest, CURRENT_VERSION) {
        eprintln!("already up to date (v{CURRENT_VERSION}).");
        return;
    }

    let asset_name = platform_asset_name();
    if asset_name.is_empty() {
        eprintln!();
        eprintln!("Error: no pre-built binary available for this platform.");
        eprintln!("Build from source: cargo install --path crates/bl-cli");
        std::process::exit(1);
    }

    let asset = match release.assets.iter().find(|a| a.name == asset_name) {
        Some(a) => a,
        None => {
            eprintln!();
            eprintln!("Error: release v{latest} has no asset '{asset_name}'");
            eprintln!("Download manually: {}", release.html_url);
            std::process::exit(1);
        }
    };

    eprintln!("v{latest} found.");
    eprintln!("Downloading {}...", asset.name);

    let resp = match ureq::get(&asset.browser_download_url)
        .set("User-Agent", &format!("biolang-cli/{CURRENT_VERSION}"))
        .timeout(Duration::from_secs(120))
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error downloading: {e}");
            std::process::exit(1);
        }
    };

    // Read full response body
    let mut body = Vec::new();
    let mut reader = resp.into_reader();
    if let Err(e) = std::io::Read::read_to_end(&mut reader, &mut body) {
        eprintln!("Error reading download: {e}");
        std::process::exit(1);
    }

    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error finding current executable: {e}");
            std::process::exit(1);
        }
    };

    // Extract to a temp directory
    let tmp_dir = std::env::temp_dir().join(format!("biolang-upgrade-{latest}"));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).unwrap_or_else(|e| {
        eprintln!("Error creating temp dir: {e}");
        std::process::exit(1);
    });

    eprintln!("Extracting...");

    #[cfg(unix)]
    {
        use std::process::Command;

        let archive_path = tmp_dir.join(&asset.name);
        fs::write(&archive_path, &body).unwrap_or_else(|e| {
            eprintln!("Error writing archive: {e}");
            std::process::exit(1);
        });

        let status = Command::new("tar")
            .args(["xzf", &archive_path.to_string_lossy(), "-C", &tmp_dir.to_string_lossy()])
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => {
                eprintln!("Error extracting archive. Install manually from:");
                eprintln!("{}", release.html_url);
                let _ = fs::remove_dir_all(&tmp_dir);
                std::process::exit(1);
            }
        }
    }

    #[cfg(windows)]
    {
        let cursor = std::io::Cursor::new(&body);
        let mut archive = zip::ZipArchive::new(cursor).unwrap_or_else(|e| {
            eprintln!("Error opening zip: {e}");
            let _ = fs::remove_dir_all(&tmp_dir);
            std::process::exit(1);
        });

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = tmp_dir.join(file.name());
            if file.is_dir() {
                fs::create_dir_all(&outpath).ok();
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent).ok();
                }
                let mut outfile = fs::File::create(&outpath).unwrap_or_else(|e| {
                    eprintln!("Error creating {}: {e}", outpath.display());
                    std::process::exit(1);
                });
                std::io::copy(&mut file, &mut outfile).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {e}", outpath.display());
                    std::process::exit(1);
                });
            }
        }
    }

    // Find the new binary
    let bin_name = if cfg!(windows) { "bl.exe" } else { "bl" };
    let new_binary = tmp_dir.join(bin_name);
    if !new_binary.exists() {
        eprintln!("Error: '{bin_name}' not found in archive.");
        eprintln!("Contents:");
        if let Ok(entries) = fs::read_dir(&tmp_dir) {
            for entry in entries.flatten() {
                eprintln!("  {}", entry.file_name().to_string_lossy());
            }
        }
        let _ = fs::remove_dir_all(&tmp_dir);
        std::process::exit(1);
    }

    // Replace the current binary
    let backup = current_exe.with_extension("old");
    eprintln!("Replacing {}...", current_exe.display());

    // On Windows, rename the running exe first (can't overwrite in-use binary)
    if let Err(e) = fs::rename(&current_exe, &backup) {
        eprintln!("Error backing up current binary: {e}");
        eprintln!("Try running as administrator, or manually replace:");
        eprintln!("  {} -> {}", new_binary.display(), current_exe.display());
        let _ = fs::remove_dir_all(&tmp_dir);
        std::process::exit(1);
    }

    if let Err(e) = fs::copy(&new_binary, &current_exe) {
        // Restore backup
        let _ = fs::rename(&backup, &current_exe);
        eprintln!("Error installing new binary: {e}");
        let _ = fs::remove_dir_all(&tmp_dir);
        std::process::exit(1);
    }

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755));
    }

    // Clean up
    let _ = fs::remove_file(&backup);
    let _ = fs::remove_dir_all(&tmp_dir);

    // Update cache
    let cache = UpdateCache {
        last_check: now_secs(),
        latest_version: latest.clone(),
        download_url: asset.browser_download_url.clone(),
        release_url: release.html_url.clone(),
    };
    write_cache(&cache);

    eprintln!("\x1b[32mUpgraded to BioLang v{latest}!\x1b[0m");
    eprintln!("Release notes: {}", release.html_url);
}
