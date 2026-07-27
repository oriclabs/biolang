//! GTEx REST API v2 client.
//!
//! API docs: <https://gtexportal.org/api/v2/redoc>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("gtex", "https://gtexportal.org/api/v2")
}

pub struct GtexClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GtexTissue {
    #[serde(rename = "tissueSiteDetailId")]
    pub tissue_id: String,
    #[serde(rename = "tissueSiteDetail")]
    pub tissue_name: String,
    #[serde(rename = "tissueSite")]
    pub tissue_site: String,
    #[serde(rename = "rnaSeqAndGenotypeSampleCount")]
    pub sample_count: u64,
    #[serde(rename = "colorHex")]
    pub color_hex: String,
}

impl Default for GtexTissue {
    fn default() -> Self {
        GtexTissue {
            tissue_id: String::new(),
            tissue_name: String::new(),
            tissue_site: String::new(),
            sample_count: 0,
            color_hex: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TissueExpression {
    /// Gene identifier (Ensembl or symbol depending on query).
    #[serde(rename = "gencodeId")]
    pub gencode_id: String,
    #[serde(rename = "geneSymbol")]
    pub gene_symbol: String,
    #[serde(rename = "tissueSiteDetailId")]
    pub tissue_id: String,
    /// Median TPM across GTEx donors.
    pub median: f64,
    /// Unit (usually "TPM").
    pub unit: String,
    #[serde(rename = "datasetId")]
    pub dataset_id: String,
}

impl Default for TissueExpression {
    fn default() -> Self {
        TissueExpression {
            gencode_id: String::new(),
            gene_symbol: String::new(),
            tissue_id: String::new(),
            median: 0.0,
            unit: String::new(),
            dataset_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EqtlRecord {
    #[serde(rename = "gencodeId")]
    pub gencode_id: String,
    #[serde(rename = "geneSymbol")]
    pub gene_symbol: String,
    #[serde(rename = "tissueSiteDetailId")]
    pub tissue_id: String,
    #[serde(rename = "variantId")]
    pub variant_id: String,
    pub rs_id: String,
    pub pos: u64,
    pub chr: String,
    pub ref_allele: String,
    pub alt_allele: String,
    /// Effect size (beta).
    pub nes: f64,
    pub p_value: f64,
    pub q_value: f64,
    #[serde(rename = "pValueThreshold")]
    pub p_value_threshold: f64,
}

impl Default for EqtlRecord {
    fn default() -> Self {
        EqtlRecord {
            gencode_id: String::new(),
            gene_symbol: String::new(),
            tissue_id: String::new(),
            variant_id: String::new(),
            rs_id: String::new(),
            pos: 0,
            chr: String::new(),
            ref_allele: String::new(),
            alt_allele: String::new(),
            nes: 0.0,
            p_value: 1.0,
            q_value: 1.0,
            p_value_threshold: 0.05,
        }
    }
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl GtexClient {
    pub fn new() -> Self {
        GtexClient {
            base: BaseClient::new(),
        }
    }

    /// List all GTEx tissues with sample counts.
    pub fn tissues(&self) -> Result<Vec<GtexTissue>> {
        let url = format!("{}/dataset/tissueSiteDetail", base_url());
        let resp = self.base.get_json(&url)?;
        let items = resp["tissueSiteDetail"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        Ok(items)
    }

    /// Median TPM expression of a gene across all tissues.
    pub fn expression(&self, gene_id: &str) -> Result<Vec<TissueExpression>> {
        let url = format!(
            "{}/expression/medianGeneExpression?gencodeId={}&datasetId=gtex_v8",
            base_url(),
            urlencoded(gene_id)
        );
        let resp = self.base.get_json(&url)?;
        let items = resp["medianGeneExpression"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        Ok(items)
    }

    /// Return the top `n` tissues by median TPM for a gene.
    pub fn top_expressed_tissues(
        &self,
        gene_id: &str,
        top_n: usize,
    ) -> Result<Vec<TissueExpression>> {
        let mut expr = self.expression(gene_id)?;
        expr.sort_by(|a, b| b.median.partial_cmp(&a.median).unwrap_or(std::cmp::Ordering::Equal));
        expr.truncate(top_n);
        Ok(expr)
    }

    /// Significant eQTLs for a gene in a tissue.
    pub fn eqtls(&self, gene_id: &str, tissue: &str) -> Result<Vec<EqtlRecord>> {
        let url = format!(
            "{}/association/singleTissueEqtl?gencodeId={}&tissueSiteDetailId={}&datasetId=gtex_v8",
            base_url(),
            urlencoded(gene_id),
            urlencoded(tissue)
        );
        let resp = self.base.get_json(&url)?;
        let items = resp["singleTissueEqtl"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        Ok(items)
    }

    /// Independent eQTLs (lead variants) for a gene in a tissue.
    pub fn independent_eqtls(&self, gene_id: &str, tissue: &str) -> Result<Vec<EqtlRecord>> {
        let url = format!(
            "{}/association/independentEqtl?gencodeId={}&tissueSiteDetailId={}&datasetId=gtex_v8",
            base_url(),
            urlencoded(gene_id),
            urlencoded(tissue)
        );
        let resp = self.base.get_json(&url)?;
        let items = resp["independentEqtl"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        Ok(items)
    }
}

impl Default for GtexClient {
    fn default() -> Self {
        Self::new()
    }
}

fn urlencoded(s: &str) -> String {
    s.replace('+', "%2B").replace(' ', "%20")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_tissue() {
        let json = serde_json::json!({
            "tissueSiteDetailId": "Liver",
            "tissueSiteDetail": "Liver",
            "tissueSite": "Liver",
            "rnaSeqAndGenotypeSampleCount": 208,
            "colorHex": "AABB33"
        });
        let tissue: GtexTissue = serde_json::from_value(json).unwrap();
        assert_eq!(tissue.tissue_id, "Liver");
        assert_eq!(tissue.sample_count, 208);
        assert_eq!(tissue.color_hex, "AABB33");
    }

    #[test]
    fn test_deserialize_tissue_list() {
        let resp = serde_json::json!({
            "tissueSiteDetail": [
                {
                    "tissueSiteDetailId": "Brain_Cortex",
                    "tissueSiteDetail": "Brain - Cortex",
                    "tissueSite": "Brain",
                    "rnaSeqAndGenotypeSampleCount": 205,
                    "colorHex": "C7B8E5"
                },
                {
                    "tissueSiteDetailId": "Whole_Blood",
                    "tissueSiteDetail": "Whole Blood",
                    "tissueSite": "Blood",
                    "rnaSeqAndGenotypeSampleCount": 670,
                    "colorHex": "FF0000"
                }
            ]
        });
        let items: Vec<GtexTissue> = resp["tissueSiteDetail"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].tissue_id, "Brain_Cortex");
        assert_eq!(items[1].tissue_id, "Whole_Blood");
        assert_eq!(items[1].sample_count, 670);
    }

    #[test]
    fn test_deserialize_expression() {
        let resp = serde_json::json!({
            "medianGeneExpression": [
                {
                    "gencodeId": "ENSG00000012048.23",
                    "geneSymbol": "BRCA1",
                    "tissueSiteDetailId": "Breast_Mammary_Tissue",
                    "median": 8.45,
                    "unit": "TPM",
                    "datasetId": "gtex_v8"
                },
                {
                    "gencodeId": "ENSG00000012048.23",
                    "geneSymbol": "BRCA1",
                    "tissueSiteDetailId": "Ovary",
                    "median": 12.3,
                    "unit": "TPM",
                    "datasetId": "gtex_v8"
                }
            ]
        });
        let items: Vec<TissueExpression> = resp["medianGeneExpression"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].gene_symbol, "BRCA1");
        assert!((items[0].median - 8.45).abs() < 1e-9);
        assert_eq!(items[1].tissue_id, "Ovary");
        assert!((items[1].median - 12.3).abs() < 1e-9);
    }

    #[test]
    fn test_top_expressed_tissues_sorts_correctly() {
        let mut records = vec![
            TissueExpression { tissue_id: "Liver".into(), median: 5.0, ..Default::default() },
            TissueExpression { tissue_id: "Brain_Cortex".into(), median: 22.1, ..Default::default() },
            TissueExpression { tissue_id: "Whole_Blood".into(), median: 1.2, ..Default::default() },
            TissueExpression { tissue_id: "Lung".into(), median: 14.8, ..Default::default() },
        ];
        records.sort_by(|a, b| b.median.partial_cmp(&a.median).unwrap_or(std::cmp::Ordering::Equal));
        records.truncate(2);
        assert_eq!(records[0].tissue_id, "Brain_Cortex");
        assert_eq!(records[1].tissue_id, "Lung");
    }

    #[test]
    fn test_deserialize_eqtl() {
        let resp = serde_json::json!({
            "singleTissueEqtl": [
                {
                    "gencodeId": "ENSG00000012048.23",
                    "geneSymbol": "BRCA1",
                    "tissueSiteDetailId": "Breast_Mammary_Tissue",
                    "variantId": "chr17_43092919_G_A_b38",
                    "rs_id": "rs1799966",
                    "pos": 43092919,
                    "chr": "chr17",
                    "ref_allele": "G",
                    "alt_allele": "A",
                    "nes": -0.21,
                    "p_value": 4.5e-8,
                    "q_value": 0.001,
                    "pValueThreshold": 0.05
                }
            ]
        });
        let items: Vec<EqtlRecord> = resp["singleTissueEqtl"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].variant_id, "chr17_43092919_G_A_b38");
        assert_eq!(items[0].rs_id, "rs1799966");
        assert!((items[0].nes - (-0.21)).abs() < 1e-9);
        assert!(items[0].p_value < 1e-7);
        assert_eq!(items[0].chr, "chr17");
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("ENSG00000012048.23"), "ENSG00000012048.23");
        assert_eq!(urlencoded("Whole Blood"), "Whole%20Blood");
        assert_eq!(urlencoded("C+T"), "C%2BT");
    }
}
