//! GEO (Gene Expression Omnibus) client using NCBI E-utilities.
//!
//! No API key required (though NCBI_API_KEY improves rate limits).
//! API docs: <https://www.ncbi.nlm.nih.gov/geo/info/geo_paccess.html>

use crate::client::BaseClient;
use crate::error::{ApiError, Result};

const ESEARCH: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi";
const ESUMMARY: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi";

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GeoSeries {
    pub accession: String,
    pub title: String,
    pub summary: String,
    pub organism: String,
    pub n_samples: u64,
    pub platform: String,
    pub pubmed_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeoSample {
    pub accession: String,
    pub title: String,
    pub source: String,
    pub organism: String,
    pub characteristics: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeoPlatform {
    pub accession: String,
    pub title: String,
    pub organism: String,
    pub technology: String,
    pub n_samples: u64,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct GeoClient {
    base: BaseClient,
}

impl GeoClient {
    pub fn new() -> Self {
        GeoClient {
            base: BaseClient::new(),
        }
    }

    fn api_key_suffix(&self) -> String {
        if let Some(key) = self.base.get_api_key("NCBI_API_KEY") {
            format!("&api_key={key}")
        } else {
            String::new()
        }
    }

    fn esearch_ids(&self, db: &str, term: &str, max: usize) -> Result<Vec<String>> {
        let url = format!(
            "{ESEARCH}?db={db}&term={}&retmax={max}&retmode=json{}",
            encode(term),
            self.api_key_suffix()
        );
        let json = self.base.get_json(&url)?;
        let ids = json["esearchresult"]["idlist"]
            .as_array()
            .ok_or_else(|| ApiError::Parse {
                context: "GEO esearch".into(),
                source: "missing idlist".into(),
            })?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        Ok(ids)
    }

    fn esummary_gds(&self, ids: &[String]) -> Result<Vec<GeoSeries>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let id_str = ids.join(",");
        let url = format!(
            "{ESUMMARY}?db=gds&id={id_str}&retmode=json{}",
            self.api_key_suffix()
        );
        let json = self.base.get_json(&url)?;
        parse_gds_summaries(&json)
    }

    /// Search GEO datasets by free-text query.
    pub fn search(&self, query: &str, max: usize) -> Result<Vec<GeoSeries>> {
        let ids = self.esearch_ids("gds", query, max)?;
        self.esummary_gds(&ids)
    }

    /// Fetch a single GEO series by accession (e.g. "GSE12345").
    pub fn series(&self, accession: &str) -> Result<GeoSeries> {
        let term = format!("{accession}[Accession]");
        let ids = self.esearch_ids("gds", &term, 1)?;
        let mut results = self.esummary_gds(&ids)?;
        results.into_iter().next().ok_or_else(|| ApiError::Parse {
            context: "GEO series".into(),
            source: format!("no series found for {accession}"),
        })
    }

    /// List samples belonging to a GEO series.
    pub fn samples(&self, series_accession: &str, max: usize) -> Result<Vec<GeoSample>> {
        let term = format!("{series_accession}[Series Accession Number]");
        let ids = self.esearch_ids("gsm", &term, max)?;
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let id_str = ids.join(",");
        let url = format!(
            "{ESUMMARY}?db=gsm&id={id_str}&retmode=json{}",
            self.api_key_suffix()
        );
        let json = self.base.get_json(&url)?;
        parse_gsm_summaries(&json)
    }

    /// List GEO platforms for an organism.
    pub fn platforms(&self, organism: &str, max: usize) -> Result<Vec<GeoPlatform>> {
        let term = format!("{organism}[Organism] AND gpl[Entry Type]");
        let ids = self.esearch_ids("gds", &term, max)?;
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let id_str = ids.join(",");
        let url = format!(
            "{ESUMMARY}?db=gds&id={id_str}&retmode=json{}",
            self.api_key_suffix()
        );
        let json = self.base.get_json(&url)?;
        parse_gpl_summaries(&json)
    }
}

impl Default for GeoClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

fn parse_gds_summaries(json: &serde_json::Value) -> Result<Vec<GeoSeries>> {
    let result = json.get("result").ok_or_else(|| ApiError::Parse {
        context: "GEO esummary".into(),
        source: "missing 'result' field".into(),
    })?;

    let uids: Vec<String> = result["uids"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut series = Vec::new();
    for uid in &uids {
        let entry = &result[uid.as_str()];
        if entry.is_null() {
            continue;
        }
        let accession = entry["Accession"]
            .as_str()
            .or_else(|| entry["accession"].as_str())
            .unwrap_or_default()
            .to_string();
        // Only include GEO Series (GSE prefix); skip GPLs in a gds query
        let pubmed_ids: Vec<String> = entry["PubMedIds"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .or_else(|| v.as_u64().map(|n| n.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        series.push(GeoSeries {
            accession,
            title: entry["title"].as_str().unwrap_or_default().to_string(),
            summary: entry["summary"].as_str().unwrap_or_default().to_string(),
            organism: entry["organism"].as_str().unwrap_or_default().to_string(),
            n_samples: entry["n_samples"]
                .as_u64()
                .or_else(|| entry["n_samples"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0),
            platform: entry["GPL"]
                .as_str()
                .map(|s| format!("GPL{s}"))
                .unwrap_or_default(),
            pubmed_ids,
        });
    }
    Ok(series)
}

fn parse_gsm_summaries(json: &serde_json::Value) -> Result<Vec<GeoSample>> {
    let result = json.get("result").ok_or_else(|| ApiError::Parse {
        context: "GEO gsm esummary".into(),
        source: "missing 'result' field".into(),
    })?;

    let uids: Vec<String> = result["uids"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut samples = Vec::new();
    for uid in &uids {
        let entry = &result[uid.as_str()];
        if entry.is_null() {
            continue;
        }
        let characteristics: Vec<String> = entry["characteristics_ch1"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_else(|| {
                entry["characteristics_ch1"]
                    .as_str()
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default()
            });

        samples.push(GeoSample {
            accession: entry["Accession"].as_str().unwrap_or_default().to_string(),
            title: entry["title"].as_str().unwrap_or_default().to_string(),
            source: entry["source_name_ch1"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            organism: entry["organism_ch1"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            characteristics,
        });
    }
    Ok(samples)
}

fn parse_gpl_summaries(json: &serde_json::Value) -> Result<Vec<GeoPlatform>> {
    let result = json.get("result").ok_or_else(|| ApiError::Parse {
        context: "GEO gpl esummary".into(),
        source: "missing 'result' field".into(),
    })?;

    let uids: Vec<String> = result["uids"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut platforms = Vec::new();
    for uid in &uids {
        let entry = &result[uid.as_str()];
        if entry.is_null() {
            continue;
        }
        platforms.push(GeoPlatform {
            accession: entry["Accession"].as_str().unwrap_or_default().to_string(),
            title: entry["title"].as_str().unwrap_or_default().to_string(),
            organism: entry["organism"].as_str().unwrap_or_default().to_string(),
            technology: entry["technology"].as_str().unwrap_or_default().to_string(),
            n_samples: entry["n_samples"].as_u64().unwrap_or(0),
        });
    }
    Ok(platforms)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn gds_summary_json() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "uids": ["200012345"],
                "200012345": {
                    "Accession": "GSE12345",
                    "title": "RNA-seq of human cancer cell lines",
                    "summary": "Transcriptome profiling of 10 cancer cell lines.",
                    "organism": "Homo sapiens",
                    "n_samples": 30,
                    "GPL": "570",
                    "PubMedIds": ["12345678"]
                }
            }
        })
    }

    fn gsm_summary_json() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "uids": ["300001"],
                "300001": {
                    "Accession": "GSM300001",
                    "title": "HeLa cells untreated rep1",
                    "source_name_ch1": "HeLa",
                    "organism_ch1": "Homo sapiens",
                    "characteristics_ch1": ["treatment: none", "passage: 10"]
                }
            }
        })
    }

    fn gpl_summary_json() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "uids": ["570"],
                "570": {
                    "Accession": "GPL570",
                    "title": "Affymetrix Human Genome U133 Plus 2.0 Array",
                    "organism": "Homo sapiens",
                    "technology": "in situ oligonucleotide",
                    "n_samples": 100000
                }
            }
        })
    }

    #[test]
    fn parse_gds_series() {
        let json = gds_summary_json();
        let series = parse_gds_summaries(&json).unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].accession, "GSE12345");
        assert_eq!(series[0].title, "RNA-seq of human cancer cell lines");
        assert_eq!(series[0].n_samples, 30);
        assert_eq!(series[0].platform, "GPL570");
        assert_eq!(series[0].pubmed_ids, vec!["12345678"]);
    }

    #[test]
    fn parse_gsm_samples() {
        let json = gsm_summary_json();
        let samples = parse_gsm_summaries(&json).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].accession, "GSM300001");
        assert_eq!(samples[0].source, "HeLa");
        assert_eq!(samples[0].organism, "Homo sapiens");
        assert_eq!(samples[0].characteristics.len(), 2);
        assert_eq!(samples[0].characteristics[0], "treatment: none");
    }

    #[test]
    fn parse_gpl_platforms() {
        let json = gpl_summary_json();
        let platforms = parse_gpl_summaries(&json).unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].accession, "GPL570");
        assert_eq!(platforms[0].technology, "in situ oligonucleotide");
        assert_eq!(platforms[0].n_samples, 100000);
    }

    #[test]
    fn parse_empty_result() {
        let json = serde_json::json!({"result": {"uids": []}});
        let series = parse_gds_summaries(&json).unwrap();
        assert!(series.is_empty());
    }

    #[test]
    fn parse_missing_result_field() {
        let json = serde_json::json!({"something_else": {}});
        assert!(parse_gds_summaries(&json).is_err());
    }

    #[test]
    fn parse_gds_zero_samples() {
        let json = serde_json::json!({
            "result": {
                "uids": ["999"],
                "999": {
                    "Accession": "GSE999",
                    "title": "Empty study",
                    "summary": "",
                    "organism": "Mus musculus",
                    "n_samples": 0,
                    "GPL": "1261",
                    "PubMedIds": []
                }
            }
        });
        let series = parse_gds_summaries(&json).unwrap();
        assert_eq!(series[0].n_samples, 0);
        assert!(series[0].pubmed_ids.is_empty());
    }
}
