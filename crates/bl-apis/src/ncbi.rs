//! NCBI E-utilities client.
//!
//! Covers databases: gene, pubmed, nucleotide, protein, snp, clinvar, taxonomy, etc.
//! API docs: <https://www.ncbi.nlm.nih.gov/books/NBK25497/>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("ncbi", "https://eutils.ncbi.nlm.nih.gov/entrez/eutils")
}

/// NCBI E-utilities client.
pub struct NcbiClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub ids: Vec<String>,
    pub count: u64,
    #[serde(default)]
    pub webenv: Option<String>,
    #[serde(default)]
    pub query_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneSummary {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub organism: String,
    #[serde(default)]
    pub chromosome: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSummary {
    pub uid: String,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub db_name: String,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    pub links: Vec<LinkSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkSet {
    pub dbto: String,
    pub ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl NcbiClient {
    pub fn new() -> Self {
        NcbiClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        NcbiClient { base }
    }

    fn api_key_param(&self) -> String {
        match self.base.get_api_key("NCBI_API_KEY") {
            Some(k) => format!("&api_key={k}"),
            None => String::new(),
        }
    }

    // -- Core E-utilities --

    /// Search an NCBI database. Returns matching IDs.
    pub fn esearch(&self, db: &str, term: &str, max: usize) -> Result<SearchResult> {
        let base = base_url();
        let encoded_term = urlencod(term);
        let url = format!(
            "{base}/esearch.fcgi?db={db}&term={encoded_term}&retmax={max}&retmode=json&usehistory=y{}",
            self.api_key_param()
        );
        let json = self.base.get_json(&url)?;

        let result = &json["esearchresult"];
        let ids: Vec<String> = result["idlist"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let count = result["count"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let webenv = result["webenv"].as_str().map(String::from);
        let query_key = result["querykey"].as_str().map(String::from);

        Ok(SearchResult {
            ids,
            count,
            webenv,
            query_key,
        })
    }

    /// Fetch document summaries by IDs.
    pub fn esummary(&self, db: &str, ids: &[&str]) -> Result<Vec<DocSummary>> {
        let base = base_url();
        let id_str = ids.join(",");
        let url = format!(
            "{base}/esummary.fcgi?db={db}&id={id_str}&retmode=json{}",
            self.api_key_param()
        );
        let json = self.base.get_json(&url)?;

        let result = &json["result"];
        let uids: Vec<String> = result["uids"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut summaries = Vec::new();
        for uid in &uids {
            if let Some(obj) = result[uid].as_object() {
                summaries.push(DocSummary {
                    uid: uid.clone(),
                    fields: obj.clone(),
                });
            }
        }
        Ok(summaries)
    }

    /// Fetch records as text (specify rettype: "fasta", "gb", "xml", etc.).
    pub fn efetch_text(&self, db: &str, ids: &[&str], rettype: &str) -> Result<String> {
        let base = base_url();
        let id_str = ids.join(",");
        let url = format!(
            "{base}/efetch.fcgi?db={db}&id={id_str}&rettype={rettype}&retmode=text{}",
            self.api_key_param()
        );
        self.base.get_text(&url)
    }

    /// Fetch records in FASTA format.
    pub fn efetch_fasta(&self, db: &str, ids: &[&str]) -> Result<String> {
        self.efetch_text(db, ids, "fasta")
    }

    /// Get database information.
    pub fn einfo(&self, db: &str) -> Result<DbInfo> {
        let base = base_url();
        let url = format!(
            "{base}/einfo.fcgi?db={db}&retmode=json{}",
            self.api_key_param()
        );
        let json = self.base.get_json(&url)?;
        let info = &json["einforesult"]["dbinfo"];
        Ok(DbInfo {
            db_name: info["dbname"]
                .as_str()
                .unwrap_or(db)
                .to_string(),
            count: info["count"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            description: info["description"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// Link records across databases.
    pub fn elink(&self, dbfrom: &str, dbto: &str, ids: &[&str]) -> Result<LinkResult> {
        let base = base_url();
        let id_str = ids.join(",");
        let url = format!(
            "{base}/elink.fcgi?dbfrom={dbfrom}&db={dbto}&id={id_str}&retmode=json{}",
            self.api_key_param()
        );
        let json = self.base.get_json(&url)?;

        let mut link_sets = Vec::new();
        if let Some(sets) = json["linksets"].as_array() {
            for set in sets {
                if let Some(dbs) = set["linksetdbs"].as_array() {
                    for db_entry in dbs {
                        let dbto_name = db_entry["dbto"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string();
                        let link_ids: Vec<String> = db_entry["links"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| {
                                        v["id"].as_str().map(String::from)
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        link_sets.push(LinkSet {
                            dbto: dbto_name,
                            ids: link_ids,
                        });
                    }
                }
            }
        }
        Ok(LinkResult { links: link_sets })
    }

    // -- Convenience methods --

    /// Search the gene database.
    pub fn search_gene(&self, term: &str, max: usize) -> Result<SearchResult> {
        self.esearch("gene", term, max)
    }

    /// Search PubMed.
    pub fn search_pubmed(&self, term: &str, max: usize) -> Result<SearchResult> {
        self.esearch("pubmed", term, max)
    }

    /// Search SNP database.
    pub fn search_snp(&self, term: &str, max: usize) -> Result<SearchResult> {
        self.esearch("snp", term, max)
    }

    /// Search ClinVar.
    pub fn search_clinvar(&self, term: &str, max: usize) -> Result<SearchResult> {
        self.esearch("clinvar", term, max)
    }

    /// Fetch gene summaries (parsed from esummary).
    pub fn fetch_gene_summary(&self, ids: &[&str]) -> Result<Vec<GeneSummary>> {
        let docs = self.esummary("gene", ids)?;
        let mut genes = Vec::new();
        for doc in docs {
            genes.push(GeneSummary {
                id: doc.uid.clone(),
                name: doc.fields.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                symbol: doc.fields.get("nomenclaturesymbol")
                    .or_else(|| doc.fields.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: doc.fields.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                organism: doc.fields.get("organism")
                    .and_then(|v| v.get("scientificname"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                chromosome: doc.fields.get("chromosome")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                location: doc.fields.get("maplocation")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                summary: doc.fields.get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        Ok(genes)
    }

    /// Fetch a single sequence in FASTA format.
    pub fn fetch_sequence(&self, id: &str) -> Result<String> {
        self.efetch_fasta("nucleotide", &[id])
    }
}

impl Default for NcbiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal URL encoding for query terms.
fn urlencod(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencod_simple() {
        assert_eq!(urlencod("BRCA1"), "BRCA1");
    }

    #[test]
    fn test_urlencod_spaces() {
        assert_eq!(urlencod("human BRCA1"), "human+BRCA1");
    }

    #[test]
    fn test_urlencod_special_chars() {
        let encoded = urlencod("gene[name]");
        assert!(encoded.contains("%5B"));
        assert!(encoded.contains("%5D"));
    }

    #[test]
    fn test_search_result_serde_roundtrip() {
        let sr = SearchResult {
            ids: vec!["672".into(), "675".into()],
            count: 2,
            webenv: Some("MCID_abc".into()),
            query_key: Some("1".into()),
        };
        let json = serde_json::to_value(&sr).unwrap();
        let back: SearchResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.ids, vec!["672", "675"]);
        assert_eq!(back.count, 2);
        assert_eq!(back.webenv.as_deref(), Some("MCID_abc"));
    }

    #[test]
    fn test_gene_summary_from_doc() {
        let fields: serde_json::Map<String, serde_json::Value> = serde_json::from_str(
            r#"{
                "name": "BRCA1",
                "nomenclaturesymbol": "BRCA1",
                "description": "BRCA1 DNA repair associated",
                "organism": {"scientificname": "Homo sapiens"},
                "chromosome": "17",
                "maplocation": "17q21.31",
                "summary": "Tumor suppressor"
            }"#,
        )
        .unwrap();
        let doc = DocSummary {
            uid: "672".into(),
            fields,
        };
        let gs = GeneSummary {
            id: doc.uid.clone(),
            name: doc.fields.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            symbol: doc.fields.get("nomenclaturesymbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            description: doc.fields.get("description").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            organism: doc.fields.get("organism").and_then(|v| v.get("scientificname")).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            chromosome: doc.fields.get("chromosome").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            location: doc.fields.get("maplocation").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            summary: doc.fields.get("summary").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        };
        assert_eq!(gs.id, "672");
        assert_eq!(gs.symbol, "BRCA1");
        assert_eq!(gs.organism, "Homo sapiens");
        assert_eq!(gs.chromosome, "17");
    }

    #[test]
    fn test_db_info_serde() {
        let info = DbInfo {
            db_name: "gene".into(),
            count: 50000,
            description: "Gene database".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DbInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.db_name, "gene");
        assert_eq!(back.count, 50000);
    }

    #[test]
    fn test_link_result_serde() {
        let lr = LinkResult {
            links: vec![LinkSet {
                dbto: "protein".into(),
                ids: vec!["NP_001".into(), "NP_002".into()],
            }],
        };
        let json = serde_json::to_string(&lr).unwrap();
        let back: LinkResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.links.len(), 1);
        assert_eq!(back.links[0].dbto, "protein");
        assert_eq!(back.links[0].ids.len(), 2);
    }

    #[test]
    fn test_urlencod_unicode() {
        let encoded = urlencod("\u{00e9}");  // e-acute
        // Non-ASCII bytes should be percent-encoded
        assert!(encoded.contains('%'));
        assert!(!encoded.contains('\u{00e9}'));
    }

    #[test]
    fn test_urlencod_empty() {
        assert_eq!(urlencod(""), "");
    }
}
