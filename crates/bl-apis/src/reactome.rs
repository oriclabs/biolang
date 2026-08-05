//! Reactome Content Service client.
//!
//! Pathway search, gene-to-pathway mapping, events.
//! API docs: <https://reactome.org/ContentService/>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

/// Strip the highlight markup Reactome wraps around matched search terms.
///
/// `/search/query` returns names like
/// `<span class="highlighting" >Apoptosis</span>`, so anything printing the
/// name — a table, a report — showed raw HTML. Only the search endpoint does
/// this; the lookup endpoints return clean names.
fn strip_highlight(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn base_url() -> String {
    config::resolve_url("reactome", "https://reactome.org/ContentService")
}

/// Reactome Content Service client.
pub struct ReactomeClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactomeEntry {
    #[serde(default, alias = "stId")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, alias = "schemaClass")]
    pub schema_class: String,
    #[serde(default)]
    pub species: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pathway {
    #[serde(default, alias = "stId")]
    pub id: String,
    #[serde(default, alias = "displayName")]
    pub name: String,
    #[serde(default)]
    pub species: String,
    #[serde(default, alias = "isInDisease")]
    pub is_disease: bool,
    #[serde(default, alias = "isInferred")]
    pub is_inferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(default, alias = "stId")]
    pub id: String,
    #[serde(default, alias = "displayName")]
    pub name: String,
    #[serde(default, alias = "schemaClass")]
    pub schema_class: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl ReactomeClient {
    pub fn new() -> Self {
        ReactomeClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        ReactomeClient { base }
    }

    /// Search Reactome by query string.
    pub fn search(&self, query: &str) -> Result<Vec<ReactomeEntry>> {
        let base = base_url();
        let url = format!("{base}/search/query?query={query}&cluster=true");
        let json = self
            .base
            .get_json_with_headers(&url, &[("Accept", "application/json")])?;

        let mut entries = Vec::new();
        if let Some(groups) = json["results"].as_array() {
            for group in groups {
                if let Some(items) = group["entries"].as_array() {
                    for item in items {
                        entries.push(ReactomeEntry {
                            id: item["stId"].as_str().unwrap_or_default().to_string(),
                            name: strip_highlight(
                                item["name"].as_str().unwrap_or_default(),
                            ),
                            schema_class: item["schemaClass"]
                                .as_str()
                                .or_else(|| group["name"].as_str())
                                .unwrap_or_default()
                                .to_string(),
                            species: item["species"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|s| s.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        });
                    }
                }
            }
        }
        Ok(entries)
    }

    /// Get a pathway by stable ID.
    pub fn pathway(&self, id: &str) -> Result<Pathway> {
        let base = base_url();
        let pw_url = format!("{base}/data/query/{id}");
        let json = self
            .base
            .get_json_with_headers(&pw_url, &[("Accept", "application/json")])?;

        Ok(Pathway {
            id: json["stId"].as_str().unwrap_or(id).to_string(),
            name: json["displayName"].as_str().unwrap_or_default().to_string(),
            species: json["speciesName"].as_str().unwrap_or_default().to_string(),
            is_disease: json["isInDisease"].as_bool().unwrap_or(false),
            is_inferred: json["isInferred"].as_bool().unwrap_or(false),
        })
    }

    /// Find pathways associated with a gene (by gene name).
    pub fn pathways_for_gene(&self, gene: &str, species: &str) -> Result<Vec<Pathway>> {
        // Use the search endpoint filtered to pathways
        let base = base_url();
        let url = format!(
            "{base}/search/query?query={gene}&species={species}&types=Pathway&cluster=true"
        );
        let json = self
            .base
            .get_json_with_headers(&url, &[("Accept", "application/json")])?;

        let mut pathways = Vec::new();
        if let Some(groups) = json["results"].as_array() {
            for group in groups {
                if let Some(items) = group["entries"].as_array() {
                    for item in items {
                        pathways.push(Pathway {
                            id: item["stId"].as_str().unwrap_or_default().to_string(),
                            name: item["name"].as_str().unwrap_or_default().to_string(),
                            species: item["species"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|s| s.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            is_disease: false,
                            is_inferred: false,
                        });
                    }
                }
            }
        }
        Ok(pathways)
    }

    /// Get events contained in a pathway.
    pub fn pathway_events(&self, id: &str) -> Result<Vec<Event>> {
        let base = base_url();
        let url = format!("{base}/data/pathway/{id}/containedEvents");
        let json = self
            .base
            .get_json_with_headers(&url, &[("Accept", "application/json")])?;

        let arr = json.as_array().ok_or_else(|| ApiError::Parse {
            context: "Reactome events".into(),
            source: "expected array".into(),
        })?;

        let mut events = Vec::new();
        for item in arr {
            events.push(Event {
                id: item["stId"].as_str().unwrap_or_default().to_string(),
                name: item["displayName"].as_str().unwrap_or_default().to_string(),
                schema_class: item["schemaClass"].as_str().unwrap_or_default().to_string(),
            });
        }
        Ok(events)
    }
}

impl Default for ReactomeClient {
    fn default() -> Self {
        Self::new()
    }
}
