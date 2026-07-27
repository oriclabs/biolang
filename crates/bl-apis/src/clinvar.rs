//! ClinVar API client using NCBI E-utilities.
//!
//! API docs: <https://www.ncbi.nlm.nih.gov/clinvar/docs/api_http/>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url(
        "ncbi_eutils",
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils",
    )
}

/// ClinVar client using NCBI E-utilities (db=clinvar).
pub struct ClinVarClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClinVarVariant {
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub variation_name: String,
    #[serde(default)]
    pub gene: String,
    #[serde(default)]
    pub clinical_significance: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub accessions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClinVarSubmission {
    #[serde(default)]
    pub accession: String,
    #[serde(default)]
    pub submitter: String,
    #[serde(default)]
    pub classification: String,
    #[serde(default)]
    pub date: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl ClinVarClient {
    pub fn new() -> Self {
        ClinVarClient {
            base: BaseClient::new(),
        }
    }

    fn api_key_param(&self) -> String {
        match self.base.get_api_key("NCBI_API_KEY") {
            Some(key) => format!("&api_key={key}"),
            None => String::new(),
        }
    }

    /// Search ClinVar and return up to `max` variants.
    pub fn search(&self, query: &str, max: usize) -> Result<Vec<ClinVarVariant>> {
        let base = base_url();
        let ak = self.api_key_param();
        let encoded_query = query.replace(' ', "+");
        let search_url = format!(
            "{base}/esearch.fcgi?db=clinvar&term={encoded_query}&retmax={max}&retmode=json{ak}"
        );

        let search_json = self.base.get_json(&search_url)?;
        let ids = extract_ids(&search_json)?;
        if ids.is_empty() {
            return Ok(vec![]);
        }

        self.fetch_summaries(&ids)
    }

    /// Get a single variant by ClinVar variation ID.
    pub fn variant(&self, variation_id: &str) -> Result<ClinVarVariant> {
        let variants = self.fetch_summaries(&[variation_id.to_string()])?;
        variants.into_iter().next().ok_or_else(|| ApiError::Parse {
            context: "ClinVar variant".into(),
            source: format!("no record found for variation_id={variation_id}"),
        })
    }

    /// Search variants for a gene symbol.
    pub fn gene_variants(&self, gene: &str, max: usize) -> Result<Vec<ClinVarVariant>> {
        self.search(&format!("{gene}[gene]"), max)
    }

    /// Get pathogenic/likely pathogenic variants for a gene.
    pub fn pathogenic(&self, gene: &str, max: usize) -> Result<Vec<ClinVarVariant>> {
        self.search(
            &format!("{gene}[gene] AND (pathogenic[clinsig] OR likely+pathogenic[clinsig])"),
            max,
        )
    }

    fn fetch_summaries(&self, ids: &[String]) -> Result<Vec<ClinVarVariant>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let base = base_url();
        let ak = self.api_key_param();
        let id_str = ids.join(",");
        let url = format!(
            "{base}/esummary.fcgi?db=clinvar&id={id_str}&retmode=json{ak}"
        );

        let json = self.base.get_json(&url)?;
        parse_summaries(&json, ids)
    }
}

impl Default for ClinVarClient {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_ids(json: &serde_json::Value) -> Result<Vec<String>> {
    let ids = json
        .pointer("/esearchresult/idlist")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::Parse {
            context: "ClinVar esearch".into(),
            source: "missing /esearchresult/idlist".into(),
        })?;

    Ok(ids
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect())
}

fn parse_summaries(json: &serde_json::Value, ids: &[String]) -> Result<Vec<ClinVarVariant>> {
    let result = json.get("result").ok_or_else(|| ApiError::Parse {
        context: "ClinVar esummary".into(),
        source: "missing 'result' key".into(),
    })?;

    let mut variants = Vec::new();
    for id in ids {
        let entry = match result.get(id.as_str()) {
            Some(e) => e,
            None => continue,
        };

        // variation name: prefer variation_set[0].variation_name
        let variation_name = entry["variation_set"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v["variation_name"].as_str())
            .unwrap_or_else(|| entry["title"].as_str().unwrap_or_default())
            .to_string();

        let gene = entry["gene_sort"].as_str().unwrap_or_default().to_string();

        let clinsig = &entry["clinical_significance"];
        let clinical_significance = clinsig["description"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let review_status = clinsig["review_status"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let conditions = entry["trait_set"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["trait_name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let accessions = entry["supporting_submissions"]
            .get("rcv")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        variants.push(ClinVarVariant {
            uid: id.clone(),
            variation_name,
            gene,
            clinical_significance,
            review_status,
            conditions,
            accessions,
        });
    }

    Ok(variants)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ids() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "esearchresult": {
                "count": "2",
                "retmax": "2",
                "idlist": ["12375", "18390"]
            }
        }"#).unwrap();
        let ids = extract_ids(&json).unwrap();
        assert_eq!(ids, vec!["12375", "18390"]);
    }

    #[test]
    fn test_extract_ids_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "esearchresult": { "count": "0", "idlist": [] }
        }"#).unwrap();
        let ids = extract_ids(&json).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_parse_summaries_basic() {
        let ids = vec!["12375".to_string()];
        let json: serde_json::Value = serde_json::from_str(r#"{
            "result": {
                "12375": {
                    "gene_sort": "BRCA1",
                    "clinical_significance": {
                        "description": "Pathogenic",
                        "review_status": "reviewed by expert panel"
                    },
                    "trait_set": [
                        { "trait_name": "Hereditary breast ovarian cancer" }
                    ],
                    "variation_set": [
                        { "variation_name": "NM_007294.4(BRCA1):c.5266dupC (p.Gln1756ProfsTer74)" }
                    ],
                    "supporting_submissions": {
                        "rcv": ["RCV000012345", "RCV000067890"]
                    }
                }
            }
        }"#).unwrap();

        let variants = parse_summaries(&json, &ids).unwrap();
        assert_eq!(variants.len(), 1);
        let v = &variants[0];
        assert_eq!(v.uid, "12375");
        assert_eq!(v.gene, "BRCA1");
        assert_eq!(v.clinical_significance, "Pathogenic");
        assert_eq!(v.review_status, "reviewed by expert panel");
        assert_eq!(v.conditions, vec!["Hereditary breast ovarian cancer"]);
        assert_eq!(v.accessions, vec!["RCV000012345", "RCV000067890"]);
        assert!(v.variation_name.contains("BRCA1"));
    }

    #[test]
    fn test_parse_summaries_missing_uid() {
        let ids = vec!["99999".to_string()];
        let json: serde_json::Value = serde_json::from_str(r#"{
            "result": { "uids": [] }
        }"#).unwrap();
        let variants = parse_summaries(&json, &ids).unwrap();
        assert!(variants.is_empty());
    }

    #[test]
    fn test_parse_summaries_multiple() {
        let ids = vec!["1".to_string(), "2".to_string()];
        let json: serde_json::Value = serde_json::from_str(r#"{
            "result": {
                "1": {
                    "gene_sort": "TP53",
                    "clinical_significance": { "description": "Pathogenic", "review_status": "criteria provided" },
                    "trait_set": [],
                    "variation_set": [{ "variation_name": "TP53 c.817C>T" }],
                    "supporting_submissions": {}
                },
                "2": {
                    "gene_sort": "KRAS",
                    "clinical_significance": { "description": "Likely pathogenic", "review_status": "criteria provided" },
                    "trait_set": [{ "trait_name": "RASopathy" }],
                    "variation_set": [{ "variation_name": "KRAS c.38G>A" }],
                    "supporting_submissions": { "rcv": ["RCV000001"] }
                }
            }
        }"#).unwrap();

        let variants = parse_summaries(&json, &ids).unwrap();
        assert_eq!(variants.len(), 2);
        let tp53 = variants.iter().find(|v| v.gene == "TP53").unwrap();
        let kras = variants.iter().find(|v| v.gene == "KRAS").unwrap();
        assert_eq!(tp53.clinical_significance, "Pathogenic");
        assert_eq!(kras.conditions, vec!["RASopathy"]);
    }

    #[test]
    fn test_default_client_constructs() {
        let client = ClinVarClient::default();
        let _ = client;
    }
}
