//! RCSB PDB (Protein Data Bank) API client.
//!
//! Structure entries, entities, sequence annotations.
//! API docs: <https://data.rcsb.org/redoc/index.html>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn data_base_url() -> String {
    config::resolve_url("pdb_data", "https://data.rcsb.org/rest/v1/core")
}

fn search_base_url() -> String {
    config::resolve_url("pdb_search", "https://search.rcsb.org/rcsbsearch/v2/query")
}

/// RCSB PDB API client.
pub struct PdbClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub resolution: Option<f64>,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub organism: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbEntity {
    pub entity_id: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entity_type: String,
    #[serde(default)]
    pub sequence: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl PdbClient {
    pub fn new() -> Self {
        PdbClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        PdbClient { base }
    }

    /// Get a PDB entry by ID (e.g. "4HHB").
    pub fn entry(&self, pdb_id: &str) -> Result<PdbEntry> {
        let data_base = data_base_url();
        let id = pdb_id.to_uppercase();
        let url = format!("{data_base}/entry/{id}");
        let json = self.base.get_json(&url)?;

        Ok(PdbEntry {
            id: id.clone(),
            title: json["struct"]["title"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            method: json["exptl"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|e| e["method"].as_str())
                .unwrap_or_default()
                .to_string(),
            resolution: json["rcsb_entry_info"]["resolution_combined"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_f64()),
            release_date: json["rcsb_accession_info"]["initial_release_date"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            organism: json["rcsb_entry_info"]["polymer_entity_count_protein"]
                .as_u64()
                .map(|_| {
                    // Get organism from polymer entities
                    json["polymer_entities"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|e| {
                            e["rcsb_entity_source_organism"]
                                .as_array()
                                .and_then(|o| o.first())
                                .and_then(|o| o["scientific_name"].as_str())
                        })
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_default(),
        })
    }

    /// Get a specific entity within a PDB entry.
    pub fn entity(&self, pdb_id: &str, entity_id: u32) -> Result<PdbEntity> {
        let data_base = data_base_url();
        let id = pdb_id.to_uppercase();
        let url = format!("{data_base}/polymer_entity/{id}/{entity_id}");
        let json = self.base.get_json(&url)?;

        Ok(PdbEntity {
            entity_id,
            description: json["rcsb_polymer_entity"]["pdbx_description"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            entity_type: json["entity_poly"]["type"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            sequence: json["entity_poly"]["pdbx_seq_one_letter_code_can"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// Full-text search, returns PDB IDs.
    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let body = serde_json::json!({
            "query": {
                "type": "terminal",
                "service": "full_text",
                "parameters": {
                    "value": query
                }
            },
            "return_type": "entry",
            "request_options": {
                "results_content_type": ["experimental"],
                "paginate": {
                    "start": 0,
                    "rows": 25
                }
            }
        });
        let search_base = search_base_url();
        let json = self.base.post_json(&search_base, &body)?;

        let ids: Vec<String> = json["result_set"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r["identifier"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(ids)
    }

    /// Get sequence for a specific entity.
    pub fn sequence(&self, pdb_id: &str, entity_id: u32) -> Result<String> {
        let ent = self.entity(pdb_id, entity_id)?;
        if ent.sequence.is_empty() {
            Err(ApiError::Parse {
                context: format!("PDB {pdb_id} entity {entity_id}"),
                source: "no sequence available".into(),
            })
        } else {
            Ok(ent.sequence)
        }
    }
}

impl Default for PdbClient {
    fn default() -> Self {
        Self::new()
    }
}
