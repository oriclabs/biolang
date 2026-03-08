//! KEGG REST API client.
//!
//! Pathways, compounds, reactions, genes.
//! API docs: <https://www.kegg.jp/kegg/rest/keggapi.html>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("kegg", "https://rest.kegg.jp")
}

/// KEGG REST API client.
pub struct KeggClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeggEntry {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeggLink {
    pub source: String,
    pub target: String,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl KeggClient {
    pub fn new() -> Self {
        KeggClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        KeggClient { base }
    }

    /// Get a KEGG entry (raw text). e.g. `get("hsa:10458")`.
    pub fn get(&self, entry: &str) -> Result<String> {
        let base = base_url();
        let url = format!("{base}/get/{entry}");
        self.base.get_text(&url)
    }

    /// Find entries in a KEGG database. e.g. `find("genes", "shiga toxin")`.
    pub fn find(&self, db: &str, query: &str) -> Result<Vec<KeggEntry>> {
        let base = base_url();
        let url = format!("{base}/find/{db}/{query}");
        let text = self.base.get_text(&url)?;
        Ok(parse_tab_lines(&text))
    }

    /// List all entries in a KEGG database.
    pub fn list(&self, db: &str) -> Result<Vec<KeggEntry>> {
        let base = base_url();
        let url = format!("{base}/list/{db}");
        let text = self.base.get_text(&url)?;
        Ok(parse_tab_lines(&text))
    }

    /// Cross-reference between databases. e.g. `link("pathway", "hsa:10458")`.
    pub fn link(&self, target_db: &str, source: &str) -> Result<Vec<KeggLink>> {
        let base = base_url();
        let url = format!("{base}/link/{target_db}/{source}");
        let text = self.base.get_text(&url)?;
        let mut links = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                links.push(KeggLink {
                    source: parts[0].to_string(),
                    target: parts[1].to_string(),
                });
            }
        }
        Ok(links)
    }

    /// Get a pathway entry (raw text).
    pub fn get_pathway(&self, pathway_id: &str) -> Result<String> {
        self.get(pathway_id)
    }

    /// Search for genes in a specific organism. e.g. `find_genes("hsa", "cancer")`.
    pub fn find_genes(&self, organism: &str, query: &str) -> Result<Vec<KeggEntry>> {
        let base = base_url();
        let url = format!("{base}/find/{organism}/{query}");
        let text = self.base.get_text(&url)?;
        Ok(parse_tab_lines(&text))
    }
}

impl Default for KeggClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse KEGG tab-separated response lines.
fn parse_tab_lines(text: &str) -> Vec<KeggEntry> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (id, desc) = match line.split_once('\t') {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => (line.to_string(), String::new()),
        };
        entries.push(KeggEntry {
            id,
            description: desc,
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tab_lines_normal() {
        let text = "hsa:10458\tBASP1; brain abundant, membrane attached signal protein 1\nhsa:4609\tMYC; MYC proto-oncogene\n";
        let entries = parse_tab_lines(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "hsa:10458");
        assert!(entries[0].description.contains("BASP1"));
        assert_eq!(entries[1].id, "hsa:4609");
    }

    #[test]
    fn test_parse_tab_lines_no_description() {
        let text = "hsa:10458\n";
        let entries = parse_tab_lines(text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "hsa:10458");
        assert_eq!(entries[0].description, "");
    }

    #[test]
    fn test_parse_tab_lines_empty() {
        let entries = parse_tab_lines("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_tab_lines_blank_lines() {
        let text = "hsa:1\tgene1\n\n\nhsa:2\tgene2\n";
        let entries = parse_tab_lines(text);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_kegg_entry_serde() {
        let entry = KeggEntry {
            id: "hsa:4609".into(),
            description: "MYC proto-oncogene".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: KeggEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "hsa:4609");
    }

    #[test]
    fn test_parse_tab_lines_tab_only() {
        // Lines containing only a tab are trimmed to empty and skipped
        let text = "\t\n\t\n";
        let entries = parse_tab_lines(text);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_parse_tab_lines_very_long_description() {
        let long_desc = "A".repeat(10_000);
        let text = format!("hsa:999\t{long_desc}\n");
        let entries = parse_tab_lines(&text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "hsa:999");
        assert_eq!(entries[0].description.len(), 10_000);
    }
}
