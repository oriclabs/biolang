//! API endpoint configuration.
//!
//! Allows users to override base URLs for all bio API clients via:
//! 1. Environment variable: `BIOLANG_{SERVICE}_URL` (highest priority)
//! 2. Config file: `~/.biolang/apis.yaml` (persistent overrides)
//! 3. Hardcoded default (fallback)
//!
//! Example `~/.biolang/apis.yaml`:
//! ```yaml
//! ncbi: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils"
//! ensembl: "https://rest.ensembl.org"
//! nfcore_catalog: "https://nf-co.re/pipelines.json"
//! nfcore_github: "https://api.github.com"
//! nfcore_raw: "https://raw.githubusercontent.com"
//! biocontainers: "https://api.biocontainers.pro/ga4gh/trs/v2"
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

static CONFIG: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Load the config file once, caching the result.
fn load_config() -> &'static HashMap<String, String> {
    CONFIG.get_or_init(|| {
        let path = dirs_path().join("apis.yaml");
        if path.exists() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return parse_yaml_map(&text);
            }
        }
        HashMap::new()
    })
}

/// Resolve the base URL for a given service.
///
/// Checks in order:
/// 1. `BIOLANG_{SERVICE}_URL` env var (uppercased)
/// 2. `~/.biolang/apis.yaml` key (lowercase)
/// 3. `default` hardcoded value
pub fn resolve_url(service: &str, default: &str) -> String {
    // 1. Env var: BIOLANG_NCBI_URL, BIOLANG_NFCORE_CATALOG_URL, etc.
    let env_key = format!("BIOLANG_{}_URL", service.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        if !val.is_empty() {
            return val;
        }
    }

    // 2. Config file
    let config = load_config();
    if let Some(val) = config.get(service) {
        if !val.is_empty() {
            return val.clone();
        }
    }

    // 3. Default
    default.to_string()
}

/// Returns the BioLang config directory (`~/.biolang/`).
fn dirs_path() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("BIOLANG_CONFIG_DIR") {
        return std::path::PathBuf::from(val);
    }
    dirs_home().join(".biolang")
}

fn dirs_home() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\Users\\default"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
    }
}

/// Minimal YAML map parser for `key: "value"` or `key: value` lines.
/// No dependency on a full YAML library.
fn parse_yaml_map(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim().to_string();
            let val = val.trim();
            // Strip surrounding quotes
            let val = val
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| val.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(val);
            map.insert(key, val.to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_map() {
        let yaml = r#"
# API endpoint overrides
ncbi: "https://custom.ncbi.example.com"
ensembl: 'https://mirror.ensembl.org'
biocontainers: https://my-proxy.example.com/trs/v2
        "#;
        let map = parse_yaml_map(yaml);
        assert_eq!(map["ncbi"], "https://custom.ncbi.example.com");
        assert_eq!(map["ensembl"], "https://mirror.ensembl.org");
        assert_eq!(map["biocontainers"], "https://my-proxy.example.com/trs/v2");
    }

    #[test]
    fn test_resolve_url_default() {
        // With no env var or config, should return default
        let url = resolve_url("test_nonexistent_service_xyz", "https://default.example.com");
        assert_eq!(url, "https://default.example.com");
    }
}
