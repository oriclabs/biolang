//! NCBI Datasets v2 API client.
//!
//! Gene, taxonomy, genome assembly queries.
//! API docs: <https://www.ncbi.nlm.nih.gov/datasets/docs/v2/api/rest-api/>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("ncbi_datasets", "https://api.ncbi.nlm.nih.gov/datasets/v2")
}

/// NCBI Datasets v2 API client.
pub struct NcbiDatasetsClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetGene {
    #[serde(default)]
    pub gene_id: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub taxname: String,
    #[serde(default)]
    pub chromosome: String,
    #[serde(default)]
    pub gene_type: String,
    #[serde(default)]
    pub common_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyInfo {
    #[serde(default)]
    pub tax_id: String,
    #[serde(default)]
    pub organism_name: String,
    #[serde(default)]
    pub common_name: String,
    #[serde(default)]
    pub lineage: Vec<String>,
    #[serde(default)]
    pub rank: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeSummary {
    #[serde(default)]
    pub accession: String,
    #[serde(default)]
    pub organism_name: String,
    #[serde(default)]
    pub assembly_level: String,
    #[serde(default)]
    pub assembly_name: String,
    #[serde(default)]
    pub total_sequence_length: u64,
    #[serde(default)]
    pub gc_percent: f64,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl NcbiDatasetsClient {
    pub fn new() -> Self {
        NcbiDatasetsClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        NcbiDatasetsClient { base }
    }

    fn api_key_header(&self) -> Vec<(&str, String)> {
        match self.base.get_api_key("NCBI_API_KEY") {
            Some(k) => vec![("api-key", k.to_string())],
            None => vec![],
        }
    }

    /// Get genes by symbol.
    pub fn gene_by_symbol(
        &self,
        symbols: &[&str],
        taxon: &str,
    ) -> Result<Vec<DatasetGene>> {
        let base = base_url();
        let syms = symbols.join(",");
        let url = format!(
            "{base}/gene/symbol/{syms}/taxon/{taxon}"
        );
        let auth = self.api_key_header();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        parse_genes(&json)
    }

    /// Get genes by NCBI gene ID.
    pub fn gene_by_id(&self, ids: &[&str]) -> Result<Vec<DatasetGene>> {
        let base = base_url();
        let id_str = ids.join(",");
        let url = format!("{base}/gene/id/{id_str}");
        let auth = self.api_key_header();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        parse_genes(&json)
    }

    /// Get taxonomy information.
    pub fn taxonomy(&self, taxon: &str) -> Result<TaxonomyInfo> {
        let base = base_url();
        let url = format!(
            "{base}/taxonomy/taxon/{taxon}"
        );
        let auth = self.api_key_header();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        let tax = json["taxonomy_nodes"]
            .as_array()
            .and_then(|a| a.first())
            .unwrap_or(&json);
        let t = &tax["taxonomy"];

        let lineage: Vec<String> = t["lineage"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(TaxonomyInfo {
            tax_id: t["tax_id"]
                .as_u64()
                .map(|n| n.to_string())
                .or_else(|| t["tax_id"].as_str().map(String::from))
                .unwrap_or_default(),
            organism_name: t["organism_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            common_name: t["common_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            lineage,
            rank: t["rank"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// Get genome assembly summary by accession.
    pub fn genome_summary(&self, accession: &str) -> Result<GenomeSummary> {
        let base = base_url();
        let url = format!(
            "{base}/genome/accession/{accession}"
        );
        let auth = self.api_key_header();
        let headers: Vec<(&str, &str)> = auth
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let json = self
            .base
            .get_json_with_headers(&url, &headers)?;

        let report = json["reports"]
            .as_array()
            .and_then(|a| a.first())
            .unwrap_or(&json);
        let info = &report["assembly_info"];
        let stats = &report["assembly_stats"];

        Ok(GenomeSummary {
            accession: report["accession"]
                .as_str()
                .unwrap_or(accession)
                .to_string(),
            organism_name: report["organism"]["organism_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            assembly_level: info["assembly_level"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            assembly_name: info["assembly_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            total_sequence_length: stats["total_sequence_length"]
                .as_u64()
                .unwrap_or(0),
            gc_percent: stats["gc_percent"]
                .as_f64()
                .unwrap_or(0.0),
        })
    }
}

impl Default for NcbiDatasetsClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_genes(json: &serde_json::Value) -> Result<Vec<DatasetGene>> {
    let mut genes = Vec::new();
    if let Some(reports) = json["reports"].as_array() {
        for report in reports {
            let gene = &report["gene"];
            genes.push(DatasetGene {
                gene_id: gene["gene_id"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| gene["gene_id"].as_str().map(String::from))
                    .unwrap_or_default(),
                symbol: gene["symbol"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                description: gene["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                taxname: gene["taxname"]
                    .as_str()
                    .or_else(|| gene["tax_name"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                chromosome: gene["chromosomes"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                gene_type: gene["type"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                common_name: gene["common_name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }
    Ok(genes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_genes_numeric_id() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"reports": [{"gene": {
                "gene_id": 672,
                "symbol": "BRCA1",
                "description": "BRCA1 DNA repair associated",
                "taxname": "Homo sapiens",
                "chromosomes": ["17"],
                "type": "PROTEIN_CODING",
                "common_name": "human"
            }}]}"#,
        )
        .unwrap();
        let genes = parse_genes(&json).unwrap();
        assert_eq!(genes.len(), 1);
        assert_eq!(genes[0].gene_id, "672");
        assert_eq!(genes[0].symbol, "BRCA1");
        assert_eq!(genes[0].taxname, "Homo sapiens");
        assert_eq!(genes[0].chromosome, "17");
        assert_eq!(genes[0].gene_type, "PROTEIN_CODING");
    }

    #[test]
    fn test_parse_genes_string_id() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"reports": [{"gene": {"gene_id": "672", "symbol": "BRCA1"}}]}"#,
        )
        .unwrap();
        let genes = parse_genes(&json).unwrap();
        assert_eq!(genes[0].gene_id, "672");
    }

    #[test]
    fn test_parse_genes_alternate_taxname() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"reports": [{"gene": {"gene_id": 1, "tax_name": "Mus musculus"}}]}"#,
        )
        .unwrap();
        let genes = parse_genes(&json).unwrap();
        assert_eq!(genes[0].taxname, "Mus musculus");
    }

    #[test]
    fn test_parse_genes_no_reports() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let genes = parse_genes(&json).unwrap();
        assert!(genes.is_empty());
    }

    #[test]
    fn test_parse_genes_empty_reports() {
        let json: serde_json::Value = serde_json::from_str(r#"{"reports": []}"#).unwrap();
        let genes = parse_genes(&json).unwrap();
        assert!(genes.is_empty());
    }

    #[test]
    fn test_dataset_gene_serde() {
        let gene = DatasetGene {
            gene_id: "672".into(),
            symbol: "BRCA1".into(),
            description: "BRCA1 DNA repair associated".into(),
            taxname: "Homo sapiens".into(),
            chromosome: "17".into(),
            gene_type: "PROTEIN_CODING".into(),
            common_name: "human".into(),
        };
        let json = serde_json::to_string(&gene).unwrap();
        let back: DatasetGene = serde_json::from_str(&json).unwrap();
        assert_eq!(back.symbol, "BRCA1");
    }

    #[test]
    fn test_parse_genes_missing_gene_object() {
        // Report exists but has no "gene" key
        let json: serde_json::Value = serde_json::from_str(
            r#"{"reports": [{"something_else": {}}]}"#,
        )
        .unwrap();
        let genes = parse_genes(&json).unwrap();
        assert_eq!(genes.len(), 1);
        assert_eq!(genes[0].gene_id, "");
        assert_eq!(genes[0].symbol, "");
    }

    #[test]
    fn test_parse_genes_null_chromosomes() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"reports": [{"gene": {"gene_id": 1, "symbol": "TEST", "chromosomes": null}}]}"#,
        )
        .unwrap();
        let genes = parse_genes(&json).unwrap();
        assert_eq!(genes.len(), 1);
        assert_eq!(genes[0].chromosome, "");
    }
}
