//! BioContainers registry client.
//!
//! Talks to the Quay.io API at <https://quay.io/api/v1>, where the
//! BioContainers images actually live.
//!
//! This used to call the BioContainers GA4GH TRS v2 API at
//! `https://api.biocontainers.pro`. That host stopped resolving — no DNS record
//! at all, while `biocontainers.pro` itself still serves — so every lookup
//! failed with a DNS error rather than an HTTP status. The images were never
//! affected; only discovery broke.
//!
//! Quay covers what the TRS API did, across three endpoints:
//!
//! * `/find/repositories?query=biocontainers/<q>` — search. The plain
//!   repository listing ignores `query=` and returns the whole namespace, so
//!   the dedicated search endpoint is the one that filters.
//! * `/repository?namespace=biocontainers&popularity=true` — listing, and the
//!   source of the popularity figure.
//! * `/repository/biocontainers/<name>?includeTags=true` — detail and versions,
//!   where TRS had a separate `/versions` route.
//!
//! Two fields changed as a result, and the public surface follows:
//!
//! * `pulls` now carries Quay's `popularity`. It is a different measure from
//!   the TRS pull count, so treat it as a relative signal rather than a
//!   download total.
//! * `license` is gone. Quay does not carry it. BioContainers embeds a licence
//!   line in the free-text description for some tools, but parsing prose would
//!   invent a field that is sometimes wrong, which is worse than absent.

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

const NAMESPACE: &str = "biocontainers";

fn base_url() -> String {
    config::resolve_url("biocontainers", "https://quay.io/api/v1")
}

/// BioContainers registry client, backed by Quay.
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
    /// Quay's popularity score. Not a pull count; see the module docs.
    #[serde(default)]
    pub pulls: f64,
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
        let url = format!("{base}/find/repositories?query={NAMESPACE}/{query}");
        let json = self.base.get_json(&url)?;
        let results = json
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::Parse {
                context: url.clone(),
                source: "no 'results' array in the search response".into(),
            })?;
        Ok(search_hits(results, limit))
    }

    /// List tools with pagination.
    pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<Tool>> {
        // Quay paginates by page rather than offset. Convert, so callers keep
        // the offset-based signature the TRS client had.
        let page = if limit == 0 { 1 } else { offset / limit + 1 };
        let base = base_url();
        let url = format!(
            "{base}/repository?namespace={NAMESPACE}&public=true&popularity=true&limit={limit}&page={page}"
        );
        let json = self.base.get_json(&url)?;
        parse_repository_list(&json, &url, limit)
    }

    /// List popular tools, most popular first.
    pub fn popular(&self, limit: usize) -> Result<Vec<Tool>> {
        // Quay has no sort parameter, so fetch a page and order it here; 100 is
        // its maximum page size.
        let base = base_url();
        let url = format!(
            "{base}/repository?namespace={NAMESPACE}&public=true&popularity=true&limit=100"
        );
        let json = self.base.get_json(&url)?;
        let mut tools = parse_repository_list(&json, &url, usize::MAX)?;
        tools.sort_by(|a, b| {
            b.pulls
                .partial_cmp(&a.pulls)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        tools.truncate(limit);
        Ok(tools)
    }

    /// Get tool detail by name, including its versions.
    pub fn tool_info(&self, id: &str) -> Result<Tool> {
        let base = base_url();
        let url = format!("{base}/repository/{NAMESPACE}/{id}?includeTags=true");
        let json = self.base.get_json(&url)?;
        Ok(Tool {
            id: id.to_string(),
            name: json
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(id)
                .to_string(),
            description: json
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string(),
            organization: NAMESPACE.to_string(),
            pulls: json
                .get("popularity")
                .and_then(|p| p.as_f64())
                .unwrap_or(0.0),
            versions: versions_from_tags(&json, id),
            aliases: Vec::new(),
        })
    }

    /// Get versions for a tool.
    pub fn tool_versions(&self, id: &str) -> Result<Vec<ToolVersion>> {
        let base = base_url();
        let url = format!("{base}/repository/{NAMESPACE}/{id}?includeTags=true");
        let json = self.base.get_json(&url)?;
        Ok(versions_from_tags(&json, id))
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

/// Keep only repositories in the BioContainers namespace.
///
/// `/find/repositories` searches all of Quay, so a query for "samtools" also
/// returns other people's forks and the organisation entry itself.
fn search_hits(results: &[serde_json::Value], limit: usize) -> Vec<Tool> {
    results
        .iter()
        .filter(|entry| {
            entry.get("kind").and_then(|k| k.as_str()) == Some("repository")
                && entry
                    .get("namespace")
                    .and_then(|n| n.get("name"))
                    .and_then(|n| n.as_str())
                    == Some(NAMESPACE)
        })
        .take(limit)
        .map(|entry| {
            let name = entry
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            Tool {
                id: name.clone(),
                name,
                description: entry
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default()
                    .to_string(),
                organization: NAMESPACE.to_string(),
                // The search endpoint carries a relevance score, not
                // popularity. Left at zero rather than reported as a
                // popularity figure it is not.
                pulls: 0.0,
                versions: Vec::new(),
                aliases: Vec::new(),
            }
        })
        .collect()
}

/// Turn Quay's `tags` map into the version list the TRS API used to return.
/// Compare container tags the way a person reads version numbers.
///
/// Digit runs compare numerically and everything else compares as text, so
/// `1.21` sorts above `1.9` and above `1.9--h91753b0_8`. A plain string compare
/// gets this backwards, which is how the "latest version" in the docs ended up
/// being the oldest build on the shelf.
fn natural_version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_ascii_digit() && y.is_ascii_digit() {
                    let mut an = String::new();
                    while ai.peek().is_some_and(|c| c.is_ascii_digit()) {
                        an.push(ai.next().unwrap());
                    }
                    let mut bn = String::new();
                    while bi.peek().is_some_and(|c| c.is_ascii_digit()) {
                        bn.push(bi.next().unwrap());
                    }
                    // Parsed, so leading zeros do not change the value; fall
                    // back to text for numbers too long to hold.
                    let ord = match (an.parse::<u128>(), bn.parse::<u128>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        _ => an.cmp(&bn),
                    };
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    ai.next();
                    bi.next();
                    let ord = x.cmp(&y);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
            }
        }
    }
}

fn versions_from_tags(json: &serde_json::Value, tool: &str) -> Vec<ToolVersion> {
    let Some(tags) = json.get("tags").and_then(|t| t.as_object()) else {
        return Vec::new();
    };
    let mut versions: Vec<ToolVersion> = tags
        .iter()
        .map(|(tag, meta)| ToolVersion {
            id: format!("{tool}:{tag}"),
            name: tag.clone(),
            meta_version: meta
                .get("last_modified")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string()),
            images: vec![ContainerImage {
                registry_host: "quay.io".to_string(),
                image_name: format!("{NAMESPACE}/{tool}:{tag}"),
                image_type: "Docker".to_string(),
                size: meta.get("size").and_then(|s| s.as_u64()),
            }],
        })
        .collect();
    // Quay returns tags in an arbitrary order, so sort for repeatable calls —
    // highest version first, so `first()` is the latest, which is how both the
    // callers and the docs read it.
    //
    // Two orderings that look right are not. Plain alphabetical puts `1.9`
    // after `1.21`, so the first element was the *oldest* release. And
    // `last_modified` is an RFC-2822 string, so comparing it as text sorts by
    // weekday name — it reported `0.1.16` as the newest samtools build. It is
    // also the wrong question: an old release rebuilt yesterday is not the
    // latest version.
    versions.sort_by(|a, b| natural_version_cmp(&b.name, &a.name));
    versions
}

fn parse_repository_list(json: &serde_json::Value, url: &str, limit: usize) -> Result<Vec<Tool>> {
    let repos = json
        .get("repositories")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::Parse {
            context: url.to_string(),
            source: "no 'repositories' array in the response".into(),
        })?;

    Ok(repos
        .iter()
        .take(limit)
        .map(|repo| {
            let name = repo
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            Tool {
                id: name.clone(),
                name,
                description: repo
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or_default()
                    .to_string(),
                organization: NAMESPACE.to_string(),
                pulls: repo
                    .get("popularity")
                    .and_then(|p| p.as_f64())
                    .unwrap_or(0.0),
                versions: Vec::new(),
                aliases: Vec::new(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_list_maps_popularity_onto_pulls() {
        let json = serde_json::json!({
            "repositories": [
                { "namespace": "biocontainers", "name": "samtools",
                  "description": "# Samtools", "popularity": 42.5 },
                { "namespace": "biocontainers", "name": "bwa", "description": "# BWA" }
            ]
        });
        let tools = parse_repository_list(&json, "test", usize::MAX).expect("parse");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "samtools");
        assert_eq!(tools[0].pulls, 42.5);
        // Quay omits popularity on some listings rather than sending zero.
        assert_eq!(tools[1].pulls, 0.0);
    }

    #[test]
    fn tags_become_sorted_versions_with_image_uris() {
        let json = serde_json::json!({
            "name": "samtools",
            "tags": {
                "1.9--h1": { "size": 20, "last_modified": "Wed, 01 Jan 2020 00:00:00 -0000" },
                "1.10--h2": { "size": 30 }
            }
        });
        let versions = versions_from_tags(&json, "samtools");
        assert_eq!(versions.len(), 2);
        // Sorted, so two calls agree; Quay's tag order is arbitrary.
        assert_eq!(versions[0].name, "1.10--h2");
        assert_eq!(versions[1].name, "1.9--h1");
        assert_eq!(
            image_uri(&versions[1].images[0]),
            "quay.io/biocontainers/samtools:1.9--h1"
        );
        assert_eq!(versions[1].images[0].size, Some(20));
    }

    #[test]
    fn no_tags_means_no_versions() {
        let json = serde_json::json!({ "name": "samtools" });
        assert!(versions_from_tags(&json, "samtools").is_empty());
    }

    #[test]
    fn search_keeps_only_biocontainers_repositories() {
        let results = vec![
            serde_json::json!({ "kind": "repository", "name": "samtools",
                                "namespace": { "name": "biocontainers" } }),
            serde_json::json!({ "kind": "repository", "name": "samtools",
                                "namespace": { "name": "someone-else" } }),
            serde_json::json!({ "kind": "organization", "name": "biocontainers",
                                "namespace": { "name": "biocontainers" } }),
        ];
        let hits = search_hits(&results, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "samtools");
        assert_eq!(hits[0].organization, "biocontainers");
    }

    #[test]
    fn search_respects_the_limit() {
        let results: Vec<_> = (0..5)
            .map(|i| {
                serde_json::json!({ "kind": "repository", "name": format!("tool{i}"),
                                    "namespace": { "name": "biocontainers" } })
            })
            .collect();
        assert_eq!(search_hits(&results, 2).len(), 2);
    }

    #[test]
    fn missing_arrays_are_reported_rather_than_panicking() {
        let json = serde_json::json!({ "unexpected": true });
        assert!(parse_repository_list(&json, "test", 10).is_err());
    }

    #[test]
    fn image_uri_without_registry_host() {
        let img = ContainerImage {
            registry_host: String::new(),
            image_name: "biocontainers/samtools:1.19".into(),
            image_type: "Docker".into(),
            size: None,
        };
        assert_eq!(image_uri(&img), "biocontainers/samtools:1.19");
    }
}
