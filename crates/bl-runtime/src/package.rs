use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package manifest (`biolang.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
}

/// A dependency — either a version string or a table with path/git.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Version(String),
    Detailed(DetailedDep),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDep {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

/// Global package directory.
pub fn packages_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".biolang")
        .join("packages")
}

/// Read a manifest from a directory.
pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join("biolang.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("Invalid manifest {}: {e}", path.display()))
}

/// Initialize a new package in the given directory.
pub fn init_package(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let manifest = Manifest {
        package: PackageInfo {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: None,
            authors: Vec::new(),
            license: None,
        },
        dependencies: HashMap::new(),
    };

    let toml_str = toml::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;

    let path = dir.join("biolang.toml");
    std::fs::write(&path, toml_str)
        .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;

    // Create main.bl
    let main_path = dir.join("main.bl");
    if !main_path.exists() {
        std::fs::write(&main_path, "# BioLang project\nprintln(\"Hello from BioLang!\")\n")
            .map_err(|e| format!("Cannot write main.bl: {e}"))?;
    }

    Ok(path)
}

/// Install a dependency by path.
pub fn install_path_dep(name: &str, source_path: &Path) -> Result<PathBuf, String> {
    let target = packages_dir().join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("Cannot remove existing {}: {e}", target.display()))?;
    }

    copy_dir_recursive(source_path, &target)
        .map_err(|e| format!("Cannot copy package: {e}"))?;

    Ok(target)
}

/// Install a dependency by git URL.
pub fn install_git_dep(name: &str, url: &str, branch: Option<&str>) -> Result<PathBuf, String> {
    let target = packages_dir().join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("Cannot remove existing {}: {e}", target.display()))?;
    }

    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(&target);

    let output = cmd.output().map_err(|e| format!("git clone failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {stderr}"));
    }

    Ok(target)
}

/// Resolve a package name to its install path.
pub fn resolve_package(name: &str) -> Option<PathBuf> {
    let path = packages_dir().join(name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// List installed packages.
pub fn list_packages() -> Vec<(String, Option<Manifest>)> {
    let dir = packages_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut packages = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let manifest = read_manifest(&entry.path()).ok();
                packages.push((name, manifest));
            }
        }
    }
    packages.sort_by(|a, b| a.0.cmp(&b.0));
    packages
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
