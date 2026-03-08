//! nf-core pipeline catalog client.
//!
//! Accesses the nf-core pipeline catalog at <https://nf-co.re>
//! and GitHub API for detailed pipeline information.

use serde::{Deserialize, Deserializer, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

fn catalog_url() -> String {
    config::resolve_url("nfcore_catalog", "https://nf-co.re/pipelines.json")
}

fn github_api_url() -> String {
    config::resolve_url("nfcore_github", "https://api.github.com")
}

fn raw_github_url() -> String {
    config::resolve_url("nfcore_raw", "https://raw.githubusercontent.com")
}

/// nf-core pipeline catalog client.
pub struct NfCoreClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub description: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub topics: Vec<String>,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    pub releases: Vec<Release>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    #[serde(default, deserialize_with = "null_as_default")]
    pub tag_name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDetail {
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub full_name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub description: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub topics: Vec<String>,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    pub html_url: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub default_branch: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub created_at: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub updated_at: String,
    #[serde(default)]
    pub open_issues_count: u64,
    #[serde(default)]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    #[serde(default, deserialize_with = "null_as_default")]
    pub spdx_id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: String,
}

/// Wrapper around the raw nextflow_schema.json content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineParams {
    #[serde(flatten)]
    pub schema: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl NfCoreClient {
    pub fn new() -> Self {
        let mut base = BaseClient::new();
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                base.set_api_key("GITHUB_TOKEN", token);
            }
        }
        NfCoreClient { base }
    }

    pub fn with_client(base: BaseClient) -> Self {
        NfCoreClient { base }
    }

    /// Build headers for GitHub API requests.
    fn github_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Accept".to_string(), "application/vnd.github.v3+json".to_string()),
            ("User-Agent".to_string(), "bio-apis/0.1".to_string()),
        ];
        if let Some(token) = self.base.get_api_key("GITHUB_TOKEN") {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        headers
    }

    fn github_get_json(&self, url: &str) -> Result<serde_json::Value> {
        let owned = self.github_headers();
        let refs: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        self.base.get_json_with_headers(url, &refs)
    }

    /// Fetch the full nf-core pipeline catalog, excluding archived pipelines.
    pub fn list_pipelines(&self) -> Result<Vec<PipelineSummary>> {
        let catalog = catalog_url();
        let json = self.base.get_json(&catalog)?;

        // The catalog may be a top-level array or an object with a
        // "remote_workflows" key.
        let arr = if let Some(a) = json.as_array() {
            a.clone()
        } else if let Some(a) = json.get("remote_workflows").and_then(|v| v.as_array()) {
            a.clone()
        } else {
            return Err(ApiError::Parse {
                context: "nf-core pipelines.json".into(),
                source: "expected array or object with remote_workflows key".into(),
            });
        };

        let mut pipelines = Vec::new();
        for item in &arr {
            let p: PipelineSummary = serde_json::from_value(item.clone()).map_err(|e| {
                ApiError::Parse {
                    context: "nf-core pipeline entry".into(),
                    source: e.to_string(),
                }
            })?;
            if !p.archived {
                pipelines.push(p);
            }
        }
        Ok(pipelines)
    }

    /// Search pipelines by name, description, or topics (case-insensitive).
    pub fn search_pipelines(&self, query: &str) -> Result<Vec<PipelineSummary>> {
        let all = self.list_pipelines()?;
        let q = query.to_lowercase();
        let filtered = all
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.topics.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect();
        Ok(filtered)
    }

    /// Fetch detailed pipeline info from GitHub API.
    pub fn pipeline_info(&self, name: &str) -> Result<PipelineDetail> {
        let github_api = github_api_url();
        let url = format!("{github_api}/repos/nf-core/{name}");
        let json = self.github_get_json(&url)?;

        // Parse license sub-object
        let license = json.get("license").and_then(|l| {
            if l.is_null() {
                None
            } else {
                Some(License {
                    spdx_id: l["spdx_id"].as_str().unwrap_or_default().to_string(),
                    name: l["name"].as_str().unwrap_or_default().to_string(),
                })
            }
        });

        Ok(PipelineDetail {
            name: json["name"].as_str().unwrap_or(name).to_string(),
            full_name: json["full_name"].as_str().unwrap_or_default().to_string(),
            description: json["description"].as_str().unwrap_or_default().to_string(),
            topics: json["topics"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            stargazers_count: json["stargazers_count"].as_u64().unwrap_or(0),
            archived: json["archived"].as_bool().unwrap_or(false),
            html_url: json["html_url"].as_str().unwrap_or_default().to_string(),
            default_branch: json["default_branch"].as_str().unwrap_or("master").to_string(),
            created_at: json["created_at"].as_str().unwrap_or_default().to_string(),
            updated_at: json["updated_at"].as_str().unwrap_or_default().to_string(),
            open_issues_count: json["open_issues_count"].as_u64().unwrap_or(0),
            license,
        })
    }

    /// Fetch releases for a pipeline from GitHub API.
    pub fn pipeline_releases(&self, name: &str) -> Result<Vec<Release>> {
        let github_api = github_api_url();
        let url = format!("{github_api}/repos/nf-core/{name}/releases");
        let json = self.github_get_json(&url)?;

        let arr = json.as_array().ok_or_else(|| ApiError::Parse {
            context: format!("nf-core {name} releases"),
            source: "expected array".into(),
        })?;

        let mut releases = Vec::new();
        for item in arr {
            releases.push(Release {
                tag_name: item["tag_name"].as_str().unwrap_or_default().to_string(),
                published_at: item["published_at"].as_str().unwrap_or_default().to_string(),
            });
        }
        Ok(releases)
    }

    /// Fetch the nextflow_schema.json (parameter definitions) for a pipeline.
    pub fn pipeline_params(&self, name: &str) -> Result<PipelineParams> {
        let raw_github = raw_github_url();
        let url = format!("{raw_github}/nf-core/{name}/master/nextflow_schema.json");
        let json = self.base.get_json(&url)?;
        Ok(PipelineParams { schema: json })
    }
}

impl Default for NfCoreClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_summary_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "name": "rnaseq",
                "description": "RNA sequencing analysis pipeline",
                "topics": ["rna", "rnaseq", "nextflow"],
                "stargazers_count": 800,
                "archived": false,
                "releases": [
                    {"tag_name": "3.14.0", "published_at": "2024-01-15T00:00:00Z"},
                    {"tag_name": "3.13.0", "published_at": "2023-10-01T00:00:00Z"}
                ]
            }"#,
        )
        .unwrap();
        let p: PipelineSummary = serde_json::from_value(json).unwrap();
        assert_eq!(p.name, "rnaseq");
        assert_eq!(p.stargazers_count, 800);
        assert_eq!(p.releases.len(), 2);
        assert_eq!(p.releases[0].tag_name, "3.14.0");
        assert!(!p.archived);
    }

    #[test]
    fn test_pipeline_summary_defaults() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let p: PipelineSummary = serde_json::from_value(json).unwrap();
        assert_eq!(p.name, "");
        assert_eq!(p.stargazers_count, 0);
        assert!(p.topics.is_empty());
        assert!(p.releases.is_empty());
        assert!(!p.archived);
    }

    #[test]
    fn test_pipeline_summary_null_fields() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"name": "test", "description": null, "topics": null, "stargazers_count": 0, "archived": false, "releases": null}"#,
        )
        .unwrap();
        let p: PipelineSummary = serde_json::from_value(json).unwrap();
        assert_eq!(p.name, "test");
        assert_eq!(p.description, "");
        assert!(p.topics.is_empty());
        assert!(p.releases.is_empty());
    }

    #[test]
    fn test_release_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"tag_name": "2.0.0", "published_at": "2024-06-01T12:00:00Z"}"#,
        )
        .unwrap();
        let r: Release = serde_json::from_value(json).unwrap();
        assert_eq!(r.tag_name, "2.0.0");
        assert_eq!(r.published_at, "2024-06-01T12:00:00Z");
    }

    #[test]
    fn test_pipeline_params_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "$schema": "http://json-schema.org/draft-07/schema",
                "title": "nf-core/rnaseq",
                "definitions": {
                    "input_output_options": {
                        "properties": {
                            "input": {
                                "type": "string",
                                "description": "Path to input samplesheet"
                            }
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let params: PipelineParams = serde_json::from_value(json).unwrap();
        assert!(params.schema.get("definitions").is_some());
    }

    #[test]
    fn test_license_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"spdx_id": "MIT", "name": "MIT License"}"#,
        )
        .unwrap();
        let l: License = serde_json::from_value(json).unwrap();
        assert_eq!(l.spdx_id, "MIT");
        assert_eq!(l.name, "MIT License");
    }
}
