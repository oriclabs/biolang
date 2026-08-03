//! cBioPortal API client.
//!
//! Accesses public cancer genomics data (TCGA, CCLE, etc.).
//! No API key required for public studies.
//! API docs: <https://www.cbioportal.org/api/swagger-ui/index.html>

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("cbioportal", "https://www.cbioportal.org/api")
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CancerStudy {
    pub study_id: String,
    pub name: String,
    pub description: String,
    pub cancer_type: String,
    pub reference_genome: String,
    pub n_samples: u64,
}

#[derive(Debug, Clone)]
pub struct MutationRecord {
    pub sample_id: String,
    pub gene: String,
    pub protein_change: String,
    pub mutation_type: String,
    pub chromosome: String,
    pub start_position: i64,
    pub ref_allele: String,
    pub alt_allele: String,
    pub variant_classification: String,
}

#[derive(Debug, Clone)]
pub struct ClinicalAttribute {
    pub sample_id: String,
    pub patient_id: String,
    pub attribute: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct GeneExpression {
    pub sample_id: String,
    pub gene: String,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct CbioPortalClient {
    base: BaseClient,
}

impl CbioPortalClient {
    pub fn new() -> Self {
        CbioPortalClient {
            base: BaseClient::new(),
        }
    }

    /// List cancer studies, optionally filtered by keyword.
    pub fn studies(&self, keyword: Option<&str>) -> Result<Vec<CancerStudy>> {
        let base = base_url();
        let url = match keyword {
            Some(kw) => {
                format!("{base}/studies?keyword={kw}&pageSize=50&sortBy=studyId&direction=ASC")
            }
            None => format!("{base}/studies?pageSize=50&sortBy=studyId&direction=ASC"),
        };
        let json = self.base.get_json(&url)?;
        parse_studies(&json)
    }

    /// Get mutations for a gene in a study.
    pub fn mutations(&self, study_id: &str, gene: &str) -> Result<Vec<MutationRecord>> {
        let base = base_url();
        let profile_id = format!("{study_id}_mutations");
        let sample_list_id = format!("{study_id}_all");
        let url = format!(
            "{base}/mutations?molecularProfileId={profile_id}&sampleListId={sample_list_id}&hugoGeneSymbol={gene}&pageSize=500"
        );
        let json = self.base.get_json(&url)?;
        parse_mutations(&json)
    }

    /// Get clinical data for all samples in a study.
    pub fn clinical_data(&self, study_id: &str) -> Result<Vec<ClinicalAttribute>> {
        let base = base_url();
        let url =
            format!("{base}/studies/{study_id}/clinical-data?clinicalDataType=SAMPLE&pageSize=500");
        let json = self.base.get_json(&url)?;
        parse_clinical_data(&json)
    }

    /// Get mRNA expression values for a gene in a study.
    pub fn gene_expression(
        &self,
        study_id: &str,
        gene: &str,
        max: usize,
    ) -> Result<Vec<GeneExpression>> {
        // Look up Entrez gene ID first
        let base = base_url();
        let gene_url = format!("{base}/genes/{gene}");
        let gene_json = self.base.get_json(&gene_url)?;
        let entrez_id = gene_json["entrezGeneId"]
            .as_i64()
            .ok_or_else(|| ApiError::Parse {
                context: "cBioPortal gene lookup".into(),
                source: format!("no entrezGeneId for {gene}"),
            })?;

        // Try common mRNA profile IDs
        let profile_id = format!("{study_id}_rna_seq_v2_mrna");
        let sample_list_id = format!("{study_id}_all");
        let url = format!(
            "{base}/molecular-profiles/{profile_id}/molecular-data?sampleListId={sample_list_id}&entrezGeneId={entrez_id}&pageSize={max}"
        );
        let json = self.base.get_json(&url)?;
        parse_expression(&json, gene)
    }

    /// Look up Entrez gene ID and symbol for a Hugo gene symbol.
    pub fn gene_info(&self, hugo_symbol: &str) -> Result<(i64, String)> {
        let base = base_url();
        let url = format!("{base}/genes/{hugo_symbol}");
        let json = self.base.get_json(&url)?;
        let entrez_id = json["entrezGeneId"]
            .as_i64()
            .ok_or_else(|| ApiError::Parse {
                context: "cBioPortal gene".into(),
                source: format!("no entrezGeneId for {hugo_symbol}"),
            })?;
        let symbol = json["hugoGeneSymbol"]
            .as_str()
            .unwrap_or(hugo_symbol)
            .to_string();
        Ok((entrez_id, symbol))
    }
}

impl Default for CbioPortalClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

fn parse_studies(json: &serde_json::Value) -> Result<Vec<CancerStudy>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "cBioPortal studies".into(),
        source: "expected JSON array".into(),
    })?;
    Ok(arr
        .iter()
        .map(|s| CancerStudy {
            study_id: s["studyId"].as_str().unwrap_or_default().to_string(),
            name: s["name"].as_str().unwrap_or_default().to_string(),
            description: s["description"].as_str().unwrap_or_default().to_string(),
            cancer_type: s["cancerTypeId"].as_str().unwrap_or_default().to_string(),
            reference_genome: s["referenceGenome"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            n_samples: s["allSampleCount"].as_u64().unwrap_or(0),
        })
        .collect())
}

fn parse_mutations(json: &serde_json::Value) -> Result<Vec<MutationRecord>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "cBioPortal mutations".into(),
        source: "expected JSON array".into(),
    })?;
    Ok(arr
        .iter()
        .map(|m| MutationRecord {
            sample_id: m["sampleId"].as_str().unwrap_or_default().to_string(),
            gene: m["gene"]["hugoGeneSymbol"]
                .as_str()
                .or_else(|| m["hugoGeneSymbol"].as_str())
                .unwrap_or_default()
                .to_string(),
            protein_change: m["proteinChange"].as_str().unwrap_or_default().to_string(),
            mutation_type: m["mutationType"].as_str().unwrap_or_default().to_string(),
            chromosome: m["chr"]
                .as_str()
                .or_else(|| m["chromosome"].as_str())
                .unwrap_or_default()
                .to_string(),
            start_position: m["startPosition"].as_i64().unwrap_or(0),
            ref_allele: m["referenceAllele"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            alt_allele: m["variantAllele"]
                .as_str()
                .or_else(|| m["altAllele"].as_str())
                .unwrap_or_default()
                .to_string(),
            variant_classification: m["variantClassification"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
        .collect())
}

fn parse_clinical_data(json: &serde_json::Value) -> Result<Vec<ClinicalAttribute>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "cBioPortal clinical data".into(),
        source: "expected JSON array".into(),
    })?;
    Ok(arr
        .iter()
        .map(|c| ClinicalAttribute {
            sample_id: c["sampleId"].as_str().unwrap_or_default().to_string(),
            patient_id: c["patientId"].as_str().unwrap_or_default().to_string(),
            attribute: c["clinicalAttributeId"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            value: c["value"].as_str().unwrap_or_default().to_string(),
        })
        .collect())
}

fn parse_expression(json: &serde_json::Value, gene: &str) -> Result<Vec<GeneExpression>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "cBioPortal expression".into(),
        source: "expected JSON array".into(),
    })?;
    Ok(arr
        .iter()
        .filter_map(|e| {
            let value = e["value"].as_f64()?;
            let sample_id = e["sampleId"].as_str().unwrap_or_default().to_string();
            let gene_sym = e["hugoGeneSymbol"].as_str().unwrap_or(gene).to_string();
            Some(GeneExpression {
                sample_id,
                gene: gene_sym,
                value,
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_studies_array() {
        let json = serde_json::json!([
            {
                "studyId": "brca_tcga",
                "name": "Breast Invasive Carcinoma (TCGA, PanCancer Atlas)",
                "description": "TCGA breast cancer cohort",
                "cancerTypeId": "brca",
                "referenceGenome": "hg19",
                "allSampleCount": 1084
            },
            {
                "studyId": "luad_tcga",
                "name": "Lung Adenocarcinoma (TCGA)",
                "description": "TCGA lung adenocarcinoma",
                "cancerTypeId": "luad",
                "referenceGenome": "hg19",
                "allSampleCount": 566
            }
        ]);
        let studies = parse_studies(&json).unwrap();
        assert_eq!(studies.len(), 2);
        assert_eq!(studies[0].study_id, "brca_tcga");
        assert_eq!(studies[0].n_samples, 1084);
        assert_eq!(studies[1].cancer_type, "luad");
    }

    #[test]
    fn parse_mutations_array() {
        let json = serde_json::json!([
            {
                "sampleId": "TCGA-A1-A0SB-01",
                "gene": {"hugoGeneSymbol": "TP53"},
                "proteinChange": "R248W",
                "mutationType": "Missense_Mutation",
                "chr": "17",
                "startPosition": 7674220,
                "referenceAllele": "C",
                "variantAllele": "T",
                "variantClassification": "Missense_Mutation"
            }
        ]);
        let muts = parse_mutations(&json).unwrap();
        assert_eq!(muts.len(), 1);
        assert_eq!(muts[0].gene, "TP53");
        assert_eq!(muts[0].protein_change, "R248W");
        assert_eq!(muts[0].chromosome, "17");
        assert_eq!(muts[0].start_position, 7674220);
    }

    #[test]
    fn parse_clinical_data_array() {
        let json = serde_json::json!([
            {
                "sampleId": "TCGA-A1-A0SB-01",
                "patientId": "TCGA-A1-A0SB",
                "clinicalAttributeId": "OS_STATUS",
                "value": "0:LIVING"
            },
            {
                "sampleId": "TCGA-A1-A0SB-01",
                "patientId": "TCGA-A1-A0SB",
                "clinicalAttributeId": "AGE",
                "value": "55"
            }
        ]);
        let clinical = parse_clinical_data(&json).unwrap();
        assert_eq!(clinical.len(), 2);
        assert_eq!(clinical[0].attribute, "OS_STATUS");
        assert_eq!(clinical[0].value, "0:LIVING");
        assert_eq!(clinical[1].attribute, "AGE");
    }

    #[test]
    fn parse_expression_array() {
        let json = serde_json::json!([
            {"sampleId": "S1", "hugoGeneSymbol": "BRCA1", "value": 8.23},
            {"sampleId": "S2", "hugoGeneSymbol": "BRCA1", "value": 6.11}
        ]);
        let expr = parse_expression(&json, "BRCA1").unwrap();
        assert_eq!(expr.len(), 2);
        assert_eq!(expr[0].sample_id, "S1");
        assert!((expr[0].value - 8.23).abs() < 1e-6);
        assert_eq!(expr[1].gene, "BRCA1");
    }

    #[test]
    fn parse_studies_empty() {
        let json = serde_json::json!([]);
        let studies = parse_studies(&json).unwrap();
        assert!(studies.is_empty());
    }

    #[test]
    fn parse_mutations_flat_gene_field() {
        // Some endpoints return hugoGeneSymbol at top level
        let json = serde_json::json!([
            {
                "sampleId": "S1",
                "hugoGeneSymbol": "KRAS",
                "proteinChange": "G12D",
                "mutationType": "Missense_Mutation",
                "chr": "12",
                "startPosition": 25398281,
                "referenceAllele": "C",
                "variantAllele": "T",
                "variantClassification": "Missense_Mutation"
            }
        ]);
        let muts = parse_mutations(&json).unwrap();
        assert_eq!(muts[0].gene, "KRAS");
        assert_eq!(muts[0].protein_change, "G12D");
    }
}
