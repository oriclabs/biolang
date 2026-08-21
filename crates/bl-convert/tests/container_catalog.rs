//! Network-backed validation for the curated BioContainers catalog.

use bl_convert::containers::TOOL_CATALOG;
use std::env;
use std::process::Command;

#[test]
#[ignore = "requires Docker/Podman and registry access; see crates/bl-convert/tests/README.md"]
fn every_pinned_catalog_image_exists() {
    let runtime = env::var("BL_CONVERT_TEST_MANIFEST_RUNTIME")
        .or_else(|_| env::var("BL_CONVERT_TEST_RUNTIME"))
        .expect("set BL_CONVERT_TEST_MANIFEST_RUNTIME=docker or podman to validate registry tags");
    assert!(
        matches!(runtime.as_str(), "docker" | "podman"),
        "manifest validation supports docker or podman, got '{runtime}'"
    );

    let mut missing = Vec::new();
    for tool in TOOL_CATALOG {
        let output = Command::new(&runtime)
            .args(["manifest", "inspect", tool.image])
            .output()
            .unwrap_or_else(|error| panic!("cannot start {runtime}: {error}"));
        if !output.status.success() {
            missing.push(format!(
                "{} ({}): {}",
                tool.name,
                tool.image,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "catalog contains missing images:\n{}",
        missing.join("\n")
    );
}
