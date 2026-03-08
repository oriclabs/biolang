//! BioContainers registry client.
//!
//! Accesses the BioContainers GA4GH TRS v2 API at <https://api.biocontainers.pro>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("biocontainers", "https://api.biocontainers.pro/ga4gh/trs/v2")
}

/// BioContainers GA4GH TRS v2 API client.
pub struct BioContainersClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub organization: String,
    #[serde(default)]
    pub versions: Vec<ToolVersion>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolVersion {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub meta_version: Option<String>,
    #[serde(default)]
    pub images: Vec<ContainerImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerImage {
    #[serde(default)]
    pub registry_host: String,
    #[serde(default)]
    pub image_name: String,
    #[serde(default)]
    pub image_type: String,
    #[serde(default)]
    pub size: Option<u64>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl BioContainersClient {
    pub fn new() -> Self {
        BioContainersClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        BioContainersClient { base }
    }

    /// Search tools by name.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Tool>> {
        let base = base_url();
        let url = format!("{base}/tools?name={query}&limit={limit}");
        let json = self.base.get_json(&url)?;
        parse_tools_array(&json, &url)
    }

    /// List tools with pagination.
    pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<Tool>> {
        let base = base_url();
        let url = format!("{base}/tools?limit={limit}&offset={offset}");
        let json = self.base.get_json(&url)?;
        parse_tools_array(&json, &url)
    }

    /// List popular tools sorted by stars descending.
    pub fn popular(&self, limit: usize) -> Result<Vec<Tool>> {
        let base = base_url();
        let url = format!("{base}/tools?sort_field=stars&sort_order=desc&limit={limit}");
        let json = self.base.get_json(&url)?;
        parse_tools_array(&json, &url)
    }

    /// Get tool detail by id (typically the tool name, e.g. "samtools").
    pub fn tool_info(&self, id: &str) -> Result<Tool> {
        let base = base_url();
        let url = format!("{base}/tools/{id}");
        let json = self.base.get_json(&url)?;
        serde_json::from_value(json).map_err(|e| ApiError::Parse {
            context: url,
            source: e.to_string(),
        })
    }

    /// Get versions for a tool.
    pub fn tool_versions(&self, id: &str) -> Result<Vec<ToolVersion>> {
        let base = base_url();
        let url = format!("{base}/tools/{id}/versions");
        let json = self.base.get_json(&url)?;
        serde_json::from_value(json).map_err(|e| ApiError::Parse {
            context: url,
            source: e.to_string(),
        })
    }
}

impl Default for BioContainersClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a full image URI from registry_host and image_name.
pub fn image_uri(img: &ContainerImage) -> String {
    if img.registry_host.is_empty() {
        img.image_name.clone()
    } else {
        format!("{}/{}", img.registry_host, img.image_name)
    }
}

fn parse_tools_array(json: &serde_json::Value, url: &str) -> Result<Vec<Tool>> {
    serde_json::from_value(json.clone()).map_err(|e| ApiError::Parse {
        context: url.to_string(),
        source: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "samtools",
                "name": "samtools",
                "description": "Tools for SAM/BAM",
                "organization": "biocontainers",
                "versions": [{
                    "id": "samtools-1.19",
                    "name": "1.19",
                    "images": [{
                        "registry_host": "quay.io",
                        "image_name": "biocontainers/samtools:1.19--h50ea8bc_0",
                        "image_type": "Docker",
                        "size": 12345
                    }]
                }],
                "aliases": ["samtools"]
            }"#,
        )
        .unwrap();
        let tool: Tool = serde_json::from_value(json).unwrap();
        assert_eq!(tool.name, "samtools");
        assert_eq!(tool.versions.len(), 1);
        assert_eq!(tool.versions[0].name, "1.19");
        assert_eq!(tool.versions[0].images.len(), 1);
        assert_eq!(
            image_uri(&tool.versions[0].images[0]),
            "quay.io/biocontainers/samtools:1.19--h50ea8bc_0"
        );
    }

    #[test]
    fn test_parse_tools_array() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"id": "bwa", "name": "bwa", "description": "", "organization": "biocontainers", "versions": [], "aliases": []}]"#,
        )
        .unwrap();
        let tools = parse_tools_array(&json, "test").unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "bwa");
    }

    #[test]
    fn test_image_uri_with_registry() {
        let img = ContainerImage {
            registry_host: "quay.io".into(),
            image_name: "biocontainers/samtools:1.19".into(),
            image_type: "Docker".into(),
            size: None,
        };
        assert_eq!(image_uri(&img), "quay.io/biocontainers/samtools:1.19");
    }

    #[test]
    fn test_image_uri_without_registry() {
        let img = ContainerImage {
            registry_host: String::new(),
            image_name: "biocontainers/samtools:1.19".into(),
            image_type: "Docker".into(),
            size: None,
        };
        assert_eq!(image_uri(&img), "biocontainers/samtools:1.19");
    }

    #[test]
    fn test_tool_missing_fields() {
        let json: serde_json::Value = serde_json::from_str(r#"{"id": "test"}"#).unwrap();
        let tool: Tool = serde_json::from_value(json).unwrap();
        assert_eq!(tool.id, "test");
        assert_eq!(tool.name, "");
        assert!(tool.versions.is_empty());
        assert!(tool.aliases.is_empty());
    }
}
