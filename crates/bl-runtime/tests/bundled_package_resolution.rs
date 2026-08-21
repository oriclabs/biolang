//! Packages shipped beside the executable must be importable, and must lose to
//! a package the user chose.
//!
//! A release archive carries `packages/` next to `bl`, because there is no
//! registry: without the bundled copy, `import "statistics"` cannot resolve on
//! a machine that has only unpacked a tarball, even though the books, the
//! website and the package examples all teach it. The ordering is the part
//! worth pinning — searching the bundled directory before the user's own would
//! let a shipped copy silently shadow whatever they checked out or installed,
//! which is the failure mode a bundled default is most likely to introduce.

use bl_core::value::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Writes a minimal package exporting `marker()` returning `label`.
fn write_package(root: &Path, name: &str, label: &str) {
    let src = root.join(name).join("src");
    fs::create_dir_all(&src).expect("package source directory");
    fs::write(
        root.join(name).join("biolang.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.1\"\n\n[lib]\nentry = \"src/mod.bl\"\n"
        ),
    )
    .expect("manifest");
    fs::write(
        src.join("mod.bl"),
        format!("fn marker() {{\n    \"{label}\"\n}}\n"),
    )
    .expect("module");
}

fn run(source: &str) -> Result<Value, String> {
    let tokens = bl_lexer::Lexer::new(source)
        .tokenize()
        .map_err(|error| error.message)?;
    let parsed = bl_parser::Parser::new(tokens)
        .parse()
        .map_err(|error| error.message)?;
    if parsed.has_errors() {
        return Err(parsed.errors[0].message.clone());
    }
    bl_runtime::interpreter::Interpreter::new()
        .run(&parsed.program)
        .map_err(|error| error.message)
}

/// `packages/` beside the test binary stands in for `packages/` beside `bl`.
fn executable_package_root() -> PathBuf {
    std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable directory")
        .join("packages")
}

/// Removes the probe package, then the `packages/` directory that held it if
/// this test created it. One of the two roots is inside the source tree, and a
/// test has no business leaving a directory there.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        if let Some(parent) = self.0.parent() {
            // Fails harmlessly when the directory is a real one with contents.
            let _ = fs::remove_dir(parent);
        }
    }
}

#[test]
fn a_package_beside_the_executable_is_importable() {
    // Unique so this cannot collide with a repository package, with the sibling
    // test, or with a stale directory from an interrupted run.
    let name = "bundled_probe_importable";
    let root = executable_package_root();
    let _cleanup = Cleanup(root.join(name));
    write_package(&root, name, "BUNDLED");

    let result = run(&format!(
        "import \"{name}\" as pkg\nlet found = pkg.marker()\nfound\n"
    ));

    assert_eq!(
        result,
        Ok(Value::Str("BUNDLED".into())),
        "a package next to the executable should resolve with no install step"
    );
}

#[test]
fn a_package_in_the_working_tree_beats_the_bundled_copy() {
    let name = "bundled_probe_precedence";
    let bundled_root = executable_package_root();
    let _bundled_cleanup = Cleanup(bundled_root.join(name));
    write_package(&bundled_root, name, "BUNDLED");

    // `packages/` at or above the working directory is searched first, so a
    // checkout or a `bl install` result is what the import reaches.
    let local_root = std::env::current_dir().expect("cwd").join("packages");
    let _local_cleanup = Cleanup(local_root.join(name));
    write_package(&local_root, name, "LOCAL");

    let result = run(&format!(
        "import \"{name}\" as pkg\nlet found = pkg.marker()\nfound\n"
    ));

    assert_eq!(
        result,
        Ok(Value::Str("LOCAL".into())),
        "the user's own package must win; a bundled copy is the floor, not the ceiling"
    );
}
