//! BioMart / Ensembl BioMart client.
//!
//! Attribute queries, gene/SNP lookups.
//! API docs: <https://www.ensembl.org/info/data/biomart/biomart_restful.html>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("biomart", "https://www.ensembl.org/biomart/martservice")
}

/// BioMart / Ensembl BioMart client.
pub struct BioMartClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl BioMartClient {
    pub fn new() -> Self {
        BioMartClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        BioMartClient { base }
    }

    /// List available datasets.
    pub fn list_datasets(&self) -> Result<Vec<Dataset>> {
        let base = base_url();
        let url = format!(
            "{base}?type=datasets&mart=ENSEMBL_MART_ENSEMBL"
        );
        let text = self.base.get_text(&url)?;
        let mut datasets = Vec::new();
        for line in text.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 3 && cols[0] == "TableSet" {
                datasets.push(Dataset {
                    name: cols[1].to_string(),
                    display_name: cols[2].to_string(),
                });
            }
        }
        Ok(datasets)
    }

    /// List attributes for a dataset.
    pub fn list_attributes(&self, dataset: &str) -> Result<Vec<Attribute>> {
        let base = base_url();
        let url = format!(
            "{base}?type=attributes&dataset={dataset}"
        );
        let text = self.base.get_text(&url)?;
        let mut attrs = Vec::new();
        for line in text.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 2 {
                attrs.push(Attribute {
                    name: cols[0].to_string(),
                    display_name: cols[1].to_string(),
                });
            }
        }
        Ok(attrs)
    }

    /// List filters for a dataset.
    pub fn list_filters(&self, dataset: &str) -> Result<Vec<Filter>> {
        let base = base_url();
        let url = format!(
            "{base}?type=filters&dataset={dataset}"
        );
        let text = self.base.get_text(&url)?;
        let mut filters = Vec::new();
        for line in text.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 2 {
                filters.push(Filter {
                    name: cols[0].to_string(),
                    display_name: cols[1].to_string(),
                });
            }
        }
        Ok(filters)
    }

    /// Execute a BioMart query with specified attributes and filters.
    /// Returns rows of tab-separated values.
    pub fn query(
        &self,
        dataset: &str,
        attributes: &[&str],
        filters: &[(&str, &str)],
    ) -> Result<Vec<Vec<String>>> {
        let xml = build_query_xml(dataset, attributes, filters);
        let base = base_url();
        let url = format!("{base}?query={xml}");
        let text = self.base.get_text(&url)?;
        let mut rows = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("[") {
                continue;
            }
            let cols: Vec<String> =
                line.split('\t').map(|s| s.to_string()).collect();
            rows.push(cols);
        }
        Ok(rows)
    }

    /// Convenience: query genes by symbol (human dataset).
    pub fn genes_by_symbol(&self, symbols: &[&str]) -> Result<Vec<Vec<String>>> {
        let filter_val = symbols.join(",");
        self.query(
            "hsapiens_gene_ensembl",
            &[
                "ensembl_gene_id",
                "external_gene_name",
                "chromosome_name",
                "start_position",
                "end_position",
                "description",
            ],
            &[("external_gene_name", &filter_val)],
        )
    }

    /// Convenience: query genes by genomic region (human dataset).
    pub fn genes_by_region(
        &self,
        chrom: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<Vec<String>>> {
        self.query(
            "hsapiens_gene_ensembl",
            &[
                "ensembl_gene_id",
                "external_gene_name",
                "chromosome_name",
                "start_position",
                "end_position",
                "gene_biotype",
            ],
            &[
                ("chromosome_name", chrom),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
            ],
        )
    }
}

impl Default for BioMartClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Build BioMart XML query.
fn build_query_xml(
    dataset: &str,
    attributes: &[&str],
    filters: &[(&str, &str)],
) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE Query><Query virtualSchemaName="default" formatter="TSV" header="0" uniqueRows="0" count="" datasetConfigVersion="0.6">"#,
    );
    xml.push_str(&format!(
        r#"<Dataset name="{dataset}" interface="default">"#
    ));
    for (name, value) in filters {
        xml.push_str(&format!(
            r#"<Filter name="{name}" value="{value}"/>"#
        ));
    }
    for attr in attributes {
        xml.push_str(&format!(r#"<Attribute name="{attr}"/>"#));
    }
    xml.push_str("</Dataset></Query>");
    xml
}
