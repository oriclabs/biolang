//! COSMIC (Catalogue Of Somatic Mutations In Cancer) client.
//!
//! Requires `COSMIC_API_KEY` environment variable.
//! API docs: <https://cancer.sanger.ac.uk/cosmic/help/api>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("cosmic", "https://cancer.sanger.ac.uk/api/v1")
}

/// COSMIC API client (requires API key).
pub struct CosmicClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicMutation {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub gene: String,
    #[serde(default)]
    pub cds: String,
    #[serde(default)]
    pub aa: String,
    #[serde(default)]
    pub primary_site: String,
    #[serde(default)]
    pub primary_histology: String,
    #[serde(default)]
    pub mutation_type: String,
    #[serde(default)]
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensusGene {
    #[serde(default)]
    pub gene_symbol: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role_in_cancer: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub hallmark: bool,
    #[serde(default)]
    pub somatic: bool,
    #[serde(default)]
    pub germline: bool,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl CosmicClient {
    pub fn new() -> Result<Self> {
        let base = BaseClient::new();
        if base.get_api_key("COSMIC_API_KEY").is_none() {
            return Err(ApiError::Auth(
                "COSMIC_API_KEY environment variable not set".into(),
            ));
        }
        Ok(CosmicClient { base })
    }

    pub fn with_client(base: BaseClient) -> Result<Self> {
        if base.get_api_key("COSMIC_API_KEY").is_none() {
            return Err(ApiError::Auth(
                "COSMIC_API_KEY not configured".into(),
            ));
        }
        Ok(CosmicClient { base })
    }

    fn auth_headers(&self) -> Vec<(&str, String)> {
        let key = self
            .base
            .get_api_key("COSMIC_API_KEY")
            .unwrap_or_default()
            .to_string();
        vec![("Authorization", format!("Basic {key}"))]
    }

    /// Search mutations by gene name.
    pub fn search_gene(&self, gene: &str) -> Result<Vec<CosmicMutation>> {
        let auth = self.auth_headers();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let base = base_url();
        let url = format!("{base}/mutations/gene/{gene}");
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        parse_mutations(&json)
    }

    /// Get a specific mutation by COSMIC ID.
    pub fn get_mutation(&self, mutation_id: &str) -> Result<CosmicMutation> {
        let auth = self.auth_headers();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let base = base_url();
        let url = format!("{base}/mutations/{mutation_id}");
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        let mutations = parse_mutations(&json)?;
        mutations.into_iter().next().ok_or_else(|| ApiError::Parse {
            context: "COSMIC mutation".into(),
            source: format!("no mutation found for {mutation_id}"),
        })
    }

    /// Get Cancer Gene Census entries.
    pub fn gene_census(&self) -> Result<Vec<CensusGene>> {
        let auth = self.auth_headers();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let base = base_url();
        let url = format!("{base}/census/genes");
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        let arr = json.as_array().ok_or_else(|| ApiError::Parse {
            context: "COSMIC census".into(),
            source: "expected array".into(),
        })?;

        let mut genes = Vec::new();
        for item in arr {
            genes.push(CensusGene {
                gene_symbol: item["gene_symbol"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: item["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                role_in_cancer: item["role_in_cancer"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                tier: item["tier"].as_u64().unwrap_or(0) as u32,
                hallmark: item["hallmark"].as_bool().unwrap_or(false),
                somatic: item["somatic"].as_bool().unwrap_or(false),
                germline: item["germline"].as_bool().unwrap_or(false),
            });
        }
        Ok(genes)
    }
}

fn parse_mutations(json: &serde_json::Value) -> Result<Vec<CosmicMutation>> {
    let arr = if json.is_array() {
        json.as_array().unwrap()
    } else if json.is_object() {
        // Single-item response
        return Ok(vec![parse_single_mutation(json)]);
    } else {
        return Err(ApiError::Parse {
            context: "COSMIC mutations".into(),
            source: "expected array or object".into(),
        });
    };

    Ok(arr.iter().map(parse_single_mutation).collect())
}

fn parse_single_mutation(item: &serde_json::Value) -> CosmicMutation {
    CosmicMutation {
        id: item["id"]
            .as_str()
            .or_else(|| item["COSMIC_ID"].as_str())
            .unwrap_or_default()
            .to_string(),
        gene: item["gene"]
            .as_str()
            .or_else(|| item["gene_name"].as_str())
            .unwrap_or_default()
            .to_string(),
        cds: item["cds"]
            .as_str()
            .or_else(|| item["CDS_MUT_SYNTAX"].as_str())
            .unwrap_or_default()
            .to_string(),
        aa: item["aa"]
            .as_str()
            .or_else(|| item["AA_MUT_SYNTAX"].as_str())
            .unwrap_or_default()
            .to_string(),
        primary_site: item["primary_site"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        primary_histology: item["primary_histology"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        mutation_type: item["mutation_type"]
            .as_str()
            .or_else(|| item["MUTATION_DESCRIPTION"].as_str())
            .unwrap_or_default()
            .to_string(),
        count: item["count"]
            .as_u64()
            .or_else(|| item["MUTATION_TOTAL"].as_u64())
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_mutation_standard_fields() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "COSM476",
                "gene": "BRAF",
                "cds": "c.1799T>A",
                "aa": "p.V600E",
                "primary_site": "thyroid",
                "primary_histology": "carcinoma",
                "mutation_type": "Substitution - Missense",
                "count": 48238
            }"#,
        )
        .unwrap();
        let m = parse_single_mutation(&json);
        assert_eq!(m.id, "COSM476");
        assert_eq!(m.gene, "BRAF");
        assert_eq!(m.cds, "c.1799T>A");
        assert_eq!(m.aa, "p.V600E");
        assert_eq!(m.primary_site, "thyroid");
        assert_eq!(m.mutation_type, "Substitution - Missense");
        assert_eq!(m.count, 48238);
    }

    #[test]
    fn test_parse_single_mutation_alternate_fields() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "COSMIC_ID": "COSM12345",
                "gene_name": "TP53",
                "CDS_MUT_SYNTAX": "c.743G>A",
                "AA_MUT_SYNTAX": "p.R248Q",
                "MUTATION_DESCRIPTION": "Substitution - Missense",
                "MUTATION_TOTAL": 5000
            }"#,
        )
        .unwrap();
        let m = parse_single_mutation(&json);
        assert_eq!(m.id, "COSM12345");
        assert_eq!(m.gene, "TP53");
        assert_eq!(m.cds, "c.743G>A");
        assert_eq!(m.aa, "p.R248Q");
        assert_eq!(m.mutation_type, "Substitution - Missense");
        assert_eq!(m.count, 5000);
    }

    #[test]
    fn test_parse_mutations_array() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[
                {"id": "COSM1", "gene": "BRAF", "cds": "", "aa": "", "count": 10},
                {"id": "COSM2", "gene": "BRAF", "cds": "", "aa": "", "count": 20}
            ]"#,
        )
        .unwrap();
        let mutations = parse_mutations(&json).unwrap();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[0].id, "COSM1");
        assert_eq!(mutations[1].id, "COSM2");
    }

    #[test]
    fn test_parse_mutations_single_object() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"id": "COSM476", "gene": "BRAF", "cds": "c.1799T>A", "aa": "p.V600E"}"#,
        )
        .unwrap();
        let mutations = parse_mutations(&json).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].id, "COSM476");
    }

    #[test]
    fn test_parse_mutations_invalid() {
        let json: serde_json::Value = serde_json::from_str(r#""string_value""#).unwrap();
        assert!(parse_mutations(&json).is_err());
    }

    #[test]
    fn test_parse_single_mutation_all_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let m = parse_single_mutation(&json);
        assert_eq!(m.id, "");
        assert_eq!(m.gene, "");
        assert_eq!(m.cds, "");
        assert_eq!(m.aa, "");
        assert_eq!(m.primary_site, "");
        assert_eq!(m.primary_histology, "");
        assert_eq!(m.mutation_type, "");
        assert_eq!(m.count, 0);
    }

    #[test]
    fn test_parse_mutations_large_count() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"id": "COSM1", "gene": "TP53", "count": 9999999999}]"#,
        )
        .unwrap();
        let mutations = parse_mutations(&json).unwrap();
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].count, 9999999999);
    }
}
