//! Typed bioinformatics API clients.
//!
//! Blocking HTTP via `ureq` — suitable for direct use or wrapping
//! in `spawn_blocking` for async contexts.

pub mod client;
pub mod config;
pub mod error;

pub mod biocontainers;
pub mod biomart;
pub mod cbioportal;
pub mod clinvar;
pub mod cosmic;
pub mod ensembl;
pub mod galaxy;
pub mod geo;
pub mod gnomad;
pub mod go;
pub mod gtex;
pub mod kegg;
pub mod ncbi;
pub mod ncbi_datasets;
pub mod nfcore;
pub mod opentargets;
pub mod pdb;
pub mod reactome;
pub mod string_db;
pub mod ucsc;
pub mod uniprot;

pub use biocontainers::BioContainersClient;
pub use biomart::BioMartClient;
pub use cbioportal::CbioPortalClient;
pub use clinvar::ClinVarClient;
pub use client::BaseClient;
pub use cosmic::CosmicClient;
pub use ensembl::EnsemblClient;
pub use error::{ApiError, Result};
pub use galaxy::GalaxyToolShedClient;
pub use geo::GeoClient;
pub use gnomad::GnomadClient;
pub use go::GoClient;
pub use gtex::GtexClient;
pub use kegg::KeggClient;
pub use ncbi::NcbiClient;
pub use ncbi_datasets::NcbiDatasetsClient;
pub use nfcore::NfCoreClient;
pub use opentargets::OpenTargetsClient;
pub use pdb::PdbClient;
pub use reactome::ReactomeClient;
pub use string_db::StringDbClient;
pub use ucsc::UcscClient;
pub use uniprot::UniProtClient;
