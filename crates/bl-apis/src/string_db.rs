//! STRING protein-protein interaction database client.
//!
//! Network, interaction partners, enrichment analysis.
//! API docs: <https://string-db.org/cgi/help>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("string_db", "https://string-db.org/api")
}

/// STRING protein interaction database client.
pub struct StringDbClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    #[serde(default, alias = "preferredName_A")]
    pub protein_a: String,
    #[serde(default, alias = "preferredName_B")]
    pub protein_b: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub nscore: f64,
    #[serde(default)]
    pub fscore: f64,
    #[serde(default)]
    pub pscore: f64,
    #[serde(default)]
    pub ascore: f64,
    #[serde(default)]
    pub escore: f64,
    #[serde(default)]
    pub dscore: f64,
    #[serde(default)]
    pub tscore: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrichment {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub term: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, alias = "number_of_genes")]
    pub gene_count: u32,
    #[serde(default, alias = "p_value")]
    pub p_value: f64,
    #[serde(default)]
    pub fdr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringProtein {
    #[serde(default, alias = "preferredName")]
    pub preferred_name: String,
    #[serde(default, alias = "stringId")]
    pub string_id: String,
    #[serde(default)]
    pub annotation: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl StringDbClient {
    pub fn new() -> Self {
        StringDbClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        StringDbClient { base }
    }

    /// Get interaction network for a set of proteins.
    pub fn network(
        &self,
        identifiers: &[&str],
        species: u32,
    ) -> Result<Vec<Interaction>> {
        let base = base_url();
        let ids = identifiers.join("%0d");
        let url = format!(
            "{base}/json/network?identifiers={ids}&species={species}"
        );
        let json = self.base.get_json(&url)?;
        parse_interactions(&json)
    }

    /// Get interaction partners for a single protein.
    pub fn interaction_partners(
        &self,
        identifier: &str,
        species: u32,
        limit: usize,
    ) -> Result<Vec<Interaction>> {
        let base = base_url();
        let url = format!(
            "{base}/json/interaction_partners?identifiers={identifier}&species={species}&limit={limit}"
        );
        let json = self.base.get_json(&url)?;
        parse_interactions(&json)
    }

    /// Functional enrichment analysis.
    pub fn enrichment(
        &self,
        identifiers: &[&str],
        species: u32,
    ) -> Result<Vec<Enrichment>> {
        let base = base_url();
        let ids = identifiers.join("%0d");
        let url = format!(
            "{base}/json/enrichment?identifiers={ids}&species={species}"
        );
        let json = self.base.get_json(&url)?;
        let arr = json.as_array().ok_or_else(|| ApiError::Parse {
            context: "STRING enrichment".into(),
            source: "expected array".into(),
        })?;
        let mut results = Vec::new();
        for item in arr {
            results.push(Enrichment {
                category: item["category"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                term: item["term"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                description: item["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                gene_count: item["number_of_genes"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                p_value: item["p_value"]
                    .as_f64()
                    .unwrap_or(1.0),
                fdr: item["fdr"]
                    .as_f64()
                    .unwrap_or(1.0),
            });
        }
        Ok(results)
    }

    /// Resolve protein identifier to STRING ID.
    pub fn resolve(
        &self,
        identifier: &str,
        species: u32,
    ) -> Result<Vec<StringProtein>> {
        let base = base_url();
        let url = format!(
            "{base}/json/resolve?identifier={identifier}&species={species}"
        );
        let json = self.base.get_json(&url)?;
        let arr = json.as_array().ok_or_else(|| ApiError::Parse {
            context: "STRING resolve".into(),
            source: "expected array".into(),
        })?;
        let mut results = Vec::new();
        for item in arr {
            results.push(StringProtein {
                preferred_name: item["preferredName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                string_id: item["stringId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                annotation: item["annotation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        Ok(results)
    }
}

impl Default for StringDbClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_interactions(json: &serde_json::Value) -> Result<Vec<Interaction>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "STRING network".into(),
        source: "expected array".into(),
    })?;
    let mut interactions = Vec::new();
    for item in arr {
        interactions.push(Interaction {
            protein_a: item["preferredName_A"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            protein_b: item["preferredName_B"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            score: item["score"].as_f64().unwrap_or(0.0),
            nscore: item["nscore"].as_f64().unwrap_or(0.0),
            fscore: item["fscore"].as_f64().unwrap_or(0.0),
            pscore: item["pscore"].as_f64().unwrap_or(0.0),
            ascore: item["ascore"].as_f64().unwrap_or(0.0),
            escore: item["escore"].as_f64().unwrap_or(0.0),
            dscore: item["dscore"].as_f64().unwrap_or(0.0),
            tscore: item["tscore"].as_f64().unwrap_or(0.0),
        });
    }
    Ok(interactions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interactions() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{
                "preferredName_A": "TP53",
                "preferredName_B": "MDM2",
                "score": 0.999,
                "nscore": 0.0,
                "fscore": 0.0,
                "pscore": 0.0,
                "ascore": 0.0,
                "escore": 0.943,
                "dscore": 0.0,
                "tscore": 0.989
            }]"#,
        )
        .unwrap();
        let interactions = parse_interactions(&json).unwrap();
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].protein_a, "TP53");
        assert_eq!(interactions[0].protein_b, "MDM2");
        assert!((interactions[0].score - 0.999).abs() < 0.001);
        assert!((interactions[0].escore - 0.943).abs() < 0.001);
    }

    #[test]
    fn test_parse_interactions_not_array() {
        let json: serde_json::Value = serde_json::from_str(r#"{"error": "bad"}"#).unwrap();
        assert!(parse_interactions(&json).is_err());
    }

    #[test]
    fn test_parse_interactions_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"[]"#).unwrap();
        let interactions = parse_interactions(&json).unwrap();
        assert!(interactions.is_empty());
    }

    #[test]
    fn test_interaction_missing_scores() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"preferredName_A": "A", "preferredName_B": "B"}]"#,
        )
        .unwrap();
        let interactions = parse_interactions(&json).unwrap();
        assert_eq!(interactions[0].score, 0.0);
    }

    #[test]
    fn test_parse_interactions_many() {
        let mut items = Vec::new();
        for i in 0..100 {
            items.push(format!(
                r#"{{"preferredName_A": "P{i}", "preferredName_B": "Q{i}", "score": 0.{i:03}}}"#,
                i = i
            ));
        }
        let text = format!("[{}]", items.join(","));
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let interactions = parse_interactions(&json).unwrap();
        assert_eq!(interactions.len(), 100);
        assert_eq!(interactions[0].protein_a, "P0");
        assert_eq!(interactions[99].protein_a, "P99");
    }

    #[test]
    fn test_parse_interactions_nan_scores_default_to_zero() {
        // JSON doesn't support NaN, so null or string "NaN" should be handled
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"preferredName_A": "A", "preferredName_B": "B", "score": null, "nscore": null}]"#,
        )
        .unwrap();
        let interactions = parse_interactions(&json).unwrap();
        assert_eq!(interactions[0].score, 0.0);
        assert_eq!(interactions[0].nscore, 0.0);
    }
}
