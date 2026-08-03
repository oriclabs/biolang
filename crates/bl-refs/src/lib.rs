//! The named reference genome registry.
//!
//! Every bio script eventually hard-codes `/data/refs/GRCh38.fa`, and every
//! such script breaks on the next machine. The paths genuinely are
//! machine-specific — that is the point — so they belong in a per-machine
//! registry rather than in the analysis, and the analysis should name the build
//! it wants and let the machine resolve it.
//!
//! The registry lives at `~/.biolang/references.toml`:
//!
//! ```toml
//! [GRCh38]
//! description = "GENCODE 44 primary assembly"
//! fasta = "/data/refs/GRCh38.primary_assembly.fa"
//! gtf = "/data/refs/gencode.v44.annotation.gtf"
//! ```
//!
//! This is a standalone crate so the runtime and the workbench can share it
//! without the workbench inheriting the runtime's native dependencies.

use std::collections::HashMap;
use std::path::PathBuf;

/// Build name to asset name to path.
pub type Registry = HashMap<String, HashMap<String, String>>;

/// Asset key that holds prose rather than a path.
pub const DESCRIPTION_KEY: &str = "description";

/// Path to the registry file.
pub fn registry_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".biolang")
        .join("references.toml")
}

/// Parse registry TOML, ignoring anything that is not a table of strings.
pub fn parse(text: &str) -> Registry {
    let Ok(parsed) = toml::from_str::<toml::Value>(text) else {
        return Registry::new();
    };
    let Some(table) = parsed.as_table() else {
        return Registry::new();
    };

    let mut registry = Registry::new();
    for (build, assets) in table {
        let Some(assets) = assets.as_table() else {
            continue;
        };
        let entries: HashMap<String, String> = assets
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_string())))
            .collect();
        if !entries.is_empty() {
            registry.insert(build.clone(), entries);
        }
    }
    registry
}

/// Render a registry as TOML, with builds and assets in a stable order so the
/// file does not churn between saves.
pub fn render(registry: &Registry) -> String {
    let mut document = toml::map::Map::new();
    let mut builds: Vec<&String> = registry.keys().collect();
    builds.sort();
    for build in builds {
        let assets = &registry[build];
        let mut table = toml::map::Map::new();
        let mut keys: Vec<&String> = assets.keys().collect();
        keys.sort();
        for key in keys {
            table.insert(key.clone(), toml::Value::String(assets[key].clone()));
        }
        document.insert(build.clone(), toml::Value::Table(table));
    }
    toml::to_string_pretty(&toml::Value::Table(document)).unwrap_or_default()
}

/// Read the registry.
///
/// A missing or malformed file reads as empty rather than failing: a script
/// that does not use `ref` must not break because of a stray file.
pub fn load() -> Registry {
    std::fs::read_to_string(registry_path())
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// Write the registry back, creating `~/.biolang` if needed.
pub fn save(registry: &Registry) -> std::io::Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render(registry))
}

/// Configured build names, sorted, for discovery and error messages.
pub fn build_names(registry: &Registry) -> Vec<String> {
    let mut names: Vec<String> = registry.keys().cloned().collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[GRCh38]
description = "GENCODE 44"
fasta = "/refs/h38.fa"
gtf = "/refs/h38.gtf"

[mm39]
fasta = "/refs/mm39.fa"
"#;

    #[test]
    fn parses_builds_and_assets() {
        let registry = parse(SAMPLE);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry["GRCh38"]["gtf"], "/refs/h38.gtf");
        assert_eq!(registry["mm39"].len(), 1);
    }

    #[test]
    fn build_names_are_sorted() {
        assert_eq!(build_names(&parse(SAMPLE)), vec!["GRCh38", "mm39"]);
    }

    #[test]
    fn malformed_toml_reads_as_empty() {
        assert!(parse("this is not toml {{{").is_empty());
        assert!(parse("").is_empty());
    }

    #[test]
    fn non_string_assets_are_ignored() {
        // A number where a path belongs is a typo, not a path.
        let registry = parse("[GRCh38]\nfasta = 42\ngtf = \"/refs/h38.gtf\"\n");
        assert_eq!(registry["GRCh38"].len(), 1);
        assert!(registry["GRCh38"].contains_key("gtf"));
    }

    #[test]
    fn a_build_with_no_usable_assets_is_dropped() {
        assert!(parse("[GRCh38]\nfasta = 42\n").is_empty());
    }

    #[test]
    fn rendering_round_trips_and_is_stable() {
        let registry = parse(SAMPLE);
        let rendered = render(&registry);
        assert_eq!(parse(&rendered), registry);
        // Same input, same bytes: the file must not churn between saves.
        assert_eq!(render(&parse(&rendered)), rendered);
    }

    #[test]
    fn rendering_orders_builds_alphabetically() {
        let rendered = render(&parse(SAMPLE));
        assert!(
            rendered.find("[GRCh38]") < rendered.find("[mm39]"),
            "unexpected order:\n{rendered}"
        );
    }
}
