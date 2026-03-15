// Always available (WASM-safe)
pub mod builtins;
pub mod checker;
pub mod datetime;
pub mod env;
pub mod hash;
pub mod interpreter;
pub mod json;
pub mod matrix;
pub mod plot;
pub mod csv;
pub mod regex_ops;
pub mod sparse;
pub mod bio_ops;
pub mod stats;
pub mod table_ops;
pub mod text_ops;
pub mod viz;
pub mod bio_plots;
pub mod graph;
pub mod markdown;
pub mod seq;
pub mod ncbi_wasm;
pub mod bio_wasm;
pub mod tempfiles;

// Native-only (require filesystem, network, or subprocess)
#[cfg(feature = "native")]
pub mod apis;
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
pub mod plugins;
#[cfg(feature = "native")]
pub mod transfer;
#[cfg(feature = "native")]
pub mod nf_parse;
#[cfg(feature = "native")]
pub mod notify;
#[cfg(feature = "native")]
pub mod sqlite;
#[cfg(feature = "native")]
pub mod parquet;
#[cfg(feature = "native")]
pub mod package;

// Bytecode compiler + JIT VM (opt-in)
#[cfg(feature = "bytecode")]
pub mod compiled;

pub use interpreter::Interpreter;
