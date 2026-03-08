//! UCSC Genome Browser API client.
//!
//! Genomes, tracks, sequences, regions.
//! API docs: <https://genome.ucsc.edu/goldenPath/help/api.html>

use serde::{Deserialize, Serialize};

use crate::client::BaseClient;
use crate::config;
use crate::error::Result;

fn base_url() -> String {
    config::resolve_url("ucsc", "https://api.genome.ucsc.edu")
}

/// UCSC Genome Browser API client.
pub struct UcscClient {
    base: BaseClient,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub organism: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    #[serde(default, rename = "shortLabel")]
    pub short_label: String,
    #[serde(default, rename = "longLabel")]
    pub long_label: String,
    #[serde(default, rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chromosome {
    pub chrom: String,
    pub size: u64,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl UcscClient {
    pub fn new() -> Self {
        UcscClient {
            base: BaseClient::new(),
        }
    }

    pub fn with_client(base: BaseClient) -> Self {
        UcscClient { base }
    }

    /// List available genomes.
    pub fn list_genomes(&self) -> Result<Vec<Genome>> {
        let base = base_url();
        let url = format!("{base}/list/ucscGenomes");
        let json = self.base.get_json(&url)?;
        let mut genomes = Vec::new();
        if let Some(obj) = json["ucscGenomes"].as_object() {
            for (name, info) in obj {
                genomes.push(Genome {
                    name: name.clone(),
                    description: info["description"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    organism: info["organism"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
        genomes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(genomes)
    }

    /// List tracks for a genome assembly.
    pub fn list_tracks(&self, genome: &str) -> Result<Vec<Track>> {
        let base = base_url();
        let url = format!("{base}/list/tracks?genome={genome}");
        let json = self.base.get_json(&url)?;
        let mut tracks = Vec::new();
        if let Some(obj) = json[genome].as_object() {
            for (name, info) in obj {
                tracks.push(Track {
                    name: name.clone(),
                    short_label: info["shortLabel"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    long_label: info["longLabel"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    type_: info["type"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
        tracks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tracks)
    }

    /// Get DNA sequence for a genomic region.
    pub fn get_sequence(
        &self,
        genome: &str,
        chrom: &str,
        start: u64,
        end: u64,
    ) -> Result<String> {
        let base = base_url();
        let url = format!(
            "{base}/getData/sequence?genome={genome};chrom={chrom};start={start};end={end}"
        );
        let json = self.base.get_json(&url)?;
        Ok(json["dna"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Get track data for a genomic region.
    pub fn get_track_data(
        &self,
        genome: &str,
        track: &str,
        chrom: &str,
        start: u64,
        end: u64,
    ) -> Result<serde_json::Value> {
        let base = base_url();
        let url = format!(
            "{base}/getData/track?genome={genome};track={track};chrom={chrom};start={start};end={end}"
        );
        self.base.get_json(&url)
    }

    /// List chromosomes for a genome.
    pub fn list_chromosomes(&self, genome: &str) -> Result<Vec<Chromosome>> {
        let base = base_url();
        let url = format!("{base}/list/chromosomes?genome={genome}");
        let json = self.base.get_json(&url)?;
        let mut chroms = Vec::new();
        if let Some(obj) = json["chromosomes"].as_object() {
            for (name, size) in obj {
                chroms.push(Chromosome {
                    chrom: name.clone(),
                    size: size.as_u64().unwrap_or(0),
                });
            }
        }
        chroms.sort_by(|a, b| a.chrom.cmp(&b.chrom));
        Ok(chroms)
    }

    /// Search within a genome.
    pub fn search(&self, genome: &str, query: &str) -> Result<serde_json::Value> {
        let base = base_url();
        let url = format!("{base}/search?genome={genome};search={query}");
        self.base.get_json(&url)
    }
}

impl Default for UcscClient {
    fn default() -> Self {
        Self::new()
    }
}
