// Always available (WASM-safe)
pub mod bio_ops;
pub mod bio_plots;
pub mod bio_wasm;
pub mod builtins;
pub mod checker;
pub mod csv;
pub mod datetime;
pub mod env;
pub mod graph;
pub mod hash;
pub mod interpreter;
pub mod json;
pub mod markdown;
pub mod matrix;
pub mod ncbi_wasm;
pub mod plot;
pub mod regex_ops;
pub mod seq;
pub mod singlecell;
pub mod variants;
pub mod rnaseq;
pub mod phylo;
pub mod chipseq;
pub mod microbiome;
pub mod statistics;
pub mod qpcr;
pub mod proteomics;
pub mod methylation;
pub mod structure;
pub mod network;
pub mod popgen;
pub mod crispr;
pub mod immune;
pub mod deconvolution;
pub mod metabolomics;
pub mod longread;
pub mod motif;
pub mod cnv;
pub mod hic;
pub mod atac;
pub mod drug;
pub mod gwas;
pub mod annotation;
pub mod sparse;
pub mod stats;
pub mod table_ops;
pub mod tempfiles;
pub mod text_ops;
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
pub mod workspace;
#[cfg(feature = "native")]
pub mod container;
#[cfg(feature = "native")]
pub mod enrich;
#[cfg(feature = "native")]
pub mod fs;
#[cfg(feature = "native")]
pub mod http;
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
pub mod sqlite;
#[cfg(feature = "native")]
pub mod transfer;

// Bytecode compiler + JIT VM (opt-in)
#[cfg(feature = "bytecode")]
pub mod compiled;

pub use interpreter::Interpreter;
