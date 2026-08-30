// Always available (WASM-safe)
pub mod annotation;
pub mod atac;
pub mod bio_ops;
pub mod bio_plots;
pub mod bio_wasm;
pub mod builtins;
pub mod checker;
pub mod chipseq;
pub mod cnv;
pub mod crispr;
pub mod csv;
pub mod datetime;
pub mod deconvolution;
pub mod drug;
pub mod env;
pub mod gpu;
pub mod graph;
pub mod gwas;
pub mod hash;
pub mod hic;
pub mod hmm;
pub mod immune;
pub mod interpreter;
pub mod json;
pub mod longread;
pub mod markdown;
pub mod matrix;
pub mod metabolomics;
pub mod methylation;
pub mod microbiome;
mod mosaic_plot;
pub mod motif;
pub mod ncbi_wasm;
pub mod network;
pub mod phylo;
pub mod plot;
pub mod popgen;
mod predictive;
pub mod proteomics;
pub mod qpcr;
pub mod regex_ops;
pub mod rnaseq;
pub mod seq;
pub mod singlecell;
pub mod sparse;
pub mod statistics;
pub mod stats;
mod stats_explore;
pub mod structure;
pub mod table_ops;
pub mod tempfiles;
pub mod text_ops;
pub mod value_export;
pub mod variants;
pub mod viz;

// Native-only (require filesystem, network, or subprocess)
#[cfg(feature = "native")]
pub mod apis;

pub mod capabilities;

pub mod provenance;

#[cfg(feature = "native")]
pub mod anndata_zarr;
#[cfg(feature = "native")]
pub mod blosc;
#[cfg(feature = "native")]
pub mod container;
#[cfg(feature = "native")]
pub mod enrich;
#[cfg(feature = "native")]
pub mod fs;
#[cfg(feature = "native")]
pub mod http;
#[cfg(feature = "native")]
pub mod interop;
#[cfg(feature = "native")]
pub mod llm;
#[cfg(feature = "native")]
pub mod nf_parse;
#[cfg(feature = "native")]
pub mod notify;
#[cfg(feature = "native")]
pub mod package;
#[cfg(feature = "native")]
pub mod parquet;
#[cfg(feature = "native")]
pub mod plugins;
#[cfg(feature = "native")]
pub mod references;
#[cfg(feature = "native")]
pub mod sqlite;
#[cfg(feature = "native")]
pub mod transfer;
#[cfg(feature = "native")]
pub mod workspace;

// Bytecode compiler + JIT VM (opt-in)
#[cfg(feature = "bytecode")]
pub mod compiled;

pub use interpreter::Interpreter;
