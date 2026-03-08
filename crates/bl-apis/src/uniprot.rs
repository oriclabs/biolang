//! UniProt REST API client.
//!
//! Protein search, entry retrieval, features, GO terms, ID mapping.
//! API docs: <https://www.uniprot.org/help/api>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::{ApiError, Result};

fn base_url() -> String {
    config::resolve_url("uniprot", "https://rest.uniprot.org")
}

/// UniProt REST API client.
pub struct UniProtClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinEntry {
    pub accession: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub organism: String,
    #[serde(default)]
    pub sequence_length: u64,
    #[serde(default)]
    pub gene_names: Vec<String>,
    #[serde(default)]
    pub function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    #[serde(default, rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTerm {
    pub id: String,
    #[serde(default)]
    pub term: String,
    #[serde(default)]
    pub aspect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdMapping {
    pub from: String,
    pub to: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl UniProtClient {
    pub fn new() -> Self {
        UniProtClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        UniProtClient { base }
    }

    /// Search UniProt entries.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<ProteinEntry>> {
        let base = base_url();
        let url = format!(
            "{base}/uniprotkb/search?query={query}&format=json&size={limit}"
        );
        let json = self.base.get_json(&url)?;
        let results = json["results"]
            .as_array()
            .ok_or_else(|| ApiError::Parse {
                context: "UniProt search".into(),
                source: "missing results array".into(),
            })?;

        let mut entries = Vec::new();
        for r in results {
            entries.push(parse_entry(r));
        }
        Ok(entries)
    }

    /// Get a single protein entry by accession.
    pub fn entry(&self, accession: &str) -> Result<ProteinEntry> {
        let base = base_url();
        let url = format!("{base}/uniprotkb/{accession}.json");
        let json = self.base.get_json(&url)?;
        Ok(parse_entry(&json))
    }

    /// Get a protein entry in FASTA format.
    pub fn entry_fasta(&self, accession: &str) -> Result<String> {
        let base = base_url();
        let url = format!("{base}/uniprotkb/{accession}.fasta");
        self.base.get_text(&url)
    }

    /// Get features (domains, binding sites, etc.) for a protein.
    pub fn features(&self, accession: &str) -> Result<Vec<Feature>> {
        let base = base_url();
        let url = format!("{base}/uniprotkb/{accession}.json");
        let json = self.base.get_json(&url)?;

        let feats = json["features"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|f| Feature {
                        type_: f["type"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        location: format_location(f),
                        description: f["description"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(feats)
    }

    /// Get GO terms associated with a protein.
    pub fn go_terms(&self, accession: &str) -> Result<Vec<GoTerm>> {
        let base = base_url();
        let url = format!("{base}/uniprotkb/{accession}.json");
        let json = self.base.get_json(&url)?;

        let mut terms = Vec::new();
        if let Some(refs) = json["uniProtKBCrossReferences"].as_array() {
            for r in refs {
                if r["database"].as_str() == Some("GO") {
                    let id = r["id"].as_str().unwrap_or_default().to_string();
                    let mut term = String::new();
                    let mut aspect = String::new();
                    if let Some(props) = r["properties"].as_array() {
                        for p in props {
                            if let Some("GoTerm") = p["key"].as_str() {
                                let val = p["value"].as_str().unwrap_or_default();
                                // Format: "C:membrane" or "F:binding" or "P:signaling"
                                if let Some(pos) = val.find(':') {
                                    aspect = val[..pos].to_string();
                                    term = val[pos + 1..].to_string();
                                } else {
                                    term = val.to_string();
                                }
                            }
                        }
                    }
                    terms.push(GoTerm { id, term, aspect });
                }
            }
        }
        Ok(terms)
    }

    /// Submit an ID mapping job. Returns a job ID.
    pub fn id_mapping_submit(
        &self,
        from_db: &str,
        to_db: &str,
        ids: &[&str],
    ) -> Result<String> {
        let ids_str = ids.join(",");
        let base = base_url();
        let url = format!("{base}/idmapping/run");
        let body = serde_json::json!({
            "from": from_db,
            "to": to_db,
            "ids": ids_str,
        });
        let resp = self.base.post_json(&url, &body)?;
        resp["jobId"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ApiError::Parse {
                context: "ID mapping submit".into(),
                source: "missing jobId".into(),
            })
    }

    /// Get ID mapping results (poll until complete).
    pub fn id_mapping_results(&self, job_id: &str) -> Result<Vec<IdMapping>> {
        let base = base_url();
        let url = format!("{base}/idmapping/results/{job_id}");
        let json = self.base.get_json(&url)?;
        let results = json["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| IdMapping {
                        from: r["from"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        to: r["to"]["primaryAccession"]
                            .as_str()
                            .or_else(|| r["to"].as_str())
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(results)
    }
}

impl Default for UniProtClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_entry(json: &serde_json::Value) -> ProteinEntry {
    let accession = json["primaryAccession"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let name = json["proteinDescription"]["recommendedName"]["fullName"]["value"]
        .as_str()
        .or_else(|| {
            json["proteinDescription"]["submissionNames"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|n| n["fullName"]["value"].as_str())
        })
        .unwrap_or_default()
        .to_string();

    let organism = json["organism"]["scientificName"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let sequence_length = json["sequence"]["length"].as_u64().unwrap_or(0);

    let gene_names: Vec<String> = json["genes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g["geneName"]["value"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let function = json["comments"]
        .as_array()
        .and_then(|comments| {
            comments.iter().find_map(|c| {
                if c["commentType"].as_str() == Some("FUNCTION") {
                    c["texts"]
                        .as_array()
                        .and_then(|t| t.first())
                        .and_then(|t| t["value"].as_str())
                        .map(String::from)
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    ProteinEntry {
        accession,
        name,
        organism,
        sequence_length,
        gene_names,
        function,
    }
}

fn format_location(f: &serde_json::Value) -> String {
    let loc = &f["location"];
    let start = loc["start"]["value"].as_u64().unwrap_or(0);
    let end = loc["end"]["value"].as_u64().unwrap_or(0);
    if start == 0 && end == 0 {
        String::new()
    } else {
        format!("{start}..{end}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entry_full() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "primaryAccession": "P38398",
                "proteinDescription": {
                    "recommendedName": {
                        "fullName": {"value": "Breast cancer type 1 susceptibility protein"}
                    }
                },
                "organism": {"scientificName": "Homo sapiens"},
                "sequence": {"length": 1863},
                "genes": [{"geneName": {"value": "BRCA1"}}],
                "comments": [{
                    "commentType": "FUNCTION",
                    "texts": [{"value": "E3 ubiquitin-protein ligase"}]
                }]
            }"#,
        )
        .unwrap();
        let entry = parse_entry(&json);
        assert_eq!(entry.accession, "P38398");
        assert_eq!(entry.name, "Breast cancer type 1 susceptibility protein");
        assert_eq!(entry.organism, "Homo sapiens");
        assert_eq!(entry.sequence_length, 1863);
        assert_eq!(entry.gene_names, vec!["BRCA1"]);
        assert_eq!(entry.function, "E3 ubiquitin-protein ligase");
    }

    #[test]
    fn test_parse_entry_submission_name_fallback() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "primaryAccession": "A0A000",
                "proteinDescription": {
                    "submissionNames": [{"fullName": {"value": "Uncharacterized protein"}}]
                },
                "organism": {"scientificName": "E. coli"},
                "sequence": {"length": 100},
                "genes": []
            }"#,
        )
        .unwrap();
        let entry = parse_entry(&json);
        assert_eq!(entry.name, "Uncharacterized protein");
        assert!(entry.gene_names.is_empty());
        assert_eq!(entry.function, "");
    }

    #[test]
    fn test_parse_entry_minimal() {
        let json: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        let entry = parse_entry(&json);
        assert_eq!(entry.accession, "");
        assert_eq!(entry.name, "");
        assert_eq!(entry.sequence_length, 0);
    }

    #[test]
    fn test_format_location() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"location": {"start": {"value": 10}, "end": {"value": 50}}}"#,
        )
        .unwrap();
        assert_eq!(format_location(&json), "10..50");
    }

    #[test]
    fn test_format_location_zero() {
        let json: serde_json::Value = serde_json::from_str(r#"{"location": {}}"#).unwrap();
        assert_eq!(format_location(&json), "");
    }

    #[test]
    fn test_protein_entry_serde() {
        let entry = ProteinEntry {
            accession: "P12345".into(),
            name: "Test protein".into(),
            organism: "Homo sapiens".into(),
            sequence_length: 500,
            gene_names: vec!["TP53".into()],
            function: "Tumor suppressor".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: ProteinEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.accession, "P12345");
        assert_eq!(back.sequence_length, 500);
    }

    #[test]
    fn test_parse_entry_multiple_gene_names() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "primaryAccession": "P04637",
                "organism": {"scientificName": "Homo sapiens"},
                "sequence": {"length": 393},
                "genes": [
                    {"geneName": {"value": "TP53"}},
                    {"geneName": {"value": "P53"}}
                ]
            }"#,
        )
        .unwrap();
        let entry = parse_entry(&json);
        assert_eq!(entry.gene_names.len(), 2);
        assert_eq!(entry.gene_names[0], "TP53");
        assert_eq!(entry.gene_names[1], "P53");
    }

    #[test]
    fn test_parse_entry_no_function_comments() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "primaryAccession": "Q12345",
                "organism": {"scientificName": "Homo sapiens"},
                "sequence": {"length": 100},
                "comments": [
                    {"commentType": "SUBCELLULAR LOCATION", "texts": [{"value": "Nucleus"}]}
                ]
            }"#,
        )
        .unwrap();
        let entry = parse_entry(&json);
        assert_eq!(entry.function, "");
    }

    #[test]
    fn test_format_location_missing_start() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"location": {"end": {"value": 50}}}"#,
        )
        .unwrap();
        // start defaults to 0, end is 50 => "0..50"
        assert_eq!(format_location(&json), "0..50");
    }

    #[test]
    fn test_format_location_missing_end() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"location": {"start": {"value": 10}}}"#,
        )
        .unwrap();
        // start is 10, end defaults to 0 => "10..0"
        assert_eq!(format_location(&json), "10..0");
    }
}
