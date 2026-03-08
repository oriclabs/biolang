//! Galaxy ToolShed client.
//!
//! Read-only access to the Galaxy ToolShed repository catalog at
//! <https://toolshed.g2.bx.psu.edu>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("galaxy_toolshed", "https://toolshed.g2.bx.psu.edu")
}

/// Galaxy ToolShed API client (read-only).
pub struct GalaxyToolShedClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type", default)]
    pub type_: String,
    #[serde(default)]
    pub remote_repository_url: String,
    #[serde(default)]
    pub homepage_url: String,
    #[serde(default)]
    pub times_downloaded: u64,
    #[serde(default)]
    pub approved: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub create_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl GalaxyToolShedClient {
    pub fn new() -> Self {
        GalaxyToolShedClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        GalaxyToolShedClient { base }
    }

    /// Search repositories by query string.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Repository>> {
        let base = base_url();
        let url = format!("{base}/api/repositories?q={query}");
        let json = self.base.get_json(&url)?;
        let mut repos = parse_repos_array(&json, &url)?;
        repos.truncate(limit);
        Ok(repos)
    }

    /// List repositories with pagination.
    pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<Repository>> {
        let base = base_url();
        // The Galaxy ToolShed API doesn't have native pagination params on
        // /api/repositories, so we fetch all and slice manually.
        let url = format!("{base}/api/repositories");
        let json = self.base.get_json(&url)?;
        let repos = parse_repos_array(&json, &url)?;
        let end = repos.len().min(offset + limit);
        let start = repos.len().min(offset);
        Ok(repos[start..end].to_vec())
    }

    /// List popular repositories sorted by download count (descending).
    pub fn popular(&self, limit: usize) -> Result<Vec<Repository>> {
        let base = base_url();
        let url = format!("{base}/api/repositories");
        let json = self.base.get_json(&url)?;
        let mut repos = parse_repos_array(&json, &url)?;
        repos.sort_by(|a, b| b.times_downloaded.cmp(&a.times_downloaded));
        repos.truncate(limit);
        Ok(repos)
    }

    /// Get repository info by owner and name.
    pub fn repository_info(&self, owner: &str, name: &str) -> Result<Repository> {
        let base = base_url();
        let url = format!("{base}/api/repositories?owner={owner}&name={name}");
        let json = self.base.get_json(&url)?;
        let repos = parse_repos_array(&json, &url)?;
        repos.into_iter().next().ok_or_else(|| ApiError::Parse {
            context: url,
            source: format!("no repository found for {owner}/{name}"),
        })
    }

    /// List tool categories.
    pub fn categories(&self) -> Result<Vec<Category>> {
        let base = base_url();
        let url = format!("{base}/api/categories");
        let json = self.base.get_json(&url)?;
        serde_json::from_value(json).map_err(|e| ApiError::Parse {
            context: url,
            source: e.to_string(),
        })
    }
}

impl Default for GalaxyToolShedClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_repos_array(json: &serde_json::Value, url: &str) -> Result<Vec<Repository>> {
    serde_json::from_value(json.clone()).map_err(|e| ApiError::Parse {
        context: url.to_string(),
        source: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "abc123",
                "name": "bwa",
                "owner": "devteam",
                "description": "BWA short read aligner",
                "type": "unrestricted",
                "remote_repository_url": "https://github.com/galaxyproject/tools-iuc",
                "homepage_url": "",
                "times_downloaded": 123456,
                "approved": "yes",
                "last_updated": "2024-01-15T10:30:00Z",
                "create_time": "2015-03-20T08:00:00Z"
            }"#,
        )
        .unwrap();
        let repo: Repository = serde_json::from_value(json).unwrap();
        assert_eq!(repo.name, "bwa");
        assert_eq!(repo.owner, "devteam");
        assert_eq!(repo.times_downloaded, 123456);
        assert_eq!(repo.type_, "unrestricted");
        assert_eq!(repo.approved, "yes");
    }

    #[test]
    fn test_repository_defaults() {
        let json: serde_json::Value = serde_json::from_str(r#"{"id": "x"}"#).unwrap();
        let repo: Repository = serde_json::from_value(json).unwrap();
        assert_eq!(repo.id, "x");
        assert_eq!(repo.name, "");
        assert_eq!(repo.times_downloaded, 0);
        assert_eq!(repo.type_, "");
    }

    #[test]
    fn test_category_deserialize() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "abc123",
                "name": "Assembly",
                "description": "Tools for genome assembly"
            }"#,
        )
        .unwrap();
        let cat: Category = serde_json::from_value(json).unwrap();
        assert_eq!(cat.name, "Assembly");
        assert_eq!(cat.description, "Tools for genome assembly");
    }

    #[test]
    fn test_category_defaults() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let cat: Category = serde_json::from_value(json).unwrap();
        assert_eq!(cat.name, "");
        assert_eq!(cat.description, "");
    }

    #[test]
    fn test_parse_repos_array() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"id": "1", "name": "bwa", "owner": "devteam"}, {"id": "2", "name": "samtools", "owner": "iuc"}]"#,
        )
        .unwrap();
        let repos = parse_repos_array(&json, "test").unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "bwa");
        assert_eq!(repos[1].name, "samtools");
    }
}
