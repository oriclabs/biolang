//! gnomAD GraphQL API client.
//!
//! API docs: <https://gnomad.broadinstitute.org/api>

use serde_json;

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("gnomad", "https://gnomad.broadinstitute.org/api")
}

pub struct GnomadClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GnomadVariant {
    pub variant_id: String,
    pub chrom: String,
    pub pos: u64,
    pub ref_allele: String,
    pub alt_allele: String,
    pub af: f64,
    pub ac: u64,
    pub an: u64,
    pub n_homozygotes: u64,
    pub consequence: String,
    pub lof: String,
    pub gene: String,
    pub dataset: String,
}

#[derive(Debug, Clone)]
pub struct GnomadGeneResult {
    pub gene_id: String,
    pub gene_name: String,
    pub variants: Vec<GnomadVariant>,
}

#[derive(Debug, Clone)]
pub struct PopulationFreq {
    pub population: String,
    pub af: f64,
    pub ac: u64,
    pub an: u64,
    pub n_homozygotes: u64,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl GnomadClient {
    pub fn new() -> Self {
        GnomadClient {
            base: BaseClient::new(),
        }
    }

    fn query(&self, query: &str, variables: serde_json::Value) -> Result<serde_json::Value> {
        let url = base_url();
        let body = serde_json::json!({ "query": query, "variables": variables });
        self.base.post_json(&url, &body)
    }

    /// Fetch a single variant by ID (e.g. "1-55516888-G-GA").
    pub fn variant(&self, variant_id: &str, dataset: &str) -> Result<GnomadVariant> {
        let q = r#"
            query Variant($variantId: String!, $datasetId: DatasetId!) {
              variant(variantId: $variantId, dataset: $datasetId) {
                variantId chrom pos ref alt
                exome { ac an af n_homozygotes }
                genome { ac an af n_homozygotes }
                consequence lof
              }
            }
        "#;
        let vars = serde_json::json!({
            "variantId": variant_id,
            "datasetId": dataset
        });
        let resp = self.query(q, vars)?;
        let v = &resp["data"]["variant"];
        if v.is_null() {
            return Err(ApiError::Parse {
                context: "gnomAD variant".into(),
                source: format!("variant {} not found in dataset {}", variant_id, dataset),
            });
        }
        Ok(parse_variant(v, "", dataset))
    }

    /// Fetch all variants in a gene.
    pub fn gene_variants(&self, gene_symbol: &str, dataset: &str) -> Result<GnomadGeneResult> {
        let q = r#"
            query GeneVariants($geneSymbol: String!, $datasetId: DatasetId!) {
              gene(gene_symbol: $geneSymbol, reference_genome: GRCh38) {
                gene_id gene_name
                variants(dataset: $datasetId) {
                  variantId chrom pos ref alt
                  exome { ac an af n_homozygotes }
                  genome { ac an af n_homozygotes }
                  consequence lof
                }
              }
            }
        "#;
        let vars = serde_json::json!({
            "geneSymbol": gene_symbol,
            "datasetId": dataset
        });
        let resp = self.query(q, vars)?;
        let gene = &resp["data"]["gene"];
        if gene.is_null() {
            return Err(ApiError::Parse {
                context: "gnomAD gene".into(),
                source: format!("gene {} not found", gene_symbol),
            });
        }
        let gene_id = gene["gene_id"].as_str().unwrap_or("").to_string();
        let gene_name = gene["gene_name"]
            .as_str()
            .unwrap_or(gene_symbol)
            .to_string();
        let variants = gene["variants"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| parse_variant(v, &gene_name, dataset))
            .collect();
        Ok(GnomadGeneResult {
            gene_id,
            gene_name,
            variants,
        })
    }

    /// Fetch population-level allele frequencies for a variant.
    pub fn variant_populations(
        &self,
        variant_id: &str,
        dataset: &str,
    ) -> Result<Vec<PopulationFreq>> {
        let q = r#"
            query VariantPops($variantId: String!, $datasetId: DatasetId!) {
              variant(variantId: $variantId, dataset: $datasetId) {
                genome {
                  populations { id ac an af n_homozygotes }
                }
                exome {
                  populations { id ac an af n_homozygotes }
                }
              }
            }
        "#;
        let vars = serde_json::json!({
            "variantId": variant_id,
            "datasetId": dataset
        });
        let resp = self.query(q, vars)?;
        let v = &resp["data"]["variant"];
        if v.is_null() {
            return Ok(vec![]);
        }
        // prefer genome populations, fall back to exome
        let pops_json = if !v["genome"]["populations"].is_null() {
            &v["genome"]["populations"]
        } else {
            &v["exome"]["populations"]
        };
        let pops = pops_json
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|p| PopulationFreq {
                population: p["id"].as_str().unwrap_or("").to_string(),
                af: p["af"].as_f64().unwrap_or(0.0),
                ac: p["ac"].as_u64().unwrap_or(0),
                an: p["an"].as_u64().unwrap_or(0),
                n_homozygotes: p["n_homozygotes"].as_u64().unwrap_or(0),
            })
            .collect();
        Ok(pops)
    }
}

impl Default for GnomadClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_variant(v: &serde_json::Value, gene: &str, dataset: &str) -> GnomadVariant {
    // Prefer exome counts; fall back to genome
    let (ac, an, af, hom) = {
        let exome = &v["exome"];
        let genome = &v["genome"];
        let src = if !exome.is_null() && exome["ac"].as_u64().unwrap_or(0) > 0 {
            exome
        } else {
            genome
        };
        (
            src["ac"].as_u64().unwrap_or(0),
            src["an"].as_u64().unwrap_or(0),
            src["af"].as_f64().unwrap_or(0.0),
            src["n_homozygotes"].as_u64().unwrap_or(0),
        )
    };
    GnomadVariant {
        variant_id: v["variantId"].as_str().unwrap_or("").to_string(),
        chrom: v["chrom"].as_str().unwrap_or("").to_string(),
        pos: v["pos"].as_u64().unwrap_or(0),
        ref_allele: v["ref"].as_str().unwrap_or("").to_string(),
        alt_allele: v["alt"].as_str().unwrap_or("").to_string(),
        af,
        ac,
        an,
        n_homozygotes: hom,
        consequence: v["consequence"].as_str().unwrap_or("").to_string(),
        lof: v["lof"].as_str().unwrap_or("").to_string(),
        gene: gene.to_string(),
        dataset: dataset.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_variant_json() -> serde_json::Value {
        serde_json::json!({
            "variantId": "1-55516888-G-GA",
            "chrom": "1",
            "pos": 55516888u64,
            "ref": "G",
            "alt": "GA",
            "exome": { "ac": 12, "an": 240000, "af": 0.00005, "n_homozygotes": 0 },
            "genome": { "ac": 3, "an": 60000, "af": 0.00005, "n_homozygotes": 0 },
            "consequence": "frameshift_variant",
            "lof": "HC"
        })
    }

    #[test]
    fn test_parse_variant_prefers_exome() {
        let v = mock_variant_json();
        let parsed = parse_variant(&v, "PCSK9", "gnomad_r4");
        assert_eq!(parsed.variant_id, "1-55516888-G-GA");
        assert_eq!(parsed.chrom, "1");
        assert_eq!(parsed.pos, 55516888);
        assert_eq!(parsed.ref_allele, "G");
        assert_eq!(parsed.alt_allele, "GA");
        assert_eq!(parsed.ac, 12); // exome preferred
        assert_eq!(parsed.an, 240000);
        assert_eq!(parsed.consequence, "frameshift_variant");
        assert_eq!(parsed.lof, "HC");
        assert_eq!(parsed.gene, "PCSK9");
        assert_eq!(parsed.dataset, "gnomad_r4");
    }

    #[test]
    fn test_parse_variant_falls_back_to_genome() {
        let v = serde_json::json!({
            "variantId": "17-43092919-G-A",
            "chrom": "17",
            "pos": 43092919u64,
            "ref": "G",
            "alt": "A",
            "exome": serde_json::Value::Null,
            "genome": { "ac": 45, "an": 125748, "af": 0.000358, "n_homozygotes": 1 },
            "consequence": "missense_variant",
            "lof": ""
        });
        let parsed = parse_variant(&v, "BRCA1", "gnomad_r4");
        assert_eq!(parsed.ac, 45);
        assert_eq!(parsed.an, 125748);
        assert!((parsed.af - 0.000358).abs() < 1e-9);
        assert_eq!(parsed.n_homozygotes, 1);
        assert_eq!(parsed.consequence, "missense_variant");
    }

    #[test]
    fn test_parse_variant_missing_fields() {
        let v = serde_json::json!({
            "variantId": "2-100-A-T",
            "chrom": "2",
            "pos": 100u64,
            "ref": "A",
            "alt": "T",
            "exome": serde_json::Value::Null,
            "genome": serde_json::Value::Null,
            "consequence": serde_json::Value::Null,
            "lof": serde_json::Value::Null
        });
        let parsed = parse_variant(&v, "", "gnomad_r4");
        assert_eq!(parsed.ac, 0);
        assert_eq!(parsed.an, 0);
        assert_eq!(parsed.af, 0.0);
        assert_eq!(parsed.consequence, "");
        assert_eq!(parsed.lof, "");
    }

    #[test]
    fn test_parse_populations() {
        let pops_json = serde_json::json!([
            { "id": "afr", "ac": 8, "an": 30000, "af": 0.000267, "n_homozygotes": 0 },
            { "id": "eur", "ac": 4, "an": 90000, "af": 0.0000444, "n_homozygotes": 0 },
            { "id": "sas", "ac": 0, "an": 15000, "af": 0.0, "n_homozygotes": 0 }
        ]);
        let pops: Vec<PopulationFreq> = pops_json
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PopulationFreq {
                population: p["id"].as_str().unwrap_or("").to_string(),
                af: p["af"].as_f64().unwrap_or(0.0),
                ac: p["ac"].as_u64().unwrap_or(0),
                an: p["an"].as_u64().unwrap_or(0),
                n_homozygotes: p["n_homozygotes"].as_u64().unwrap_or(0),
            })
            .collect();
        assert_eq!(pops.len(), 3);
        assert_eq!(pops[0].population, "afr");
        assert_eq!(pops[0].ac, 8);
        assert_eq!(pops[1].population, "eur");
        assert_eq!(pops[2].an, 15000);
        assert_eq!(pops[2].ac, 0);
    }

    #[test]
    fn test_parse_gene_result() {
        let resp = serde_json::json!({
            "data": {
                "gene": {
                    "gene_id": "ENSG00000169174",
                    "gene_name": "PCSK9",
                    "variants": [
                        {
                            "variantId": "1-55516888-G-GA",
                            "chrom": "1", "pos": 55516888u64, "ref": "G", "alt": "GA",
                            "exome": { "ac": 12, "an": 240000, "af": 0.00005, "n_homozygotes": 0 },
                            "genome": serde_json::Value::Null,
                            "consequence": "frameshift_variant", "lof": "HC"
                        },
                        {
                            "variantId": "1-55524276-C-T",
                            "chrom": "1", "pos": 55524276u64, "ref": "C", "alt": "T",
                            "exome": { "ac": 1, "an": 240000, "af": 0.0000042, "n_homozygotes": 0 },
                            "genome": serde_json::Value::Null,
                            "consequence": "stop_gained", "lof": "HC"
                        }
                    ]
                }
            }
        });
        let gene = &resp["data"]["gene"];
        let gene_id = gene["gene_id"].as_str().unwrap_or("").to_string();
        let gene_name = gene["gene_name"].as_str().unwrap_or("").to_string();
        let variants: Vec<GnomadVariant> = gene["variants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| parse_variant(v, &gene_name, "gnomad_r4"))
            .collect();
        assert_eq!(gene_id, "ENSG00000169174");
        assert_eq!(gene_name, "PCSK9");
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].lof, "HC");
        assert_eq!(variants[1].consequence, "stop_gained");
    }
}
