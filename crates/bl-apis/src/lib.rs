//! Typed bioinformatics API clients.
//!
//! Blocking HTTP via `ureq` — suitable for direct use or wrapping
//! in `spawn_blocking` for async contexts.

pub mod error;
pub mod client;
pub mod config;

pub mod ncbi;
pub mod ensembl;
pub mod uniprot;
pub mod ucsc;
pub mod biomart;
pub mod kegg;
pub mod string_db;
pub mod pdb;
pub mod reactome;
pub mod go;
pub mod cosmic;
pub mod ncbi_datasets;
pub mod biocontainers;
pub mod nfcore;
pub mod galaxy;

pub use error::{ApiError, Result};
pub use client::BaseClient;
pub use ncbi::NcbiClient;
pub use ensembl::EnsemblClient;
pub use uniprot::UniProtClient;
pub use ucsc::UcscClient;
pub use biomart::BioMartClient;
pub use kegg::KeggClient;
pub use string_db::StringDbClient;
pub use pdb::PdbClient;
pub use reactome::ReactomeClient;
pub use go::GoClient;
pub use cosmic::CosmicClient;
pub use ncbi_datasets::NcbiDatasetsClient;
pub use biocontainers::BioContainersClient;
pub use nfcore::NfCoreClient;
pub use galaxy::GalaxyToolShedClient;
