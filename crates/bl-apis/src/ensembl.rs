//! Ensembl REST API client.
//!
//! Gene lookup, sequences, VEP, orthologs.
//! API docs: <https://rest.ensembl.org/>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("ensembl", "https://rest.ensembl.org")
}

/// Ensembl REST API client.
pub struct EnsemblClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub id: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub species: String,
    #[serde(default)]
    pub biotype: String,
    #[serde(default)]
    pub start: u64,
    #[serde(default)]
    pub end: u64,
    #[serde(default)]
    pub strand: i8,
    #[serde(default, alias = "seq_region_name")]
    pub chromosome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    pub id: String,
    #[serde(default)]
    pub seq: String,
    #[serde(default, alias = "molecule_type")]
    pub molecule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VepResult {
    #[serde(default)]
    pub allele_string: String,
    #[serde(default)]
    pub most_severe_consequence: String,
    #[serde(default)]
    pub transcript_consequences: Vec<TranscriptConsequence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptConsequence {
    #[serde(default)]
    pub gene_id: String,
    #[serde(default)]
    pub transcript_id: String,
    #[serde(default)]
    pub consequence_terms: Vec<String>,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub biotype: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ortholog {
    #[serde(default)]
    pub source: OrthologGene,
    #[serde(default)]
    pub target: OrthologGene,
    #[serde(default, rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrthologGene {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub species: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl EnsemblClient {
    pub fn new() -> Self {
        EnsemblClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        EnsemblClient { base }
    }

    fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let base = base_url();
        let url = format!("{base}{path}");
        self.base
            .get_json_with_headers(&url, &[("Content-Type", "application/json")])
    }

    /// Look up a gene by Ensembl ID.
    pub fn gene_by_id(&self, id: &str) -> Result<Gene> {
        let json = self.get_json(&format!("/lookup/id/{id}?expand=0"))?;
        parse_gene(&json)
    }

    /// Look up a gene by symbol and species.
    pub fn gene_by_symbol(&self, species: &str, symbol: &str) -> Result<Gene> {
        let json = self.get_json(&format!(
            "/lookup/symbol/{species}/{symbol}?expand=0"
        ))?;
        parse_gene(&json)
    }

    /// Fetch sequence by Ensembl ID. `seq_type`: "genomic", "cdna", "cds", "protein".
    pub fn sequence_by_id(&self, id: &str, seq_type: &str) -> Result<Sequence> {
        let json = self.get_json(&format!(
            "/sequence/id/{id}?type={seq_type}"
        ))?;
        Ok(Sequence {
            id: json["id"].as_str().unwrap_or(id).to_string(),
            seq: json["seq"].as_str().unwrap_or_default().to_string(),
            molecule: json["molecule"]
                .as_str()
                .unwrap_or(seq_type)
                .to_string(),
        })
    }

    /// Fetch sequence by genomic region.
    pub fn sequence_by_region(
        &self,
        species: &str,
        region: &str,
    ) -> Result<Sequence> {
        let json = self.get_json(&format!(
            "/sequence/region/{species}/{region}"
        ))?;
        Ok(Sequence {
            id: json["id"]
                .as_str()
                .unwrap_or(region)
                .to_string(),
            seq: json["seq"].as_str().unwrap_or_default().to_string(),
            molecule: json["molecule"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// Variant Effect Predictor via HGVS notation.
    pub fn vep_hgvs(&self, hgvs: &str) -> Result<Vec<VepResult>> {
        let json = self.get_json(&format!("/vep/human/hgvs/{hgvs}"))?;
        parse_vep_array(&json)
    }

    /// Variant Effect Predictor via region/allele.
    pub fn vep_region(
        &self,
        species: &str,
        region: &str,
        allele: &str,
    ) -> Result<Vec<VepResult>> {
        let json = self.get_json(&format!(
            "/vep/{species}/region/{region}/{allele}"
        ))?;
        parse_vep_array(&json)
    }

    /// Fetch orthologs for a gene.
    pub fn orthologs(&self, id: &str) -> Result<Vec<Ortholog>> {
        let json = self.get_json(&format!(
            "/homology/id/{id}?type=orthologues"
        ))?;

        let mut results = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for entry in data {
                if let Some(homologies) = entry["homologies"].as_array() {
                    for h in homologies {
                        results.push(Ortholog {
                            source: OrthologGene {
                                id: h["source"]["id"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                species: h["source"]["species"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            },
                            target: OrthologGene {
                                id: h["target"]["id"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                species: h["target"]["species"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            },
                            type_: h["type"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string(),
                        });
                    }
                }
            }
        }
        Ok(results)
    }

    /// Raw lookup by ID (returns full JSON).
    pub fn lookup_id(&self, id: &str) -> Result<serde_json::Value> {
        self.get_json(&format!("/lookup/id/{id}?expand=1"))
    }
}

impl Default for EnsemblClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_gene(json: &serde_json::Value) -> Result<Gene> {
    Ok(Gene {
        id: json["id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        symbol: json["display_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        description: json["description"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        species: json["species"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        biotype: json["biotype"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        start: json["start"].as_u64().unwrap_or(0),
        end: json["end"].as_u64().unwrap_or(0),
        strand: json["strand"].as_i64().unwrap_or(0) as i8,
        chromosome: json["seq_region_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    })
}

fn parse_vep_array(json: &serde_json::Value) -> Result<Vec<VepResult>> {
    let arr = json.as_array().ok_or_else(|| ApiError::Parse {
        context: "VEP response".into(),
        source: "expected array".into(),
    })?;
    let mut results = Vec::new();
    for item in arr {
        let tcs: Vec<TranscriptConsequence> = item["transcript_consequences"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|tc| serde_json::from_value(tc.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        results.push(VepResult {
            allele_string: item["allele_string"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            most_severe_consequence: item["most_severe_consequence"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            transcript_consequences: tcs,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gene() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "ENSG00000012048",
                "display_name": "BRCA1",
                "description": "BRCA1 DNA repair associated",
                "species": "homo_sapiens",
                "biotype": "protein_coding",
                "start": 43044295,
                "end": 43170245,
                "strand": -1,
                "seq_region_name": "17"
            }"#,
        )
        .unwrap();
        let gene = parse_gene(&json).unwrap();
        assert_eq!(gene.id, "ENSG00000012048");
        assert_eq!(gene.symbol, "BRCA1");
        assert_eq!(gene.species, "homo_sapiens");
        assert_eq!(gene.biotype, "protein_coding");
        assert_eq!(gene.start, 43044295);
        assert_eq!(gene.end, 43170245);
        assert_eq!(gene.strand, -1);
        assert_eq!(gene.chromosome, "17");
    }

    #[test]
    fn test_parse_gene_missing_fields() {
        let json: serde_json::Value = serde_json::from_str(r#"{"id": "ENSG00000000001"}"#).unwrap();
        let gene = parse_gene(&json).unwrap();
        assert_eq!(gene.id, "ENSG00000000001");
        assert_eq!(gene.symbol, "");
        assert_eq!(gene.start, 0);
        assert_eq!(gene.strand, 0);
    }

    #[test]
    fn test_parse_vep_array() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{
                "allele_string": "C/T",
                "most_severe_consequence": "missense_variant",
                "transcript_consequences": [{
                    "gene_id": "ENSG00000012048",
                    "transcript_id": "ENST00000357654",
                    "consequence_terms": ["missense_variant"],
                    "impact": "MODERATE",
                    "biotype": "protein_coding"
                }]
            }]"#,
        )
        .unwrap();
        let results = parse_vep_array(&json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].allele_string, "C/T");
        assert_eq!(results[0].most_severe_consequence, "missense_variant");
        assert_eq!(results[0].transcript_consequences.len(), 1);
        assert_eq!(results[0].transcript_consequences[0].gene_id, "ENSG00000012048");
        assert_eq!(results[0].transcript_consequences[0].impact, "MODERATE");
    }

    #[test]
    fn test_parse_vep_not_array() {
        let json: serde_json::Value = serde_json::from_str(r#"{"error": "bad"}"#).unwrap();
        assert!(parse_vep_array(&json).is_err());
    }

    #[test]
    fn test_parse_vep_empty_consequences() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"allele_string": "A/G", "most_severe_consequence": "intergenic_variant"}]"#,
        )
        .unwrap();
        let results = parse_vep_array(&json).unwrap();
        assert_eq!(results[0].transcript_consequences.len(), 0);
    }

    #[test]
    fn test_transcript_consequence_serde() {
        let tc = TranscriptConsequence {
            gene_id: "ENSG1".into(),
            transcript_id: "ENST1".into(),
            consequence_terms: vec!["splice_donor_variant".into()],
            impact: "HIGH".into(),
            biotype: "protein_coding".into(),
        };
        let json = serde_json::to_value(&tc).unwrap();
        let back: TranscriptConsequence = serde_json::from_value(json).unwrap();
        assert_eq!(back.impact, "HIGH");
        assert_eq!(back.consequence_terms, vec!["splice_donor_variant"]);
    }

    #[test]
    fn test_parse_gene_null_fields() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "id": null,
                "display_name": null,
                "description": null,
                "species": null,
                "biotype": null,
                "start": null,
                "end": null,
                "strand": null,
                "seq_region_name": null
            }"#,
        )
        .unwrap();
        let gene = parse_gene(&json).unwrap();
        assert_eq!(gene.id, "");
        assert_eq!(gene.symbol, "");
        assert_eq!(gene.start, 0);
        assert_eq!(gene.end, 0);
        assert_eq!(gene.strand, 0);
        assert_eq!(gene.chromosome, "");
    }

    #[test]
    fn test_parse_vep_array_empty() {
        let json: serde_json::Value = serde_json::from_str(r#"[]"#).unwrap();
        let results = parse_vep_array(&json).unwrap();
        assert!(results.is_empty());
    }
}
