//! Gene Ontology client via EBI QuickGO.
//!
//! Term lookup, annotations, slimming.
//! API docs: <https://www.ebi.ac.uk/QuickGO/api/index.html>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("go", "https://www.ebi.ac.uk/QuickGO/services")
}

/// Gene Ontology (EBI QuickGO) client.
pub struct GoClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTermInfo {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub aspect: String,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub is_obsolete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    #[serde(default, alias = "goId")]
    pub go_id: String,
    #[serde(default, alias = "goName")]
    pub go_name: String,
    #[serde(default, alias = "goAspect")]
    pub aspect: String,
    #[serde(default, alias = "goEvidence")]
    pub evidence: String,
    #[serde(default)]
    pub qualifier: String,
    #[serde(default, alias = "geneProductId")]
    pub gene_product_id: String,
    #[serde(default)]
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlimResult {
    pub go_id: String,
    #[serde(default)]
    pub mapped_to: Vec<String>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl GoClient {
    pub fn new() -> Self {
        GoClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        GoClient { base }
    }

    /// Look up a GO term by ID (e.g. "GO:0008150").
    pub fn term(&self, go_id: &str) -> Result<GoTermInfo> {
        let base = base_url();
        let url = format!("{base}/ontology/go/terms/{go_id}");
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .ok_or_else(|| ApiError::Parse {
                context: "GO term".into(),
                source: "missing results".into(),
            })?;

        let item = results.first().ok_or_else(|| ApiError::Parse {
            context: "GO term".into(),
            source: format!("no term found for {go_id}"),
        })?;

        Ok(parse_term(item))
    }

    /// Get GO annotations for a gene product.
    pub fn annotations(&self, gene: &str, limit: usize) -> Result<Vec<Annotation>> {
        let base = base_url();
        let url = format!(
            "{base}/annotation/search?geneProductId={gene}&limit={limit}"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| Annotation {
                        go_id: item["goId"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        go_name: item["goName"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        aspect: item["goAspect"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        evidence: item["goEvidence"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        qualifier: item["qualifier"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        gene_product_id: item["geneProductId"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        symbol: item["symbol"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    }

    /// Search GO terms by text query.
    pub fn search(&self, query: &str) -> Result<Vec<GoTermInfo>> {
        let base = base_url();
        let url = format!(
            "{base}/ontology/go/search?query={query}&limit=25"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| arr.iter().map(parse_term).collect())
            .unwrap_or_default();
        Ok(results)
    }

    /// Get child terms of a GO term.
    pub fn children(&self, go_id: &str) -> Result<Vec<GoTermInfo>> {
        let base = base_url();
        let url = format!(
            "{base}/ontology/go/terms/{go_id}/children"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .flat_map(|item| {
                        item["children"]
                            .as_array()
                            .map(|kids| kids.iter().map(parse_term).collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    }

    /// Get parent terms of a GO term.
    pub fn parents(&self, go_id: &str) -> Result<Vec<GoTermInfo>> {
        // QuickGO doesn't have a direct parents endpoint,
        // but we can use the ancestors endpoint and filter to direct parents
        let base = base_url();
        let url = format!(
            "{base}/ontology/go/terms/{go_id}/ancestors?relations=is_a,part_of"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| arr.iter().map(parse_term).collect())
            .unwrap_or_default();
        Ok(results)
    }

    /// Get all ancestor terms of a GO term (transitive).
    pub fn ancestors(&self, go_id: &str) -> Result<Vec<GoTermInfo>> {
        let base = base_url();
        let url = format!(
            "{base}/ontology/go/terms/{go_id}/ancestors"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| arr.iter().map(parse_term).collect())
            .unwrap_or_default();
        Ok(results)
    }

    /// Get all descendant terms of a GO term (transitive).
    pub fn descendants(&self, go_id: &str) -> Result<Vec<GoTermInfo>> {
        let base = base_url();
        let url = format!(
            "{base}/ontology/go/terms/{go_id}/descendants"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| arr.iter().map(parse_term).collect())
            .unwrap_or_default();
        Ok(results)
    }

    /// Map GO terms to a slim set.
    pub fn slim(
        &self,
        go_ids: &[&str],
        slim_set: &str,
    ) -> Result<Vec<SlimResult>> {
        let base = base_url();
        let ids = go_ids.join(",");
        let url = format!(
            "{base}/ontology/go/slim/{ids}?slimsToIds={slim_set}"
        );
        let json = self.base.get_json_with_headers(
            &url,
            &[("Accept", "application/json")],
        )?;

        let results = json["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        let mapped: Vec<String> = item["slimsFromId"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        SlimResult {
                            go_id: item["slimsToIds"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string(),
                            mapped_to: mapped,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    }
}

impl Default for GoClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_term(item: &serde_json::Value) -> GoTermInfo {
    GoTermInfo {
        id: item["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        name: item["name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        aspect: item["aspect"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        definition: item["definition"]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        is_obsolete: item["isObsolete"].as_bool().unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_term_full() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "GO:0008150",
                "name": "biological_process",
                "aspect": "biological_process",
                "definition": {"text": "A biological process represents..."},
                "isObsolete": false
            }"#,
        )
        .unwrap();
        let term = parse_term(&json);
        assert_eq!(term.id, "GO:0008150");
        assert_eq!(term.name, "biological_process");
        assert_eq!(term.aspect, "biological_process");
        assert!(term.definition.starts_with("A biological"));
        assert!(!term.is_obsolete);
    }

    #[test]
    fn test_parse_term_obsolete() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"id": "GO:0000001", "name": "old term", "isObsolete": true}"#,
        )
        .unwrap();
        let term = parse_term(&json);
        assert!(term.is_obsolete);
    }

    #[test]
    fn test_parse_term_minimal() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let term = parse_term(&json);
        assert_eq!(term.id, "");
        assert_eq!(term.name, "");
        assert!(!term.is_obsolete);
    }

    #[test]
    fn test_go_term_info_serde() {
        let term = GoTermInfo {
            id: "GO:0005634".into(),
            name: "nucleus".into(),
            aspect: "cellular_component".into(),
            definition: "A membrane-bounded organelle...".into(),
            is_obsolete: false,
        };
        let json = serde_json::to_string(&term).unwrap();
        let back: GoTermInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "GO:0005634");
    }

    #[test]
    fn test_parse_term_long_definition() {
        let long_def = "A ".repeat(5000);
        let json: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"id": "GO:0000001", "name": "test", "definition": {{"text": "{}"}}}}"#,
            long_def
        ))
        .unwrap();
        let term = parse_term(&json);
        assert_eq!(term.id, "GO:0000001");
        assert_eq!(term.definition.len(), long_def.len());
    }

    #[test]
    fn test_parse_term_definition_as_string() {
        // When definition is a plain string instead of {"text": ...}
        let json: serde_json::Value = serde_json::from_str(
            r#"{"id": "GO:0000002", "name": "test", "definition": "plain string"}"#,
        )
        .unwrap();
        let term = parse_term(&json);
        assert_eq!(term.id, "GO:0000002");
        // definition["text"] on a string value returns Null, so definition defaults to ""
        assert_eq!(term.definition, "");
    }
}
