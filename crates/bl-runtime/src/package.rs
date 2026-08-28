use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package manifest (`biolang.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
    #[serde(default)]
    pub lib: Option<LibraryInfo>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfo {
    pub entry: String,
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
        lib: None,
    };

    let toml_str = toml::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;

    let path = dir.join("biolang.toml");
    std::fs::write(&path, toml_str).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;

    // Create main.bl
    let main_path = dir.join("main.bl");
    if !main_path.exists() {
        std::fs::write(
            &main_path,
            "# BioLang project\nprintln(\"Hello from BioLang!\")\n",
        )
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

    copy_dir_recursive(source_path, &target).map_err(|e| format!("Cannot copy package: {e}"))?;

    Ok(target)
}

/// Install a dependency by git URL.
pub fn install_git_dep(name: &str, url: &str, branch: Option<&str>) -> Result<PathBuf, String> {
    let target = packages_dir().join(name);
    let parent = target
        .parent()
        .ok_or_else(|| format!("Cannot resolve package directory for {name}"))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(&format!(".{name}-install-"))
        .tempdir_in(parent)
        .map_err(|e| format!("Cannot create package staging directory: {e}"))?;
    let checkout = staging.path().join("checkout");

    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(&checkout);

    let output = cmd.output().map_err(|e| format!("git clone failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {stderr}"));
    }

    let source = package_source_in_checkout(&checkout, name)?;
    let manifest = read_manifest(&source)?;
    if manifest.package.name != name {
        return Err(format!(
            "Package manifest names '{}' but it was installed as '{name}'",
            manifest.package.name
        ));
    }

    // Copy and validate before replacing an existing install. A network error,
    // an invalid repository, or a malformed package therefore leaves the
    // user's working version untouched.
    let prepared = staging.path().join("prepared");
    copy_dir_recursive(&source, &prepared)
        .map_err(|e| format!("Cannot prepare package '{name}': {e}"))?;
    resolve_library_entry(&prepared)?;

    let backup = staging.path().join("previous");
    let had_previous = target.exists();
    if had_previous {
        std::fs::rename(&target, &backup)
            .map_err(|e| format!("Cannot prepare to replace {}: {e}", target.display()))?;
    }
    if let Err(error) = std::fs::rename(&prepared, &target) {
        if had_previous {
            let _ = std::fs::rename(&backup, &target);
        }
        return Err(format!(
            "Cannot activate package at {}: {error}",
            target.display()
        ));
    }
    Ok(target)
}

/// Built-in package sources used by `bl install <name>` when `<name>` is not
/// a local path. Releases bundle these packages, but source or Cargo installs
/// can fetch them without requiring BIOLANG_PATH.
pub fn registry_git_url(name: &str) -> Option<&'static str> {
    match name {
        "statistics" => Some("https://github.com/oriclabs/biolang.git"),
        _ => None,
    }
}

fn package_source_in_checkout(checkout: &Path, name: &str) -> Result<PathBuf, String> {
    if checkout.join("biolang.toml").is_file() {
        return Ok(checkout.to_path_buf());
    }
    let monorepo_package = checkout.join("packages").join(name);
    if monorepo_package.join("biolang.toml").is_file() {
        return Ok(monorepo_package);
    }
    Err(format!(
        "Git repository does not contain biolang.toml at its root or packages/{name}/biolang.toml"
    ))
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

/// Resolve the explicit `[lib].entry` declared by a package manifest.
///
/// The canonical-path check prevents a package from using `..` or an absolute
/// path to make its public entrypoint escape the package directory.
pub fn resolve_library_entry(dir: &Path) -> Result<Option<PathBuf>, String> {
    if !dir.join("biolang.toml").is_file() {
        return Ok(None);
    }

    let manifest = read_manifest(dir)?;
    let Some(lib) = manifest.lib else {
        return Ok(None);
    };

    let package_root = dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve package directory {}: {e}", dir.display()))?;
    let entry = dir.join(&lib.entry).canonicalize().map_err(|e| {
        format!(
            "Cannot resolve library entry '{}' for package '{}': {e}",
            lib.entry, manifest.package.name
        )
    })?;

    if !entry.starts_with(&package_root) {
        return Err(format!(
            "Library entry '{}' for package '{}' escapes the package directory",
            lib.entry, manifest.package.name
        ));
    }
    if !entry.is_file() {
        return Err(format!(
            "Library entry '{}' for package '{}' is not a file",
            lib.entry, manifest.package.name
        ));
    }

    Ok(Some(entry))
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

/// List files bundled under a package's `examples` directory.
pub fn list_examples(package_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let root = package_dir.join("examples");
    if !root.is_dir() {
        return Err(format!(
            "Package at {} does not contain an examples directory",
            package_dir.display()
        ));
    }
    let mut examples = Vec::new();
    collect_relative_files(&root, &root, &mut examples)
        .map_err(|error| format!("Cannot read {}: {error}", root.display()))?;
    examples.sort();
    if examples.is_empty() {
        return Err(format!("Package examples are empty at {}", root.display()));
    }
    Ok(examples)
}

/// Copy a package's complete example set to a new or empty working directory.
pub fn copy_examples(package_dir: &Path, destination: &Path) -> Result<PathBuf, String> {
    let source = package_dir.join("examples");
    list_examples(package_dir)?;
    if destination.exists() {
        if !destination.is_dir() {
            return Err(format!(
                "Example destination is not a directory: {}",
                destination.display()
            ));
        }
        let mut entries = std::fs::read_dir(destination)
            .map_err(|error| format!("Cannot read {}: {error}", destination.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "Example destination must be new or empty: {}",
                destination.display()
            ));
        }
    }
    copy_dir_recursive(&source, destination)
        .map_err(|error| format!("Cannot copy package examples: {error}"))?;
    destination
        .canonicalize()
        .or_else(|_| std::path::absolute(destination))
        .map_err(|error| format!("Cannot resolve {}: {error}", destination.display()))
}

fn collect_relative_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_relative_files(root, &entry.path(), files)?;
        } else {
            let path = entry.path();
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_name() == ".git" {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::{
        copy_dir_recursive, copy_examples, list_examples, package_source_in_checkout,
        registry_git_url,
    };
    use std::path::PathBuf;

    #[test]
    fn lists_and_copies_nested_package_examples() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("demo");
        let examples = package.join("examples");
        std::fs::create_dir_all(examples.join("data")).unwrap();
        std::fs::write(examples.join("quickstart.bl"), "println(\"ok\")\n").unwrap();
        std::fs::write(examples.join("data").join("input.tsv"), "gene\tcount\n").unwrap();

        assert_eq!(
            list_examples(&package).unwrap(),
            vec![
                PathBuf::from("data").join("input.tsv"),
                PathBuf::from("quickstart.bl"),
            ]
        );

        let destination = temp.path().join("working-examples");
        copy_examples(&package, &destination).unwrap();
        assert!(destination.join("quickstart.bl").is_file());
        assert!(destination.join("data").join("input.tsv").is_file());
    }

    #[test]
    fn package_copy_excludes_git_metadata_at_any_depth() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        std::fs::create_dir_all(source.join(".git").join("objects")).unwrap();
        std::fs::create_dir_all(source.join("nested").join(".git")).unwrap();
        std::fs::write(source.join("biolang.toml"), "[package]\nname='demo'\n").unwrap();
        std::fs::write(source.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            source.join("nested").join(".git").join("HEAD"),
            "metadata\n",
        )
        .unwrap();

        copy_dir_recursive(&source, &destination).unwrap();

        assert!(destination.join("biolang.toml").is_file());
        assert!(!destination.join(".git").exists());
        assert!(!destination.join("nested").join(".git").exists());
    }

    #[test]
    fn refuses_to_merge_examples_into_a_nonempty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("demo");
        std::fs::create_dir_all(package.join("examples")).unwrap();
        std::fs::write(package.join("examples").join("quickstart.bl"), "").unwrap();
        let destination = temp.path().join("existing");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("keep.txt"), "keep").unwrap();

        let error = copy_examples(&package, &destination).unwrap_err();
        assert!(error.contains("new or empty"));
        assert_eq!(
            std::fs::read_to_string(destination.join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn resolves_named_registry_and_monorepo_package_without_network() {
        assert_eq!(
            registry_git_url("statistics"),
            Some("https://github.com/oriclabs/biolang.git")
        );
        assert_eq!(registry_git_url("unknown"), None);

        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("packages").join("statistics");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("biolang.toml"),
            "[package]\nname='statistics'\nversion='0.1.0'\n",
        )
        .unwrap();
        assert_eq!(
            package_source_in_checkout(temp.path(), "statistics").unwrap(),
            package
        );
    }
}
