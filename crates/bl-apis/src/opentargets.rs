//! Open Targets Platform API client.
//!
//! Uses the GraphQL endpoint at <https://api.platform.opentargets.org/api/v4/graphql>.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url(
        "opentargets",
        "https://api.platform.opentargets.org/api/v4/graphql",
    )
}

/// Open Targets Platform GraphQL client.
pub struct OpenTargetsClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub approved_symbol: String,
    #[serde(default)]
    pub approved_name: String,
    #[serde(default)]
    pub biotype: String,
    #[serde(default)]
    pub chromosome: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiseaseAssociation {
    #[serde(default)]
    pub disease_id: String,
    #[serde(default)]
    pub disease_name: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub datatypes_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrugRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub molecule_type: String,
    #[serde(default)]
    pub max_clinical_trial_phase: i32,
    #[serde(default)]
    pub indication: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SafetyEvent {
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl OpenTargetsClient {
    pub fn new() -> Self {
        OpenTargetsClient {
            base: BaseClient::new(),
        }
    }

    fn gql<T: for<'de> serde::Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
        path: &[&str],
    ) -> Result<T> {
        let url = base_url();
        let body = json!({ "query": query, "variables": variables });
        let resp = self.base.post_json(&url, &body)?;

        // Navigate the data path
        let mut cur = &resp;
        for key in path {
            cur = cur.get(key).ok_or_else(|| ApiError::Parse {
                context: "OpenTargets GraphQL".into(),
                source: format!("missing key '{key}' in response"),
            })?;
        }

        serde_json::from_value(cur.clone()).map_err(|e| ApiError::Parse {
            context: "OpenTargets GraphQL".into(),
            source: e.to_string(),
        })
    }

    /// Fetch basic target information by Ensembl gene ID.
    pub fn target(&self, gene_id: &str) -> Result<TargetInfo> {
        let query = r#"
            query Target($id: String!) {
                target(ensemblId: $id) {
                    id
                    approvedSymbol
                    approvedName
                    biotype
                    genomicLocation { chromosome }
                    functionDescriptions
                }
            }
        "#;
        let resp = self.gql::<serde_json::Value>(
            query,
            json!({ "id": gene_id }),
            &["data", "target"],
        )?;

        Ok(TargetInfo {
            id: resp["id"].as_str().unwrap_or_default().to_string(),
            approved_symbol: resp["approvedSymbol"].as_str().unwrap_or_default().to_string(),
            approved_name: resp["approvedName"].as_str().unwrap_or_default().to_string(),
            biotype: resp["biotype"].as_str().unwrap_or_default().to_string(),
            chromosome: resp["genomicLocation"]["chromosome"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            description: resp["functionDescriptions"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// Fetch disease associations for a gene (sorted by overall score descending).
    pub fn disease_associations(&self, gene_id: &str, max: usize) -> Result<Vec<DiseaseAssociation>> {
        let query = r#"
            query Associations($id: String!, $size: Int!) {
                target(ensemblId: $id) {
                    associatedDiseases(orderByScore: "score", page: { size: $size, index: 0 }) {
                        rows {
                            disease { id name }
                            score
                            datatypeScores { componentId score }
                        }
                    }
                }
            }
        "#;
        let resp = self.gql::<serde_json::Value>(
            query,
            json!({ "id": gene_id, "size": max }),
            &["data", "target", "associatedDiseases", "rows"],
        )?;

        let rows = resp.as_array().ok_or_else(|| ApiError::Parse {
            context: "OpenTargets associations".into(),
            source: "expected array".into(),
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(DiseaseAssociation {
                disease_id: row["disease"]["id"].as_str().unwrap_or_default().to_string(),
                disease_name: row["disease"]["name"].as_str().unwrap_or_default().to_string(),
                score: row["score"].as_f64().unwrap_or(0.0),
                datatypes_score: row["datatypeScores"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v["score"].as_f64())
                    .unwrap_or(0.0),
            });
        }
        Ok(out)
    }

    /// Fetch known drugs for a target.
    pub fn drugs(&self, gene_id: &str) -> Result<Vec<DrugRecord>> {
        let query = r#"
            query Drugs($id: String!) {
                target(ensemblId: $id) {
                    knownDrugs {
                        rows {
                            drug { id name moleculeType maxPhase }
                            disease { name }
                        }
                    }
                }
            }
        "#;
        let resp = self.gql::<serde_json::Value>(
            query,
            json!({ "id": gene_id }),
            &["data", "target", "knownDrugs", "rows"],
        )?;

        let rows = resp.as_array().ok_or_else(|| ApiError::Parse {
            context: "OpenTargets drugs".into(),
            source: "expected array".into(),
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(DrugRecord {
                id: row["drug"]["id"].as_str().unwrap_or_default().to_string(),
                name: row["drug"]["name"].as_str().unwrap_or_default().to_string(),
                molecule_type: row["drug"]["moleculeType"].as_str().unwrap_or_default().to_string(),
                max_clinical_trial_phase: row["drug"]["maxPhase"].as_i64().unwrap_or(0) as i32,
                indication: row["disease"]["name"].as_str().unwrap_or_default().to_string(),
            });
        }
        Ok(out)
    }

    /// Fetch safety events for a target.
    pub fn safety(&self, gene_id: &str) -> Result<Vec<SafetyEvent>> {
        let query = r#"
            query Safety($id: String!) {
                target(ensemblId: $id) {
                    safetyLiabilities {
                        event
                        effects { direction dosing }
                        datasource { id }
                    }
                }
            }
        "#;
        let resp = self.gql::<serde_json::Value>(
            query,
            json!({ "id": gene_id }),
            &["data", "target", "safetyLiabilities"],
        )?;

        let arr = resp.as_array().ok_or_else(|| ApiError::Parse {
            context: "OpenTargets safety".into(),
            source: "expected array".into(),
        })?;

        let mut out = Vec::new();
        for item in arr {
            let effects = item["effects"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|e| {
                            format!(
                                "{} ({})",
                                e["direction"].as_str().unwrap_or(""),
                                e["dosing"].as_str().unwrap_or("")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            let sources = item["datasource"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s["id"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            out.push(SafetyEvent {
                event: item["event"].as_str().unwrap_or_default().to_string(),
                effects,
                sources,
            });
        }
        Ok(out)
    }
}

impl Default for OpenTargetsClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_info() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "id": "ENSG00000157764",
            "approvedSymbol": "BRAF",
            "approvedName": "B-Raf proto-oncogene",
            "biotype": "protein_coding",
            "genomicLocation": { "chromosome": "7" },
            "functionDescriptions": ["Protein kinase involved in MAPK signaling"]
        }"#).unwrap();

        let info = TargetInfo {
            id: json["id"].as_str().unwrap_or_default().to_string(),
            approved_symbol: json["approvedSymbol"].as_str().unwrap_or_default().to_string(),
            approved_name: json["approvedName"].as_str().unwrap_or_default().to_string(),
            biotype: json["biotype"].as_str().unwrap_or_default().to_string(),
            chromosome: json["genomicLocation"]["chromosome"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            description: json["functionDescriptions"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        };

        assert_eq!(info.id, "ENSG00000157764");
        assert_eq!(info.approved_symbol, "BRAF");
        assert_eq!(info.biotype, "protein_coding");
        assert_eq!(info.chromosome, "7");
        assert_eq!(info.description, "Protein kinase involved in MAPK signaling");
    }

    #[test]
    fn test_parse_disease_associations() {
        let json: serde_json::Value = serde_json::from_str(r#"[
            {
                "disease": { "id": "EFO_0000616", "name": "melanoma" },
                "score": 0.85,
                "datatypeScores": [{ "componentId": "genetic_association", "score": 0.9 }]
            },
            {
                "disease": { "id": "EFO_0000305", "name": "breast carcinoma" },
                "score": 0.72,
                "datatypeScores": []
            }
        ]"#).unwrap();

        let rows = json.as_array().unwrap();
        let first = &rows[0];
        assert_eq!(first["disease"]["name"].as_str().unwrap(), "melanoma");
        assert!((first["score"].as_f64().unwrap() - 0.85).abs() < 1e-9);
    }

    #[test]
    fn test_parse_drug_record() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "drug": {
                "id": "CHEMBL1336",
                "name": "VEMURAFENIB",
                "moleculeType": "Small molecule",
                "maxPhase": 4
            },
            "disease": { "name": "melanoma" }
        }"#).unwrap();

        let rec = DrugRecord {
            id: json["drug"]["id"].as_str().unwrap_or_default().to_string(),
            name: json["drug"]["name"].as_str().unwrap_or_default().to_string(),
            molecule_type: json["drug"]["moleculeType"].as_str().unwrap_or_default().to_string(),
            max_clinical_trial_phase: json["drug"]["maxPhase"].as_i64().unwrap_or(0) as i32,
            indication: json["disease"]["name"].as_str().unwrap_or_default().to_string(),
        };

        assert_eq!(rec.id, "CHEMBL1336");
        assert_eq!(rec.name, "VEMURAFENIB");
        assert_eq!(rec.max_clinical_trial_phase, 4);
        assert_eq!(rec.indication, "melanoma");
    }

    #[test]
    fn test_parse_safety_event() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "event": "cardiotoxicity",
            "effects": [
                { "direction": "activation", "dosing": "high" }
            ],
            "datasource": [{ "id": "FDA" }]
        }"#).unwrap();

        let effects: Vec<String> = json["effects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| format!("{} ({})", e["direction"].as_str().unwrap_or(""), e["dosing"].as_str().unwrap_or("")))
            .collect();

        assert_eq!(json["event"].as_str().unwrap(), "cardiotoxicity");
        assert_eq!(effects[0], "activation (high)");
    }

    #[test]
    fn test_default_client_constructs() {
        let client = OpenTargetsClient::default();
        // Just ensure it constructs without panicking
        let _ = client;
    }
}
