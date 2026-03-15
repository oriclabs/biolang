use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, GenomicInterval, Strand, StreamValue, Table, Value};

use crate::env::Environment;

use std::io::Write;
use std::sync::{Arc, Mutex};

thread_local! {
    static OUTPUT_BUFFER: std::cell::RefCell<Option<Arc<Mutex<String>>>> =
        std::cell::RefCell::new(None);
    /// True when the last write_output call did not end with a newline.
    static NEEDS_NEWLINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Set a thread-local output capture buffer. When set, print/println write here instead of stdout.
pub fn set_output_buffer(buf: Option<Arc<Mutex<String>>>) {
    OUTPUT_BUFFER.with(|cell| *cell.borrow_mut() = buf);
}

pub(crate) fn write_output(text: &str) {
    OUTPUT_BUFFER.with(|cell| {
        let borrow = cell.borrow();
        if let Some(ref buf) = *borrow {
            if let Ok(mut s) = buf.lock() {
                s.push_str(text);
            }
        } else {
            print!("{text}");
            let _ = std::io::stdout().flush();
            NEEDS_NEWLINE.with(|flag| flag.set(!text.ends_with('\n')));
        }
    });
}

/// If the last `print()` call left the cursor mid-line, emit a newline.
/// Call this after interpreting a statement in the REPL.
pub fn flush_trailing_newline() {
    NEEDS_NEWLINE.with(|flag| {
        if flag.get() {
            println!();
            flag.set(false);
        }
    });
}

/// Register all built-in functions into the environment.
pub fn register_builtins(env: &mut Environment) {
    let mut builtins: Vec<(&str, Arity)> = vec![
        ("print", Arity::AtLeast(0)),
        ("println", Arity::AtLeast(0)),
        ("len", Arity::Exact(1)),
        ("ncols", Arity::Exact(1)),
        ("columns", Arity::Exact(1)),
        ("type", Arity::Exact(1)),
        ("typeof", Arity::Exact(1)),
        ("range", Arity::Range(1, 3)),
        ("map", Arity::Exact(2)),
        ("each", Arity::Exact(2)),
        ("filter", Arity::Exact(2)),
        ("flat_map", Arity::Exact(2)),
        ("scan", Arity::Exact(3)),
        ("reduce", Arity::Range(2, 3)),
        ("sort", Arity::Range(1, 2)),
        ("abs", Arity::Exact(1)),
        ("min", Arity::AtLeast(1)),
        ("max", Arity::AtLeast(1)),
        ("int", Arity::Exact(1)),
        ("float", Arity::Exact(1)),
        ("str", Arity::Exact(1)),
        ("bool", Arity::Exact(1)),
        ("push", Arity::Exact(2)),
        ("pop", Arity::Exact(1)),
        ("contains", Arity::Exact(2)),
        ("keys", Arity::Exact(1)),
        ("values", Arity::Exact(1)),
        ("reverse", Arity::Exact(1)),
        ("join", Arity::Range(1, 2)),
        ("split", Arity::Exact(2)),
        ("head", Arity::Range(1, 2)),
        ("tail", Arity::Range(1, 2)),
        ("collect", Arity::Exact(1)),
        ("next", Arity::Exact(1)),
        ("count", Arity::Exact(1)),
        ("take", Arity::Exact(2)),
        ("to_stream", Arity::Exact(1)),
        ("table", Arity::Exact(1)),
        ("to_table", Arity::Exact(1)),
        ("interval", Arity::Range(3, 4)),
        ("mutate", Arity::Exact(3)),
        ("summarize", Arity::Exact(2)),
        // List operations
        ("zip", Arity::Exact(2)),
        ("enumerate", Arity::Exact(1)),
        ("flatten", Arity::Exact(1)),
        ("chunk", Arity::Exact(2)),
        ("slice", Arity::Range(2, 3)),
        ("concat", Arity::Exact(2)),
        ("first", Arity::Exact(1)),
        ("last", Arity::Exact(1)),
        ("drop", Arity::Range(1, 2)),
        ("window", Arity::Exact(2)),
        ("frequencies", Arity::Exact(1)),
        ("repeat", Arity::Exact(2)),
        // HOF: count_if (dispatched in interpreter.rs)
        ("count_if", Arity::Exact(2)),
        // Column extraction
        ("col", Arity::Exact(2)),
        // Set similarity
        ("jaccard", Arity::Exact(2)),
        // HOFs: any, all, none, find, find_index, take_while (dispatched in interpreter.rs)
        ("any", Arity::Exact(2)),
        ("all", Arity::Exact(2)),
        ("none", Arity::Exact(2)),
        ("find", Arity::Exact(2)),
        ("find_index", Arity::Exact(2)),
        ("take_while", Arity::Exact(2)),
        // String operations
        ("ascii", Arity::Exact(1)),
        ("chr", Arity::Exact(1)),
        ("substr", Arity::Range(2, 3)),
        ("replace", Arity::Exact(3)),
        ("trim", Arity::Exact(1)),
        ("upper", Arity::Exact(1)),
        ("lower", Arity::Exact(1)),
        ("starts_with", Arity::Exact(2)),
        ("ends_with", Arity::Exact(2)),
        // Map/Record operations
        ("merge", Arity::Exact(2)),
        ("has_key", Arity::Exact(2)),
        ("remove_key", Arity::Exact(2)),
        ("to_string", Arity::Exact(1)),
        ("parse_json", Arity::Exact(1)),
        ("compare", Arity::Exact(2)),
        ("exit", Arity::Range(0, 1)),
        // Error handling
        ("try_call", Arity::Exact(1)),
        ("error", Arity::Exact(1)),
        // F7: help() builtin
        ("help", Arity::Exact(1)),
        // F11: Unit builtins
        ("bp", Arity::Exact(1)),
        ("kb", Arity::Exact(1)),
        ("mb", Arity::Exact(1)),
        ("gb", Arity::Exact(1)),
        // F12: par_map, par_filter (dispatched as HOF in interpreter)
        ("par_map", Arity::Exact(2)),
        ("par_filter", Arity::Exact(2)),
        // F13: Property testing
        ("prop_test", Arity::Exact(3)),
        ("gen_int", Arity::Range(1, 3)),
        ("gen_float", Arity::Exact(0)),
        ("gen_str", Arity::Range(0, 1)),
        // F4: Range type check
        ("is_range", Arity::Exact(1)),
        ("is_enum", Arity::Exact(1)),
        // Batch 2: Set operations
        ("set", Arity::Exact(1)),
        ("union", Arity::Exact(2)),
        ("intersection", Arity::Exact(2)),
        ("difference", Arity::Exact(2)),
        ("symmetric_difference", Arity::Exact(2)),
        ("is_subset", Arity::Exact(2)),
        ("is_superset", Arity::Exact(2)),
        // Batch 2: Type predicates for new types
        ("is_set", Arity::Exact(1)),
        ("is_regex", Arity::Exact(1)),
        ("is_future", Arity::Exact(1)),
        // Batch 2: Async
        ("await_all", Arity::Exact(1)),
        // Type conversion
        ("into", Arity::Exact(2)),
        // Batch 2: Decorators
        ("memoize", Arity::Exact(1)),
        ("time_it", Arity::Exact(1)),
        ("once", Arity::Exact(1)),
        // Batch 2: Bio — genomic range queries
        ("interval_tree", Arity::Exact(1)),
        ("query_overlaps", Arity::Range(4, 4)),
        ("count_overlaps", Arity::Range(4, 4)),
        ("bulk_overlaps", Arity::Exact(2)),
        ("query_nearest", Arity::Range(3, 4)),
        ("coverage", Arity::Range(1, 4)),
        // Batch 2: Bio — sequence pattern matching
        ("motif_find", Arity::Exact(2)),
        ("motif_count", Arity::Exact(2)),
        ("consensus", Arity::Exact(1)),
        ("pwm", Arity::Exact(1)),
        ("pwm_scan", Arity::Range(2, 3)),
        // Batch 2: Bio — pipeline viz
        ("pipeline_steps", Arity::Exact(1)),
        // Language features: where (HOF alias for filter), case_when (HOF), tee
        ("where", Arity::Exact(2)),
        ("case_when", Arity::AtLeast(2)),
        ("tee", Arity::Exact(2)),
        ("tap", Arity::Exact(2)),
        ("inspect", Arity::Exact(2)),
        ("group_apply", Arity::Exact(3)),
        // Bio type constructors
        ("gene", Arity::Exact(1)),
        ("variant", Arity::Range(1, 4)),
        ("genome", Arity::Exact(1)),
        // Type predicates for new types
        ("is_gene", Arity::Exact(1)),
        ("is_variant", Arity::Exact(1)),
        ("is_genome", Arity::Exact(1)),
        ("is_quality", Arity::Exact(1)),
        ("is_aligned_read", Arity::Exact(1)),
        // AlignedRead builtins
        ("aligned_read", Arity::Exact(1)),
        ("flagstat", Arity::Exact(1)),
        // Variant classification builtins
        ("is_snp", Arity::Exact(1)),
        ("is_indel", Arity::Exact(1)),
        ("is_transition", Arity::Exact(1)),
        ("is_transversion", Arity::Exact(1)),
        ("is_het", Arity::Exact(1)),
        ("is_hom_ref", Arity::Exact(1)),
        ("is_hom_alt", Arity::Exact(1)),
        ("is_multiallelic", Arity::Exact(1)),
        ("variant_type", Arity::Exact(1)),
        ("variant_summary", Arity::Exact(1)),
        ("tstv_ratio", Arity::Exact(1)),
        ("het_hom_ratio", Arity::Exact(1)),
        ("parse_vcf_info", Arity::Exact(1)),
        // Collection: partition, sort_by (HOFs dispatched in interpreter)
        ("partition", Arity::Exact(2)),
        ("sort_by", Arity::Exact(2)),
    ];

    // Add table ops builtins
    for (name, arity) in crate::table_ops::table_builtin_list() {
        builtins.push((name, arity));
    }

    // Add stats/math/string builtins
    for (name, arity) in crate::stats::stats_builtin_list() {
        builtins.push((name, arity));
    }

    // Add JSON builtins
    for (name, arity) in crate::json::json_builtin_list() {
        builtins.push((name, arity));
    }

    // Add filesystem builtins (native only)
    #[cfg(feature = "native")]
    for (name, arity) in crate::fs::fs_builtin_list() {
        builtins.push((name, arity));
    }

    // Add HTTP builtins (native only)
    #[cfg(feature = "native")]
    for (name, arity) in crate::http::http_builtin_list() {
        builtins.push((name, arity));
    }

    // Add regex builtins
    for (name, arity) in crate::regex_ops::regex_builtin_list() {
        builtins.push((name, arity));
    }

    // Add CSV/TSV builtins
    for (name, arity) in crate::csv::csv_builtin_list() {
        builtins.push((name, arity));
    }

    // Add inline utility builtins
    builtins.push(("env", Arity::Exact(1)));
    builtins.push(("cwd", Arity::Exact(0)));
    builtins.push(("assert", Arity::Exact(2)));
    builtins.push(("debug", Arity::Exact(1)));

    // Native-only: sleep, compression, doctor, config
    #[cfg(feature = "native")]
    {
        builtins.push(("sleep", Arity::Exact(1)));
        builtins.push(("gzip", Arity::Exact(2)));
        builtins.push(("gunzip", Arity::Exact(2)));
        builtins.push(("bgzip", Arity::Exact(2)));
        builtins.push(("doctor", Arity::Exact(0)));
        builtins.push(("config", Arity::Range(0, 1)));
    }

    // Add plot builtins
    for (name, arity) in crate::plot::plot_builtin_list() {
        builtins.push((name, arity));
    }

    // Add viz builtins
    for (name, arity) in crate::viz::viz_builtin_list() {
        builtins.push((name, arity));
    }

    // Add bio_plots builtins
    for (name, arity) in crate::bio_plots::bio_plots_builtin_list() {
        builtins.push((name, arity));
    }

    // Add matrix builtins
    for (name, arity) in crate::matrix::matrix_builtin_list() {
        builtins.push((name, arity));
    }

    // Add enrichment builtins (native only — reads GMT files)
    #[cfg(feature = "native")]
    for (name, arity) in crate::enrich::enrich_builtin_list() {
        builtins.push((name, arity));
    }

    // Add graph builtins
    for (name, arity) in crate::graph::graph_builtin_list() {
        builtins.push((name, arity));
    }

    // Add container builtins (native only — subprocess + network)
    #[cfg(feature = "native")]
    for (name, arity) in crate::container::container_builtin_list() {
        builtins.push((name, arity));
    }

    // Add LLM chat builtins (native only — HTTP to API providers)
    #[cfg(feature = "native")]
    for (name, arity) in crate::llm::llm_builtin_list() {
        builtins.push((name, arity));
    }

    // Add transfer protocol builtins (native only — FTP, SFTP, S3, etc.)
    #[cfg(feature = "native")]
    for (name, arity) in crate::transfer::transfer_builtin_list() {
        builtins.push((name, arity));
    }

    // Add Nextflow parser builtins (native only — reads .nf files)
    #[cfg(feature = "native")]
    for (name, arity) in crate::nf_parse::nf_parse_builtin_list() {
        builtins.push((name, arity));
    }

    // Add notification builtins (native only — webhook/SMTP)
    #[cfg(feature = "native")]
    for (name, arity) in crate::notify::notify_builtin_list() {
        builtins.push((name, arity));
    }

    // Add SQLite builtins (native only)
    #[cfg(feature = "native")]
    for (name, arity) in crate::sqlite::sqlite_builtin_list() {
        builtins.push((name, arity));
    }

    // Add Parquet builtins (native only)
    #[cfg(feature = "native")]
    for (name, arity) in crate::parquet::parquet_builtin_list() {
        builtins.push((name, arity));
    }

    // Add hash builtins
    for (name, arity) in crate::hash::hash_builtin_list() {
        builtins.push((name, arity));
    }

    // Add datetime builtins
    for (name, arity) in crate::datetime::datetime_builtin_list() {
        builtins.push((name, arity));
    }

    // Add text processing builtins
    for (name, arity) in crate::text_ops::text_builtin_list() {
        builtins.push((name, arity));
    }

    // Add sparse matrix builtins
    for (name, arity) in crate::sparse::sparse_builtin_list() {
        builtins.push((name, arity));
    }

    // Add bio ops builtins (graph, phylo, dim reduction, clustering, diff expr)
    for (name, arity) in crate::bio_ops::bio_ops_builtin_list() {
        builtins.push((name, arity));
    }

    // Add markdown/HTML export builtins
    for (name, arity) in crate::markdown::markdown_builtin_list() {
        builtins.push((name, arity));
    }

    // Add core sequence builtins (WASM-safe — pure string transforms via bio_core)
    for (name, arity) in crate::seq::seq_builtin_list() {
        builtins.push((name, arity));
    }

    // Add NCBI E-utilities builtins (WASM-safe — uses fetch hook for browser API calls)
    // On native builds these are shadowed by the full apis.rs builtins registered above.
    #[cfg(not(feature = "native"))]
    for (name, arity) in crate::ncbi_wasm::ncbi_wasm_builtin_list() {
        builtins.push((name, arity));
    }

    // GAP 1: Coordinate system builtins
    builtins.extend_from_slice(&[
        ("coord_bed", Arity::Exact(1)),
        ("coord_vcf", Arity::Exact(1)),
        ("coord_gff", Arity::Exact(1)),
        ("coord_sam", Arity::Exact(1)),
        ("coord_convert", Arity::Exact(2)),
        ("coord_system", Arity::Exact(1)),
        ("coord_check", Arity::Exact(2)),
        // Chromosome name normalization
        ("strip_chr", Arity::Exact(1)),
        ("add_chr", Arity::Exact(1)),
        ("normalize_chrom", Arity::Exact(1)),
    ]);

    // GAP 2: K-mer builtins
    builtins.extend_from_slice(&[
        ("kmer_encode", Arity::Exact(2)),
        ("kmer_decode", Arity::Exact(1)),
        ("kmer_rc", Arity::Exact(1)),
        ("kmer_canonical", Arity::Exact(1)),
        ("kmer_count", Arity::Range(2, 3)),
        ("kmer_distinct", Arity::Exact(2)),
        ("kmer_spectrum", Arity::Exact(1)),
        ("minimizers", Arity::Exact(3)),
    ]);

    // GAP 3: Streaming enhancements
    builtins.extend_from_slice(&[
        ("stream_chunks", Arity::Exact(2)),
        ("stream_take", Arity::Exact(2)),
        ("stream_skip", Arity::Exact(2)),
        ("stream_batch", Arity::Exact(3)), // HOF dispatched in interpreter
        ("memory_usage", Arity::Exact(0)),
    ]);

    // GAP 4: Parallel primitives
    builtins.extend_from_slice(&[
        ("scatter_by", Arity::Exact(2)), // HOF dispatched in interpreter
        ("bench", Arity::Exact(3)),      // HOF dispatched in interpreter
    ]);

    // GAP 6: Typed table columns
    builtins.extend_from_slice(&[
        ("table_col_types", Arity::Exact(1)),
        ("table_set_col_type", Arity::Exact(3)),
        ("table_validate", Arity::Exact(1)),
        ("table_schema", Arity::Exact(1)),
        ("table_cast", Arity::Exact(3)),
    ]);

    // GAP 7: Pipe fusion (explicit)
    builtins.push(("pipe_fuse", Arity::AtLeast(2)));

    // GAP 8: Data provenance
    builtins.extend_from_slice(&[
        ("with_provenance", Arity::Exact(2)),
        ("provenance", Arity::Exact(1)),
        ("provenance_chain", Arity::Exact(1)),
        ("checkpoint", Arity::Exact(2)),
        ("resume_checkpoint", Arity::Exact(1)),
    ]);

    // Add type predicate builtins
    builtins.extend_from_slice(&[
        ("is_nil", Arity::Exact(1)),
        ("is_int", Arity::Exact(1)),
        ("is_float", Arity::Exact(1)),
        ("is_num", Arity::Exact(1)),
        ("is_str", Arity::Exact(1)),
        ("is_bool", Arity::Exact(1)),
        ("is_list", Arity::Exact(1)),
        ("is_map", Arity::Exact(1)),
        ("is_record", Arity::Exact(1)),
        ("is_table", Arity::Exact(1)),
        ("is_function", Arity::Exact(1)),
        ("is_dna", Arity::Exact(1)),
        ("is_rna", Arity::Exact(1)),
        ("is_protein", Arity::Exact(1)),
        ("is_interval", Arity::Exact(1)),
        ("is_matrix", Arity::Exact(1)),
        ("is_stream", Arity::Exact(1)),
        ("is_kmer", Arity::Exact(1)),
        ("is_sparse", Arity::Exact(1)),
    ]);

    // Add bio builtins (native: full bl-bio with noodles; WASM: lightweight text parsers)
    #[cfg(feature = "native")]
    for (name, arity) in bl_bio::bio_builtin_list() {
        builtins.push((name, arity));
    }
    #[cfg(not(feature = "native"))]
    for (name, arity) in crate::bio_wasm::bio_wasm_builtin_list() {
        builtins.push((name, arity));
    }

    // Add bio API builtins (native only — typed clients for NCBI, Ensembl, UniProt, etc.)
    #[cfg(feature = "native")]
    for (name, arity) in crate::apis::apis_builtin_list() {
        builtins.push((name, arity));
    }

    for (name, arity) in builtins {
        env.define(
            name.to_string(),
            Value::NativeFunction {
                name: name.to_string(),
                arity,
            },
        );
    }
}

/// Collect all known builtin function names (for "did you mean?" suggestions).
pub fn all_builtin_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = vec![
        "print", "println", "len", "ncols", "columns", "type", "typeof",
        "range", "map", "each", "filter", "flat_map", "scan", "reduce",
        "sort", "abs", "min", "max", "int", "float", "str", "bool",
        "push", "pop", "contains", "keys", "values", "reverse", "join", "split",
        "head", "tail", "collect", "next", "count", "take", "to_stream",
        "table", "to_table", "interval", "mutate", "summarize",
        "zip", "enumerate", "flatten", "chunk", "slice", "concat",
        "first", "last", "drop", "window", "frequencies", "repeat",
        "count_if", "col", "jaccard",
        "any", "all", "none", "find", "find_index", "take_while",
        "ascii", "chr", "substr", "replace", "trim", "upper", "lower",
        "starts_with", "ends_with",
        "merge", "has_key", "remove_key", "to_string", "parse_json",
        "compare", "exit", "try_call", "error", "help",
        "bp", "kb", "mb", "gb",
        "par_map", "par_filter",
        "prop_test", "gen_int", "gen_float", "gen_str",
        "is_range", "is_enum",
        "set", "union", "intersection", "difference", "symmetric_difference",
        "is_subset", "is_superset",
        "is_set", "is_regex", "is_future", "await_all", "into",
        "memoize", "time_it", "once",
        "interval_tree", "query_overlaps", "count_overlaps", "bulk_overlaps",
        "query_nearest", "coverage",
        "motif_find", "motif_count", "consensus", "pwm", "pwm_scan",
        "pipeline_steps",
        "where", "case_when", "tee", "tap", "inspect", "group_apply",
        "gene", "variant", "genome",
        "is_gene", "is_variant", "is_genome", "is_quality", "is_aligned_read",
        "aligned_read", "flagstat",
        "is_snp", "is_indel", "is_transition", "is_transversion",
        "is_het", "is_hom_ref", "is_hom_alt", "is_multiallelic",
        "variant_type", "variant_summary", "tstv_ratio", "het_hom_ratio",
        "parse_vcf_info", "partition", "sort_by",
        "coord_bed", "coord_vcf", "coord_gff", "coord_sam",
        "coord_convert", "coord_system", "coord_check",
        "strip_chr", "add_chr", "normalize_chrom",
        "kmer_encode", "kmer_decode", "kmer_rc", "kmer_canonical",
        "kmer_count", "kmer_distinct", "kmer_spectrum", "minimizers",
        "stream_chunks", "stream_take", "stream_skip", "stream_batch", "memory_usage",
        "scatter_by", "bench",
        "table_col_types", "table_set_col_type", "table_validate", "table_schema", "table_cast",
        "pipe_fuse",
        "with_provenance", "provenance", "provenance_chain", "checkpoint", "resume_checkpoint",
        "is_nil", "is_int", "is_float", "is_num", "is_str", "is_bool",
        "is_list", "is_map", "is_record", "is_table", "is_function",
        "is_dna", "is_rna", "is_protein", "is_interval", "is_matrix",
        "is_stream", "is_kmer", "is_sparse",
        "env", "cwd", "assert", "debug",
    ];
    // Add names from sub-module builtin lists
    for (n, _) in crate::table_ops::table_builtin_list() { names.push(n); }
    for (n, _) in crate::stats::stats_builtin_list() { names.push(n); }
    for (n, _) in crate::json::json_builtin_list() { names.push(n); }
    for (n, _) in crate::regex_ops::regex_builtin_list() { names.push(n); }
    for (n, _) in crate::csv::csv_builtin_list() { names.push(n); }
    for (n, _) in crate::plot::plot_builtin_list() { names.push(n); }
    for (n, _) in crate::viz::viz_builtin_list() { names.push(n); }
    for (n, _) in crate::bio_plots::bio_plots_builtin_list() { names.push(n); }
    for (n, _) in crate::matrix::matrix_builtin_list() { names.push(n); }
    #[cfg(feature = "native")]
    for (n, _) in crate::enrich::enrich_builtin_list() { names.push(n); }
    for (n, _) in crate::graph::graph_builtin_list() { names.push(n); }
    for (n, _) in crate::bio_ops::bio_ops_builtin_list() { names.push(n); }
    for (n, _) in crate::markdown::markdown_builtin_list() { names.push(n); }
    for (n, _) in crate::hash::hash_builtin_list() { names.push(n); }
    for (n, _) in crate::datetime::datetime_builtin_list() { names.push(n); }
    for (n, _) in crate::text_ops::text_builtin_list() { names.push(n); }
    for (n, _) in crate::sparse::sparse_builtin_list() { names.push(n); }
    for (n, _) in crate::seq::seq_builtin_list() { names.push(n); }
    #[cfg(feature = "native")]
    for (n, _) in bl_bio::bio_builtin_list() { names.push(n); }
    #[cfg(not(feature = "native"))]
    for (n, _) in crate::bio_wasm::bio_wasm_builtin_list() { names.push(n); }
    #[cfg(not(feature = "native"))]
    for (n, _) in crate::ncbi_wasm::ncbi_wasm_builtin_list() { names.push(n); }
    #[cfg(feature = "native")]
    {
        for (n, _) in crate::fs::fs_builtin_list() { names.push(n); }
        for (n, _) in crate::http::http_builtin_list() { names.push(n); }
        for (n, _) in crate::container::container_builtin_list() { names.push(n); }
        for (n, _) in crate::llm::llm_builtin_list() { names.push(n); }
        for (n, _) in crate::transfer::transfer_builtin_list() { names.push(n); }
        for (n, _) in crate::nf_parse::nf_parse_builtin_list() { names.push(n); }
        for (n, _) in crate::notify::notify_builtin_list() { names.push(n); }
        for (n, _) in crate::sqlite::sqlite_builtin_list() { names.push(n); }
        for (n, _) in crate::parquet::parquet_builtin_list() { names.push(n); }
        for (n, _) in crate::apis::apis_builtin_list() { names.push(n); }
    }
    names
}

/// Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Find the most similar builtin name using Levenshtein distance.
pub fn suggest_builtin(name: &str) -> Option<String> {
    let max_dist = (name.len() / 3).max(2);
    let mut best: Option<(String, usize)> = None;
    for builtin_name in all_builtin_names() {
        let dist = levenshtein(name, builtin_name);
        if dist > 0 && dist <= max_dist {
            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((builtin_name.to_string(), dist));
            }
        }
    }
    best.map(|(s, _)| s)
}

/// Format a value for print/println — human-readable, unwrapped bio types, truncated long data.
fn format_for_print(val: &Value) -> String {
    const MAX_SEQ: usize = 80;
    match val {
        Value::Str(s) => s.clone(),
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => {
            if seq.data.len() > MAX_SEQ {
                format!("{}... ({} total)", &seq.data[..MAX_SEQ], seq.data.len())
            } else {
                seq.data.clone()
            }
        }
        Value::Quality(scores) => {
            let ascii: String = scores.iter().map(|&b| (b + 33) as char).collect();
            if ascii.len() > MAX_SEQ {
                format!("{}... ({} total)", &ascii[..MAX_SEQ], ascii.len())
            } else {
                ascii
            }
        }
        Value::Interval(iv) => format!("{iv}"),
        Value::Gene { symbol, chrom, start, end, strand, biotype, .. } => {
            format!("{symbol} {chrom}:{start}-{end}:{strand} [{biotype}]")
        }
        Value::Variant { chrom, pos, ref_allele, alt_allele, quality, .. } => {
            format!("{chrom}:{pos} {ref_allele}>{alt_allele} Q={quality:.0}")
        }
        Value::Genome { name, assembly, .. } => format!("{name} {assembly}"),
        Value::AlignedRead(r) => {
            format!("{} {}:{} {}", r.qname, r.rname, r.pos, r.cigar)
        }
        // Everything else: use Display
        other => format!("{other}"),
    }
}

/// Execute a built-in function by name.
pub fn call_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "print" => {
            let parts: Vec<String> = args.iter().map(|a| format_for_print(a)).collect();
            write_output(&parts.join(" "));
            Ok(Value::Nil)
        }
        "println" => {
            let parts: Vec<String> = args.iter().map(|a| format_for_print(a)).collect();
            write_output(&format!("{}\n", parts.join(" ")));
            Ok(Value::Nil)
        }
        "len" => match &args[0] {
            Value::Str(s) => Ok(Value::Int(s.len() as i64)),
            Value::List(l) => Ok(Value::Int(l.len() as i64)),
            Value::Map(m) | Value::Record(m) => Ok(Value::Int(m.len() as i64)),
            Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => {
                Ok(Value::Int(seq.data.len() as i64))
            }
            Value::Table(t) => Ok(Value::Int(t.num_rows() as i64)),
            Value::Matrix(m) => Ok(Value::Int((m.nrow * m.ncol) as i64)),
            Value::Set(s) => Ok(Value::Int(s.len() as i64)),
            Value::Kmer(km) => Ok(Value::Int(km.k as i64)),
            Value::SparseMatrix(sm) => Ok(Value::Int((sm.nrow * sm.ncol) as i64)),
            Value::Range { start, end, inclusive } => {
                let len = if *inclusive { end - start + 1 } else { end - start };
                Ok(Value::Int(len.max(0)))
            }
            other => Err(BioLangError::type_error(
                format!("len() not supported for {}", other.type_of()),
                None,
            )),
        },
        "ncols" => match &args[0] {
            Value::Table(t) => Ok(Value::Int(t.num_cols() as i64)),
            Value::Matrix(m) => Ok(Value::Int(m.ncol as i64)),
            Value::SparseMatrix(sm) => Ok(Value::Int(sm.ncol as i64)),
            other => Err(BioLangError::type_error(
                format!("ncols() requires Table or Matrix, got {}", other.type_of()),
                None,
            )),
        },
        "columns" => match &args[0] {
            Value::Table(t) => Ok(Value::List(
                t.columns.iter().map(|c| Value::Str(c.clone())).collect(),
            )),
            Value::Record(m) | Value::Map(m) => Ok(Value::List(
                m.keys().map(|k| Value::Str(k.clone())).collect(),
            )),
            other => Err(BioLangError::type_error(
                format!("columns() requires Table or Record, got {}", other.type_of()),
                None,
            )),
        },
        "type" | "typeof" => Ok(Value::Str(args[0].type_of().to_string())),
        "range" => {
            let (start, end, step) = match args.len() {
                1 => (0, require_int(&args[0], "range")?, 1),
                2 => (
                    require_int(&args[0], "range")?,
                    require_int(&args[1], "range")?,
                    1,
                ),
                3 => (
                    require_int(&args[0], "range")?,
                    require_int(&args[1], "range")?,
                    require_int(&args[2], "range")?,
                ),
                _ => unreachable!(),
            };
            if step == 0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "range step cannot be zero",
                    None,
                ));
            }
            let count = if step > 0 {
                ((end - start).max(0) as u64 + step as u64 - 1) / step as u64
            } else {
                ((start - end).max(0) as u64 + (-step) as u64 - 1) / (-step) as u64
            };
            if count > 10_000_000 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "range() would produce {count} elements (limit: 10,000,000). \
                         Use a smaller range or process data with streams."
                    ),
                    None,
                ));
            }
            let mut result = Vec::with_capacity(count as usize);
            let mut i = start;
            if step > 0 {
                while i < end {
                    result.push(Value::Int(i));
                    i += step;
                }
            } else {
                while i > end {
                    result.push(Value::Int(i));
                    i += step;
                }
            }
            Ok(Value::List(result))
        }
        "abs" => match &args[0] {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            other => Err(BioLangError::type_error(
                format!("abs() requires number, got {}", other.type_of()),
                None,
            )),
        },
        "min" => {
            if args.len() == 1 {
                if let Value::List(items) = &args[0] {
                    return list_min(items);
                }
            }
            list_min(&args)
        }
        "max" => {
            if args.len() == 1 {
                if let Value::List(items) = &args[0] {
                    return list_max(items);
                }
            }
            list_max(&args)
        }
        "int" => match &args[0] {
            Value::Int(n) => Ok(Value::Int(*n)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::Str(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
                BioLangError::type_error(format!("cannot convert '{s}' to Int"), None)
            }),
            Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            other => Err(BioLangError::type_error(
                format!("cannot convert {} to Int", other.type_of()),
                None,
            )),
        },
        "float" => match &args[0] {
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Int(n) => Ok(Value::Float(*n as f64)),
            Value::Str(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
                BioLangError::type_error(format!("cannot convert '{s}' to Float"), None)
            }),
            other => Err(BioLangError::type_error(
                format!("cannot convert {} to Float", other.type_of()),
                None,
            )),
        },
        "str" | "to_string" => Ok(Value::Str(format!("{}", args[0]))),
        "bool" => Ok(Value::Bool(args[0].is_truthy())),
        "parse_json" => {
            let s = match &args[0] {
                Value::Str(s) => s.as_str(),
                other => return Err(BioLangError::type_error(
                    format!("parse_json() requires Str, got {}", other.type_of()),
                    None,
                )),
            };
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(json_val) => Ok(json_to_value(json_val)),
                Err(e) => Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("parse_json: {e}"),
                    None,
                )),
            }
        }
        "compare" => {
            let ord = match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => a.cmp(b),
                (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                (Value::Str(a), Value::Str(b)) => a.cmp(b),
                (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal),
                _ => return Err(BioLangError::type_error(
                    format!("compare() requires comparable types, got {} and {}", args[0].type_of(), args[1].type_of()),
                    None,
                )),
            };
            Ok(Value::Int(match ord {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }))
        }
        "exit" => {
            let code = if args.is_empty() { 0 } else { require_int(&args[0], "exit")? as i32 };
            std::process::exit(code);
        }
        "into" => {
            let target = match &args[1] {
                Value::Str(s) => s.as_str(),
                other => return Err(BioLangError::type_error(
                    format!("into() requires type name as Str, got {}", other.type_of()),
                    None,
                )),
            };
            match target {
                "List" | "list" => match &args[0] {
                    Value::List(_) => Ok(args[0].clone()),
                    Value::Set(items) => Ok(Value::List(items.clone())),
                    Value::Tuple(items) => Ok(Value::List(items.clone())),
                    Value::Range { start, end, .. } => {
                        Ok(Value::List(((*start as i64)..(*end as i64)).map(Value::Int).collect()))
                    }
                    other => Ok(Value::List(vec![other.clone()])),
                },
                "Set" | "set" => match &args[0] {
                    Value::Set(_) => Ok(args[0].clone()),
                    Value::List(items) => {
                        let mut result = Vec::new();
                        for item in items {
                            if !result.contains(item) { result.push(item.clone()); }
                        }
                        Ok(Value::Set(result))
                    }
                    other => Ok(Value::Set(vec![other.clone()])),
                },
                "Table" | "table" => match &args[0] {
                    Value::Table(_) => Ok(args[0].clone()),
                    Value::List(items) => {
                        // List of Records -> Table
                        if let Some(Value::Record(first)) = items.first() {
                            let columns: Vec<String> = first.keys().cloned().collect();
                            let mut rows = Vec::new();
                            for item in items {
                                if let Value::Record(map) = item {
                                    let row: Vec<Value> = columns.iter().map(|c| map.get(c).cloned().unwrap_or(Value::Nil)).collect();
                                    rows.push(row);
                                }
                            }
                            Ok(Value::Table(bl_core::value::Table::new(columns, rows)))
                        } else {
                            Ok(Value::Table(bl_core::value::Table::new(vec!["value".into()], items.iter().map(|v| vec![v.clone()]).collect())))
                        }
                    }
                    other => Err(BioLangError::type_error(
                        format!("cannot convert {} into Table", other.type_of()),
                        None,
                    )),
                },
                "Str" | "str" | "String" => Ok(Value::Str(format!("{}", args[0]))),
                "Int" | "int" => match &args[0] {
                    Value::Int(_) => Ok(args[0].clone()),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Str(s) => s.parse::<i64>().map(Value::Int).map_err(|_|
                        BioLangError::type_error(format!("cannot convert '{s}' to Int"), None)),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    other => Err(BioLangError::type_error(format!("cannot convert {} to Int", other.type_of()), None)),
                },
                "Float" | "float" => match &args[0] {
                    Value::Float(_) => Ok(args[0].clone()),
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Str(s) => s.parse::<f64>().map(Value::Float).map_err(|_|
                        BioLangError::type_error(format!("cannot convert '{s}' to Float"), None)),
                    other => Err(BioLangError::type_error(format!("cannot convert {} to Float", other.type_of()), None)),
                },
                "Record" | "record" => match &args[0] {
                    Value::Record(_) | Value::Map(_) => Ok(args[0].clone()),
                    other => Err(BioLangError::type_error(
                        format!("cannot convert {} into Record", other.type_of()),
                        None,
                    )),
                },
                other => Err(BioLangError::type_error(
                    format!("into(): unknown target type '{other}'"),
                    None,
                )),
            }
        }
        "push" => {
            let mut list = match &args[0] {
                Value::List(l) => l.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("push() requires List, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            list.push(args[1].clone());
            Ok(Value::List(list))
        }
        "pop" => {
            let mut list = match &args[0] {
                Value::List(l) => l.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("pop() requires List, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            list.pop();
            Ok(Value::List(list))
        }
        "contains" => match (&args[0], &args[1]) {
            (Value::List(list), val) => Ok(Value::Bool(list.contains(val))),
            (Value::Set(items), val) => Ok(Value::Bool(items.contains(val))),
            (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
            (Value::Map(m), Value::Str(key)) | (Value::Record(m), Value::Str(key)) => {
                Ok(Value::Bool(m.contains_key(key)))
            }
            (a, _) => Err(BioLangError::type_error(
                format!("contains() not supported for {}", a.type_of()),
                None,
            )),
        },
        "keys" => match &args[0] {
            Value::Map(m) | Value::Record(m) => {
                Ok(Value::List(m.keys().map(|k| Value::Str(k.clone())).collect()))
            }
            other => Err(BioLangError::type_error(
                format!("keys() requires Map or Record, got {}", other.type_of()),
                None,
            )),
        },
        "values" => match &args[0] {
            Value::Map(m) | Value::Record(m) => Ok(Value::List(m.values().cloned().collect())),
            other => Err(BioLangError::type_error(
                format!("values() requires Map or Record, got {}", other.type_of()),
                None,
            )),
        },
        "reverse" => match &args[0] {
            Value::List(l) => {
                let mut r = l.clone();
                r.reverse();
                Ok(Value::List(r))
            }
            Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
            Value::Table(t) => {
                let mut rows = t.rows.clone();
                rows.reverse();
                Ok(Value::Table(Table::new(t.columns.clone(), rows)))
            }
            other => Err(BioLangError::type_error(
                format!("reverse() not supported for {}", other.type_of()),
                None,
            )),
        },
        "join" => {
            let sep = if args.len() > 1 {
                match &args[1] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("join() separator must be Str, got {}", other.type_of()),
                            None,
                        ))
                    }
                }
            } else {
                String::new()
            };
            match &args[0] {
                Value::List(items) => {
                    let strs: Vec<String> = items.iter().map(|v| format!("{v}")).collect();
                    Ok(Value::Str(strs.join(&sep)))
                }
                Value::Table(t) => {
                    let strs: Vec<String> = table_to_records(t).iter().map(|v| format!("{v}")).collect();
                    Ok(Value::Str(strs.join(&sep)))
                }
                other => Err(BioLangError::type_error(
                    format!("join() requires List or Table, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "split" => match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(sep)) => Ok(Value::List(
                s.split(sep.as_str())
                    .map(|p| Value::Str(p.to_string()))
                    .collect(),
            )),
            _ => Err(BioLangError::type_error(
                "split() requires (Str, Str)",
                None,
            )),
        },
        "ascii" => match &args[0] {
            Value::Str(s) => {
                let c = s.chars().next().ok_or_else(|| {
                    BioLangError::runtime(ErrorKind::TypeError, "ascii() requires non-empty string", None)
                })?;
                Ok(Value::Int(c as i64))
            }
            _ => Err(BioLangError::type_error("ascii() requires Str", None)),
        },
        "chr" => {
            let code = require_int(&args[0], "chr")? as u32;
            let c = char::from_u32(code).ok_or_else(|| {
                BioLangError::runtime(ErrorKind::TypeError, format!("chr({code}): invalid code point"), None)
            })?;
            Ok(Value::Str(c.to_string()))
        }
        "substr" => match &args[0] {
            Value::Str(s) => {
                let start = require_int(&args[1], "substr")? as usize;
                let chars: Vec<char> = s.chars().collect();
                let end = if args.len() > 2 {
                    (require_int(&args[2], "substr")? as usize).min(chars.len())
                } else {
                    chars.len()
                };
                let slice: String = chars.get(start..end).unwrap_or(&[]).iter().collect();
                Ok(Value::Str(slice))
            }
            _ => Err(BioLangError::type_error("substr() requires Str", None)),
        },
        "replace" => match (&args[0], &args[1], &args[2]) {
            (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
            }
            _ => Err(BioLangError::type_error("replace() requires (Str, Str, Str)", None)),
        },
        "trim" => match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
            _ => Err(BioLangError::type_error("trim() requires Str", None)),
        },
        "upper" => match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
            _ => Err(BioLangError::type_error("upper() requires Str", None)),
        },
        "lower" => match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
            _ => Err(BioLangError::type_error("lower() requires Str", None)),
        },
        "starts_with" => match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(prefix)) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
            _ => Err(BioLangError::type_error("starts_with() requires (Str, Str)", None)),
        },
        "ends_with" => match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(suffix)) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
            _ => Err(BioLangError::type_error("ends_with() requires (Str, Str)", None)),
        },
        "head" => {
            let n = if args.len() > 1 {
                require_int(&args[1], "head")? as usize
            } else {
                5
            };
            match &args[0] {
                Value::List(l) => Ok(Value::List(l.iter().take(n).cloned().collect())),
                Value::Table(t) => {
                    let rows = t.rows.iter().take(n).cloned().collect();
                    Ok(Value::Table(Table::new(t.columns.clone(), rows)))
                }
                Value::Stream(s) => {
                    let mut items = Vec::with_capacity(n);
                    for _ in 0..n {
                        match s.next() {
                            Some(v) => items.push(v),
                            None => break,
                        }
                    }
                    Ok(Value::List(items))
                }
                other => Err(BioLangError::type_error(
                    format!("head() requires List, Table, or Stream, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "tail" => {
            let n = if args.len() > 1 {
                require_int(&args[1], "tail")? as usize
            } else {
                5
            };
            match &args[0] {
                Value::List(l) => {
                    let skip = l.len().saturating_sub(n);
                    Ok(Value::List(l.iter().skip(skip).cloned().collect()))
                }
                Value::Table(t) => {
                    let skip = t.rows.len().saturating_sub(n);
                    let rows = t.rows.iter().skip(skip).cloned().collect();
                    Ok(Value::Table(Table::new(t.columns.clone(), rows)))
                }
                other => Err(BioLangError::type_error(
                    format!("tail() requires List or Table, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "collect" => match args.into_iter().next().unwrap() {
            Value::Stream(s) => {
                if s.is_exhausted() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "stream already consumed: Stream({}) has been fully iterated. \
                             Streams can only be consumed once — store the result with collect() \
                             on first use if you need it again.",
                            s.label
                        ),
                        None,
                    ));
                }
                let items = s.collect_all();
                if items.len() > 1_000_000 {
                    eprintln!(
                        "\x1b[33mWarning:\x1b[0m collect() materialized {} items into memory.",
                        items.len()
                    );
                    eprintln!("  Tip: Use head(n) or take(n) before collect() to limit memory usage.");
                }
                Ok(Value::List(items))
            }
            Value::List(l) => Ok(Value::List(l)),
            Value::Table(t) => Ok(Value::Table(t)), // pass through
            Value::Range { start, end, inclusive } => {
                let end_val = if inclusive { end + 1 } else { end };
                let count = (end_val - start).max(0) as u64;
                if count > 10_000_000 {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "collect() on range would produce {count} elements (limit: 10,000,000). \
                             Use a smaller range or process with head()/take()."
                        ),
                        None,
                    ));
                }
                Ok(Value::List((start..end_val).map(Value::Int).collect()))
            }
            other => Err(BioLangError::type_error(
                format!("collect() requires Stream, List, Table, or Range, got {}", other.type_of()),
                None,
            )),
        },
        "next" => match &args[0] {
            Value::Stream(s) => Ok(s.next().unwrap_or(Value::Nil)),
            other => Err(BioLangError::type_error(
                format!("next() requires Stream, got {}", other.type_of()),
                None,
            )),
        },
        "count" => match &args[0] {
            Value::Stream(s) => {
                let mut n = 0i64;
                while s.next().is_some() {
                    n += 1;
                }
                Ok(Value::Int(n))
            }
            Value::List(l) => Ok(Value::Int(l.len() as i64)),
            Value::Table(t) => Ok(Value::Int(t.num_rows() as i64)),
            other => Err(BioLangError::type_error(
                format!("count() requires Stream, List, or Table, got {}", other.type_of()),
                None,
            )),
        },
        "take" => {
            let n = require_int(&args[1], "take")? as usize;
            match args.into_iter().next().unwrap() {
                Value::Stream(s) => {
                    let mut items = Vec::with_capacity(n);
                    for _ in 0..n {
                        match s.next() {
                            Some(v) => items.push(v),
                            None => break,
                        }
                    }
                    Ok(Value::List(items))
                }
                Value::List(l) => Ok(Value::List(l.into_iter().take(n).collect())),
                Value::Table(t) => {
                    let rows = t.rows.into_iter().take(n).collect();
                    Ok(Value::Table(Table::new(t.columns, rows)))
                }
                other => Err(BioLangError::type_error(
                    format!("take() requires Stream, List, or Table, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "to_stream" => match args.into_iter().next().unwrap() {
            Value::List(l) => Ok(Value::Stream(StreamValue::from_list("list", l))),
            Value::Table(t) => {
                let records = table_to_records(&t);
                Ok(Value::Stream(StreamValue::from_list("table", records)))
            }
            Value::Stream(s) => Ok(Value::Stream(s)),
            other => Err(BioLangError::type_error(
                format!("to_stream() requires List or Table, got {}", other.type_of()),
                None,
            )),
        },
        "table" | "to_table" => {
            match &args[0] {
                // table([{a: 1, b: 2}, ...]) — List of Records (row-oriented)
                Value::List(items) => {
                    let mut recs = Vec::new();
                    for item in items {
                        match item {
                            Value::Record(map) => recs.push(map.clone()),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!(
                                        "table() requires List of Records, found {}",
                                        other.type_of()
                                    ),
                                    None,
                                ))
                            }
                        }
                    }
                    Ok(Value::Table(Table::from_records(&recs)))
                }
                // table({a: [1,2], b: [3,4]}) — Record of Lists (column-oriented, Polars-style)
                Value::Record(map) | Value::Map(map) => {
                    let columns: Vec<String> = map.keys().cloned().collect();
                    if columns.is_empty() {
                        return Ok(Value::Table(Table::new(vec![], vec![])));
                    }
                    let col_data: Vec<&Vec<Value>> = columns
                        .iter()
                        .map(|c| match map.get(c).unwrap() {
                            Value::List(items) => Ok(items),
                            _ => Err(BioLangError::type_error(
                                "table() column values must be Lists",
                                None,
                            )),
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let n = col_data[0].len();
                    let mut rows = Vec::with_capacity(n);
                    for i in 0..n {
                        let row: Vec<Value> = col_data
                            .iter()
                            .map(|col| col.get(i).cloned().unwrap_or(Value::Nil))
                            .collect();
                        rows.push(row);
                    }
                    Ok(Value::Table(Table::new(columns, rows)))
                }
                Value::Stream(s) => {
                    let mut recs = Vec::new();
                    while let Some(item) = s.next() {
                        match item {
                            Value::Record(map) => recs.push(map),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!(
                                        "table() requires Stream of Records, found {}",
                                        other.type_of()
                                    ),
                                    None,
                                ))
                            }
                        }
                        if recs.len() > 1_000_000 {
                            eprintln!(
                                "\x1b[33mWarning:\x1b[0m table() materialized {} records from stream.",
                                recs.len()
                            );
                            eprintln!("  Tip: Use head(n) before table() to limit memory usage.");
                        }
                    }
                    Ok(Value::Table(Table::from_records(&recs)))
                }
                other => Err(BioLangError::type_error(
                    format!("table() requires List, Record, or Stream, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "interval" => {
            let chrom = match &args[0] {
                Value::Str(s) => s.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("interval() chrom must be Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            if chrom.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "interval() chrom must not be empty",
                    None,
                ));
            }
            let start = require_int(&args[1], "interval")?;
            let end = require_int(&args[2], "interval")?;
            if start < 0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("interval() start must be non-negative, got {start}"),
                    None,
                ));
            }
            if end < start {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("interval() end ({end}) must be >= start ({start})"),
                    None,
                ));
            }
            let strand = if args.len() > 3 {
                match &args[3] {
                    Value::Str(s) => match s.as_str() {
                        "+" => Strand::Plus,
                        "-" => Strand::Minus,
                        _ => Strand::Unknown,
                    },
                    _ => Strand::Unknown,
                }
            } else {
                Strand::Unknown
            };
            Ok(Value::Interval(GenomicInterval {
                chrom,
                start,
                end,
                strand,
            }))
        }
        // ── Inline utility builtins ──────────────────────────────
        "cwd" => {
            match std::env::current_dir() {
                Ok(p) => Ok(Value::Str(p.to_string_lossy().to_string())),
                Err(e) => Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("cwd(): {e}"),
                    None,
                )),
            }
        }
        "env" => {
            let name = match &args[0] {
                Value::Str(s) => s,
                other => {
                    return Err(BioLangError::type_error(
                        format!("env() requires Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            match std::env::var(name) {
                Ok(val) => Ok(Value::Str(val)),
                Err(_) => Ok(Value::Nil),
            }
        }
        "assert" => {
            if !args[0].is_truthy() {
                let msg = match &args[1] {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                };
                Err(BioLangError::runtime(
                    ErrorKind::AssertionFailed,
                    msg,
                    None,
                ))
            } else {
                Ok(Value::Nil)
            }
        }
        "debug" => {
            let val = &args[0];
            eprintln!("[{}] {}", val.type_of(), val);
            Ok(Value::Nil)
        }
        #[cfg(feature = "native")]
        "sleep" => {
            let ms = require_int(&args[0], "sleep")?;
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
            Ok(Value::Nil)
        }
        #[cfg(feature = "native")]
        "gzip" | "bgzip" => {
            let input = match &args[0] {
                Value::Str(s) => s.as_str(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("{name}() input requires Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let output = match &args[1] {
                Value::Str(s) => s.as_str(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("{name}() output requires Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let data = std::fs::read(input).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("{name}() read failed: {e}"), None)
            })?;
            let file = std::fs::File::create(output).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("{name}() create failed: {e}"), None)
            })?;
            let mut encoder =
                flate2::write::GzEncoder::new(file, flate2::Compression::default());
            encoder.write_all(&data).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("{name}() compress failed: {e}"), None)
            })?;
            encoder.finish().map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("{name}() finish failed: {e}"), None)
            })?;
            Ok(Value::Str(output.to_string()))
        }
        #[cfg(feature = "native")]
        "gunzip" => {
            let input = match &args[0] {
                Value::Str(s) => s.as_str(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("gunzip() input requires Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let output = match &args[1] {
                Value::Str(s) => s.as_str(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("gunzip() output requires Str, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let file = std::fs::File::open(input).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("gunzip() open failed: {e}"), None)
            })?;
            let decoder = flate2::read::GzDecoder::new(file);
            // Cap decompressed size at 2 GB to prevent decompression bombs
            const MAX_DECOMPRESS: u64 = 2_000_000_000;
            let mut limited = std::io::Read::take(decoder, MAX_DECOMPRESS + 1);
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut limited, &mut data).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("gunzip() decompress failed: {e}"), None)
            })?;
            if data.len() as u64 > MAX_DECOMPRESS {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("gunzip() decompressed data exceeds 2 GB limit — possible decompression bomb"),
                    None,
                ));
            }
            std::fs::write(output, &data).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("gunzip() write failed: {e}"), None)
            })?;
            Ok(Value::Str(output.to_string()))
        }
        #[cfg(feature = "native")]
        "doctor" => builtin_doctor(),
        #[cfg(feature = "native")]
        "config" => builtin_config(args),
        // ── List operations ───────────────────────────────────────
        "zip" => {
            let list_a = match &args[0] {
                Value::List(l) => l.clone(),
                Value::Table(t) => table_to_records(t),
                other => {
                    return Err(BioLangError::type_error(
                        format!("zip() requires List or Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let list_b = match &args[1] {
                Value::List(l) => l.clone(),
                Value::Table(t) => table_to_records(t),
                other => {
                    return Err(BioLangError::type_error(
                        format!("zip() requires List or Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let pairs: Vec<Value> = list_a
                .into_iter()
                .zip(list_b)
                .map(|(a, b)| Value::List(vec![a, b]))
                .collect();
            Ok(Value::List(pairs))
        }
        "enumerate" => {
            let owned;
            let items = match &args[0] {
                Value::List(l) => l,
                Value::Table(t) => { owned = table_to_records(t); &owned }
                other => {
                    return Err(BioLangError::type_error(
                        format!("enumerate() requires List or Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let result: Vec<Value> = items
                .iter()
                .enumerate()
                .map(|(i, v)| Value::List(vec![Value::Int(i as i64), v.clone()]))
                .collect();
            Ok(Value::List(result))
        }
        "flatten" => {
            let items = match &args[0] {
                Value::List(l) => l.clone(),
                Value::Table(t) => table_to_records(t),
                other => {
                    return Err(BioLangError::type_error(
                        format!("flatten() requires List or Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let mut result = Vec::new();
            for item in &items {
                match item {
                    Value::List(inner) => result.extend(inner.iter().cloned()),
                    other => result.push(other.clone()),
                }
            }
            Ok(Value::List(result))
        }
        "chunk" => {
            let owned;
            let items = match &args[0] {
                Value::List(l) => l,
                Value::Table(t) => { owned = table_to_records(t); &owned }
                other => {
                    return Err(BioLangError::type_error(
                        format!("chunk() requires List or Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let size = require_int(&args[1], "chunk")? as usize;
            if size == 0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "chunk() size must be > 0",
                    None,
                ));
            }
            let chunks: Vec<Value> = items
                .chunks(size)
                .map(|c| Value::List(c.to_vec()))
                .collect();
            Ok(Value::List(chunks))
        }
        "first" => {
            match &args[0] {
                Value::List(l) => Ok(l.first().cloned().unwrap_or(Value::Nil)),
                Value::Table(t) => {
                    if t.rows.is_empty() {
                        Ok(Value::Nil)
                    } else {
                        Ok(Value::Record(t.row_to_record(0)))
                    }
                }
                Value::Stream(s) => Ok(s.next().unwrap_or(Value::Nil)),
                other => Err(BioLangError::type_error(
                    format!("first() requires List, Table, or Stream, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "last" => {
            match &args[0] {
                Value::List(l) => Ok(l.last().cloned().unwrap_or(Value::Nil)),
                Value::Table(t) => {
                    if t.rows.is_empty() {
                        Ok(Value::Nil)
                    } else {
                        Ok(Value::Record(t.row_to_record(t.rows.len() - 1)))
                    }
                }
                Value::Stream(s) => {
                    let mut last_val = Value::Nil;
                    while let Some(v) = s.next() {
                        last_val = v;
                    }
                    Ok(last_val)
                }
                other => Err(BioLangError::type_error(
                    format!("last() requires List, Table, or Stream, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "drop" => {
            let n = if args.len() > 1 {
                require_int(&args[1], "drop")? as usize
            } else {
                1
            };
            match &args[0] {
                Value::List(l) => {
                    let start = n.min(l.len());
                    Ok(Value::List(l[start..].to_vec()))
                }
                Value::Table(t) => {
                    let start = n.min(t.rows.len());
                    let rows = t.rows[start..].to_vec();
                    Ok(Value::Table(Table::new(t.columns.clone(), rows)))
                }
                other => Err(BioLangError::type_error(
                    format!("drop() requires List or Table, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "window" => {
            // Handle bio sequences: convert to list of single-char sequences
            if let Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) = &args[0] {
                let size = require_int(&args[1], "window")? as usize;
                if size == 0 || size > seq.data.len() {
                    return Ok(Value::List(vec![]));
                }
                let windows: Vec<Value> = (0..=seq.data.len() - size)
                    .map(|i| {
                        let sub = bl_core::value::BioSequence { data: seq.data[i..i + size].to_string() };
                        match &args[0] {
                            Value::DNA(_) => Value::DNA(sub),
                            Value::RNA(_) => Value::RNA(sub),
                            Value::Protein(_) => Value::Protein(sub),
                            _ => unreachable!(),
                        }
                    })
                    .collect();
                return Ok(Value::List(windows));
            }
            let owned;
            let items = match &args[0] {
                Value::List(l) => l,
                Value::Table(t) => { owned = table_to_records(t); &owned }
                other => {
                    return Err(BioLangError::type_error(
                        format!("window() requires List, Table, or bio sequence, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let size = require_int(&args[1], "window")? as usize;
            if size == 0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "window() size must be > 0",
                    None,
                ));
            }
            if items.len() < size {
                return Ok(Value::List(vec![]));
            }
            let windows: Vec<Value> = items
                .windows(size)
                .map(|w| Value::List(w.to_vec()))
                .collect();
            Ok(Value::List(windows))
        }
        "frequencies" => {
            let items = match args.into_iter().next().unwrap() {
                Value::List(l) => l,
                Value::Table(t) => table_to_records(&t),
                Value::Stream(s) => {
                    let mut v = Vec::new();
                    while let Some(item) = s.next() {
                        v.push(item);
                        if v.len() == 1_000_001 {
                            eprintln!(
                                "\x1b[33mWarning:\x1b[0m frequencies() is consuming a large stream into memory."
                            );
                            eprintln!("  Tip: Use head(n) before frequencies() to limit memory usage.");
                        }
                    }
                    v
                }
                other => {
                    return Err(BioLangError::type_error(
                        format!("frequencies() requires List or Stream, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let mut counts: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
            for item in &items {
                let key = format!("{}", item);
                let entry = counts.entry(key).or_insert(Value::Int(0));
                if let Value::Int(n) = entry {
                    *n += 1;
                }
            }
            Ok(Value::Record(counts))
        }
        "repeat" => {
            match (&args[0], &args[1]) {
                (Value::Str(s), Value::Int(n)) => {
                    let n = (*n).max(0) as usize;
                    let total = s.len().saturating_mul(n);
                    if total > 100_000_000 {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!(
                                "repeat() would produce {total} bytes (limit: 100 MB)"
                            ),
                            None,
                        ));
                    }
                    Ok(Value::Str(s.repeat(n)))
                }
                (Value::List(l), Value::Int(n)) => {
                    let n = (*n).max(0) as usize;
                    let total = l.len().saturating_mul(n);
                    if total > 10_000_000 {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!(
                                "repeat() would produce {total} elements (limit: 10,000,000)"
                            ),
                            None,
                        ));
                    }
                    let mut result = Vec::with_capacity(total);
                    for _ in 0..n {
                        result.extend(l.iter().cloned());
                    }
                    Ok(Value::List(result))
                }
                (other, _) => Err(BioLangError::type_error(
                    format!("repeat() requires String or List as first argument, got {}", other.type_of()),
                    None,
                )),
            }
        }
        "col" => {
            let table = match &args[0] {
                Value::Table(t) => t,
                other => {
                    return Err(BioLangError::type_error(
                        format!("col() requires Table, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let col_name = match &args[1] {
                Value::Str(s) => s.as_str(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("col() requires String column name, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let col_idx = table
                .columns
                .iter()
                .position(|c| c == &col_name)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::NameError,
                        format!("col(): column '{}' not found", col_name),
                        None,
                    )
                })?;
            let values: Vec<Value> = table.rows.iter().map(|row| row[col_idx].clone()).collect();
            Ok(Value::List(values))
        }
        "jaccard" => {
            let set_a: std::collections::HashSet<String> = match &args[0] {
                Value::List(l) => l.iter().map(|v| format!("{v}")).collect(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("jaccard() requires List, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let set_b: std::collections::HashSet<String> = match &args[1] {
                Value::List(l) => l.iter().map(|v| format!("{v}")).collect(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("jaccard() requires List, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let intersection = set_a.intersection(&set_b).count() as f64;
            let union = set_a.union(&set_b).count() as f64;
            if union == 0.0 {
                Ok(Value::Float(0.0))
            } else {
                Ok(Value::Float(intersection / union))
            }
        }
        "slice" => {
            let items = match &args[0] {
                Value::List(l) => l,
                Value::Str(s) => {
                    let start = require_int(&args[1], "slice")? as usize;
                    let end = if args.len() > 2 {
                        require_int(&args[2], "slice")? as usize
                    } else {
                        s.len()
                    };
                    let end = end.min(s.len());
                    let start = start.min(end);
                    return Ok(Value::Str(s[start..end].to_string()));
                }
                Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => {
                    let start = require_int(&args[1], "slice")? as usize;
                    let end = if args.len() > 2 {
                        require_int(&args[2], "slice")? as usize
                    } else {
                        seq.data.len()
                    };
                    let end = end.min(seq.data.len());
                    let start = start.min(end);
                    let sliced = bl_core::value::BioSequence { data: seq.data[start..end].to_string() };
                    return Ok(match &args[0] {
                        Value::DNA(_) => Value::DNA(sliced),
                        Value::RNA(_) => Value::RNA(sliced),
                        Value::Protein(_) => Value::Protein(sliced),
                        _ => unreachable!(),
                    });
                }
                Value::Table(_) => {
                    return crate::table_ops::call_table_builtin("slice", args);
                }
                other => {
                    return Err(BioLangError::type_error(
                        format!("slice() requires List, Str, Table, or bio sequence, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let start = require_int(&args[1], "slice")? as usize;
            let end = if args.len() > 2 {
                require_int(&args[2], "slice")? as usize
            } else {
                items.len()
            };
            let end = end.min(items.len());
            let start = start.min(end);
            Ok(Value::List(items[start..end].to_vec()))
        }
        // ── Dual-dispatch: List vs Table ─────────────────────────
        "cumsum" => match &args[0] {
            Value::Table(_) => crate::table_ops::call_table_builtin("cumsum", args),
            _ => crate::stats::call_stats_builtin("cumsum", args),
        },
        "sample" => match &args[0] {
            Value::Table(_) => crate::table_ops::call_table_builtin("sample", args),
            _ => crate::stats::call_stats_builtin("sample", args),
        },
        "concat" => match (&args[0], &args[1]) {
            (Value::List(a), Value::List(b)) => {
                let mut result = a.clone();
                result.extend(b.iter().cloned());
                Ok(Value::List(result))
            }
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
            (Value::Table(_), Value::Table(_)) => {
                // Delegate to table_ops concat which supports variadic
                crate::table_ops::call_table_builtin("concat", args)
            }
            _ => Err(BioLangError::type_error(
                "concat() requires two Lists, Strs, or Tables",
                None,
            )),
        },
        // ── Map/Record operations ────────────────────────────────
        "merge" => match (&args[0], &args[1]) {
            (Value::Map(a), Value::Map(b)) | (Value::Record(a), Value::Record(b)) => {
                let mut result = a.clone();
                for (k, v) in b {
                    result.insert(k.clone(), v.clone());
                }
                if matches!(&args[0], Value::Record(_)) {
                    Ok(Value::Record(result))
                } else {
                    Ok(Value::Map(result))
                }
            }
            _ => Err(BioLangError::type_error(
                "merge() requires two Maps or two Records",
                None,
            )),
        },
        "has_key" => match (&args[0], &args[1]) {
            (Value::Map(m), Value::Str(k)) | (Value::Record(m), Value::Str(k)) => {
                Ok(Value::Bool(m.contains_key(k)))
            }
            _ => Err(BioLangError::type_error(
                "has_key() requires (Map/Record, Str)",
                None,
            )),
        },
        "remove_key" => match (&args[0], &args[1]) {
            (Value::Map(m), Value::Str(k)) => {
                let mut result = m.clone();
                result.remove(k);
                Ok(Value::Map(result))
            }
            (Value::Record(m), Value::Str(k)) => {
                let mut result = m.clone();
                result.remove(k);
                Ok(Value::Record(result))
            }
            _ => Err(BioLangError::type_error(
                "remove_key() requires (Map/Record, Str)",
                None,
            )),
        },
        // ── Error handling ───────────────────────────────────────
        "error" => {
            let msg = match &args[0] {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            };
            Err(BioLangError::runtime(ErrorKind::TypeError, msg, None))
        }
        // ── Type predicates ────────────────────────────────────
        "is_nil" => Ok(Value::Bool(matches!(args[0], Value::Nil))),
        "is_int" => Ok(Value::Bool(matches!(args[0], Value::Int(_)))),
        "is_float" => Ok(Value::Bool(matches!(args[0], Value::Float(_)))),
        "is_num" => Ok(Value::Bool(matches!(args[0], Value::Int(_) | Value::Float(_)))),
        "is_str" => Ok(Value::Bool(matches!(args[0], Value::Str(_)))),
        "is_bool" => Ok(Value::Bool(matches!(args[0], Value::Bool(_)))),
        "is_list" => Ok(Value::Bool(matches!(args[0], Value::List(_)))),
        "is_map" => Ok(Value::Bool(matches!(args[0], Value::Map(_)))),
        "is_record" => Ok(Value::Bool(matches!(args[0], Value::Record(_)))),
        "is_table" => Ok(Value::Bool(matches!(args[0], Value::Table(_)))),
        "is_function" => Ok(Value::Bool(matches!(
            args[0],
            Value::Function { .. } | Value::NativeFunction { .. } | Value::PluginFunction { .. }
        ))),
        "is_dna" => Ok(Value::Bool(matches!(args[0], Value::DNA(_)))),
        "is_rna" => Ok(Value::Bool(matches!(args[0], Value::RNA(_)))),
        "is_protein" => Ok(Value::Bool(matches!(args[0], Value::Protein(_)))),
        "is_interval" => Ok(Value::Bool(matches!(args[0], Value::Interval(_)))),
        "is_matrix" => Ok(Value::Bool(matches!(args[0], Value::Matrix(_)))),
        "is_stream" => Ok(Value::Bool(matches!(args[0], Value::Stream(_)))),
        "is_range" => Ok(Value::Bool(matches!(args[0], Value::Range { .. }))),
        "is_enum" => Ok(Value::Bool(matches!(args[0], Value::EnumValue { .. }))),
        "is_set" => Ok(Value::Bool(matches!(args[0], Value::Set(_)))),
        "is_regex" => Ok(Value::Bool(matches!(args[0], Value::Regex { .. }))),
        "is_future" => Ok(Value::Bool(matches!(args[0], Value::Future(_)))),
        "is_kmer" => Ok(Value::Bool(matches!(args[0], Value::Kmer(_)))),
        "is_sparse" => Ok(Value::Bool(matches!(args[0], Value::SparseMatrix(_)))),
        // ── Set operations ───────────────────────────────────────
        "set" => match &args[0] {
            Value::List(items) => {
                let mut result = Vec::new();
                for item in items {
                    if !result.contains(item) {
                        result.push(item.clone());
                    }
                }
                Ok(Value::Set(result))
            }
            Value::Set(s) => Ok(Value::Set(s.clone())),
            other => Err(BioLangError::type_error(
                format!("set() requires List or Set, got {}", other.type_of()),
                None,
            )),
        },
        "union" => {
            let (a, b) = require_two_sets(&args)?;
            let mut result = a;
            for item in b {
                if !result.contains(&item) {
                    result.push(item);
                }
            }
            Ok(Value::Set(result))
        }
        "intersection" => {
            let (a, b) = require_two_sets(&args)?;
            let result: Vec<Value> = a.into_iter().filter(|item| b.contains(item)).collect();
            Ok(Value::Set(result))
        }
        "difference" => {
            let (a, b) = require_two_sets(&args)?;
            let result: Vec<Value> = a.into_iter().filter(|item| !b.contains(item)).collect();
            Ok(Value::Set(result))
        }
        "symmetric_difference" => {
            let (a, b) = require_two_sets(&args)?;
            let mut result: Vec<Value> = a.iter().filter(|item| !b.contains(item)).cloned().collect();
            for item in &b {
                if !a.contains(item) {
                    result.push(item.clone());
                }
            }
            Ok(Value::Set(result))
        }
        "is_subset" => {
            let (a, b) = require_two_sets(&args)?;
            Ok(Value::Bool(a.iter().all(|item| b.contains(item))))
        }
        "is_superset" => {
            let (a, b) = require_two_sets(&args)?;
            Ok(Value::Bool(b.iter().all(|item| a.contains(item))))
        }
        // ── Async: await_all ─────────────────────────────────────
        "await_all" => {
            let futures = match &args[0] {
                Value::List(l) => l.clone(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("await_all() requires List, got {}", other.type_of()),
                        None,
                    ))
                }
            };
            let results: Vec<Value> = futures
                .into_iter()
                .map(|v| match v {
                    Value::Future(ref state) => {
                        let mut guard = state.lock().map_err(|_| {
                            BioLangError::runtime(ErrorKind::TypeError, "future lock poisoned", None)
                        })?;
                        match &*guard {
                            bl_core::value::FutureState::Resolved(val) => Ok(val.clone()),
                            bl_core::value::FutureState::Pending { .. } => {
                                // For now, futures are resolved on await in interpreter
                                // If still pending, just return Nil
                                *guard = bl_core::value::FutureState::Resolved(Value::Nil);
                                Ok(Value::Nil)
                            }
                        }
                    }
                    other => Ok(other),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Value::List(results))
        }
        // ── Decorator builtins ───────────────────────────────────
        "memoize" | "time_it" | "once" => {
            // These are handled as decorators in the interpreter.
            // When called directly as functions, just return the input function.
            Ok(args.into_iter().next().unwrap_or(Value::Nil))
        }
        // ── Genomic range queries (F13) ──────────────────────────
        "interval_tree" => builtin_interval_tree(&args),
        "query_overlaps" => builtin_query_overlaps(&args),
        "count_overlaps" => builtin_count_overlaps(&args),
        "bulk_overlaps" => builtin_bulk_overlaps(&args),
        "query_nearest" => builtin_query_nearest(&args),
        "coverage" => {
            // Interval-tree coverage for Record, viz coverage for List
            if matches!(&args[0], Value::List(_)) {
                crate::viz::call_viz_builtin("coverage", args)
            } else {
                builtin_coverage(&args)
            }
        }
        // ── Sequence pattern matching (F14) ──────────────────────
        "motif_find" => builtin_motif_find(&args),
        "motif_count" => builtin_motif_count(&args),
        "consensus" => builtin_consensus(&args),
        "pwm" => builtin_pwm(&args),
        "pwm_scan" => builtin_pwm_scan(&args),
        // ── Pipeline steps (F15) ─────────────────────────────────
        "pipeline_steps" => builtin_pipeline_steps(&args),
        // F7: help() — show function doc and signature
        "help" => {
            match &args[0] {
                Value::Function { name, params, doc, is_generator, .. } => {
                    let fn_name = name.as_deref().unwrap_or("anonymous");
                    let gen_marker = if *is_generator { "*" } else { "" };
                    let param_strs: Vec<String> = params
                        .iter()
                        .map(|p| {
                            let rest = if p.rest { "..." } else { "" };
                            let ann = p
                                .type_ann
                                .as_ref()
                                .map(|t| format!(": {}", t.name))
                                .unwrap_or_default();
                            format!("{rest}{}{ann}", p.name)
                        })
                        .collect();
                    let sig = format!("fn{gen_marker} {fn_name}({})", param_strs.join(", "));
                    let doc_str = doc.as_deref().unwrap_or("(no documentation)");
                    Ok(Value::Str(format!("{sig}\n\n{doc_str}")))
                }
                Value::NativeFunction { name, arity } => {
                    let arity_str = match arity {
                        Arity::Exact(n) => format!("{n} args"),
                        Arity::AtLeast(n) => format!("{n}+ args"),
                        Arity::Range(a, b) => format!("{a}-{b} args"),
                    };
                    Ok(Value::Str(format!("<builtin {name}> ({arity_str})")))
                }
                other => Ok(Value::Str(format!("{}: {}", other.type_of(), other))),
            }
        }
        // F11: Unit builtins — return Int (bp count) for composable arithmetic
        "bp" => {
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                _ => Err(BioLangError::type_error("bp() requires number", None)),
            }
        }
        "kb" => {
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n * 1_000)),
                Value::Float(f) => Ok(Value::Int((*f * 1_000.0) as i64)),
                _ => Err(BioLangError::type_error("kb() requires number", None)),
            }
        }
        "mb" => {
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n * 1_000_000)),
                Value::Float(f) => Ok(Value::Int((*f * 1_000_000.0) as i64)),
                _ => Err(BioLangError::type_error("mb() requires number", None)),
            }
        }
        "gb" => {
            match &args[0] {
                Value::Int(n) => Ok(Value::Int(*n * 1_000_000_000)),
                Value::Float(f) => Ok(Value::Int((*f * 1_000_000_000.0) as i64)),
                _ => Err(BioLangError::type_error("gb() requires number", None)),
            }
        }
        // F13: PRNG generators for property testing
        "gen_int" => {
            let (min, max) = match args.len() {
                1 => (0, require_int(&args[0], "gen_int")?),
                2 => (require_int(&args[0], "gen_int")?, require_int(&args[1], "gen_int")?),
                3 => {
                    let seed = require_int(&args[2], "gen_int")?;
                    let min = require_int(&args[0], "gen_int")?;
                    let max = require_int(&args[1], "gen_int")?;
                    // Simple LCG PRNG
                    let val = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % (max - min + 1).max(1)) + min;
                    return Ok(Value::Int(val.abs()));
                }
                _ => unreachable!(),
            };
            // Without seed, use a simple hash of current time
            let val = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(42);
            let result = (val.abs() % (max - min + 1).max(1)) + min;
            Ok(Value::Int(result))
        }
        "gen_float" => {
            let val = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| (d.as_nanos() % 1_000_000) as f64 / 1_000_000.0)
                .unwrap_or(0.5);
            Ok(Value::Float(val))
        }
        "gen_str" => {
            let len = if args.is_empty() { 10 } else { require_int(&args[0], "gen_str")? as usize };
            let chars = "abcdefghijklmnopqrstuvwxyz";
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as usize)
                .unwrap_or(42);
            let s: String = (0..len)
                .map(|i| {
                    let idx = (seed.wrapping_add(i.wrapping_mul(7919))) % chars.len();
                    chars.as_bytes()[idx] as char
                })
                .collect();
            Ok(Value::Str(s))
        }
        // any, all, find, find_index are HOFs — handled in interpreter.rs
        // try_call is a HOF — handled in interpreter.rs
        // map, filter, reduce, sort are handled in interpreter (they take closures)
        // Try module builtins, then table ops, then stats, then bio
        _ if crate::text_ops::is_text_builtin(name) => {
            crate::text_ops::call_text_builtin(name, args)
        }
        _ if crate::hash::is_hash_builtin(name) => crate::hash::call_hash_builtin(name, args),
        _ if crate::datetime::is_datetime_builtin(name) => {
            crate::datetime::call_datetime_builtin(name, args)
        }
        _ if crate::json::is_json_builtin(name) => crate::json::call_json_builtin(name, args),
        #[cfg(feature = "native")]
        _ if crate::fs::is_fs_builtin(name) => crate::fs::call_fs_builtin(name, args),
        #[cfg(feature = "native")]
        _ if crate::http::is_http_builtin(name) => crate::http::call_http_builtin(name, args),
        _ if crate::regex_ops::is_regex_builtin(name) => {
            crate::regex_ops::call_regex_builtin(name, args)
        }
        _ if crate::csv::is_csv_builtin(name) => crate::csv::call_csv_builtin(name, args),
        #[cfg(feature = "native")]
        _ if crate::parquet::is_parquet_builtin(name) => crate::parquet::call_parquet_builtin(name, args),
        _ if crate::table_ops::is_table_builtin(name) => {
            crate::table_ops::call_table_builtin(name, args)
        }
        _ if crate::stats::is_stats_builtin(name) => {
            crate::stats::call_stats_builtin(name, args)
        }
        _ if crate::plot::is_plot_builtin(name) => {
            crate::plot::call_plot_builtin(name, args)
        }
        _ if crate::matrix::is_matrix_builtin(name) => {
            crate::matrix::call_matrix_builtin(name, args)
        }
        _ if crate::bio_plots::is_bio_plots_builtin(name) => {
            crate::bio_plots::call_bio_plots_builtin(name, args)
        }
        _ if crate::viz::is_viz_builtin(name) => {
            crate::viz::call_viz_builtin(name, args)
        }
        // GAP 5: Sparse matrix operations
        _ if crate::sparse::is_sparse_builtin(name) => {
            crate::sparse::call_sparse_builtin(name, args)
        }
        // GAP 10: Bio operations (graph, phylo, dim reduction, clustering, diff expr)
        _ if crate::bio_ops::is_bio_ops_builtin(name) => {
            crate::bio_ops::call_bio_ops_builtin(name, args)
        }
        // GAP 1: Coordinate system builtins
        "coord_bed" => builtin_coord_tag(args, "bed"),
        "coord_vcf" => builtin_coord_tag(args, "vcf"),
        "coord_gff" => builtin_coord_tag(args, "gff"),
        "coord_sam" => builtin_coord_tag(args, "sam"),
        "coord_convert" => builtin_coord_convert(args),
        "coord_system" => builtin_coord_system(args),
        "coord_check" => builtin_coord_check(args),
        "strip_chr" => builtin_strip_chr(args),
        "add_chr" => builtin_add_chr(args),
        "normalize_chrom" => builtin_normalize_chrom(args),
        // GAP 2: K-mer builtins
        "kmer_encode" => builtin_kmer_encode(args),
        "kmer_decode" => builtin_kmer_decode(args),
        "kmer_rc" => builtin_kmer_rc(args),
        "kmer_canonical" => builtin_kmer_canonical(args),
        "kmer_count" => builtin_kmer_count(args),
        "kmer_distinct" => builtin_kmer_distinct(args),
        "kmer_spectrum" => builtin_kmer_spectrum(args),
        "minimizers" => builtin_minimizers(args),
        // GAP 3: Streaming
        "stream_chunks" => builtin_stream_chunks(args),
        "stream_take" => builtin_stream_take(args),
        "stream_skip" => builtin_stream_skip(args),
        // stream_batch is a HOF dispatched in interpreter.rs
        "memory_usage" => builtin_memory_usage(),
        // GAP 6: Typed table columns
        "table_col_types" => builtin_table_col_types(args),
        "table_set_col_type" => builtin_table_set_col_type(args),
        "table_validate" => builtin_table_validate(args),
        "table_schema" => builtin_table_schema(args),
        "table_cast" => builtin_table_cast(args),
        // GAP 7: Pipe fusion (explicit)
        "pipe_fuse" => builtin_pipe_fuse(args),
        // GAP 8: Provenance
        "with_provenance" => builtin_with_provenance(args),
        "provenance" => builtin_provenance(args),
        "provenance_chain" => builtin_provenance_chain(args),
        "checkpoint" => builtin_checkpoint(args),
        "resume_checkpoint" => builtin_resume_checkpoint(args),
        // ── tee/tap/inspect: side-effect pass-through ──────────────
        // These are HOFs dispatched in interpreter.rs; these arms are fallbacks
        "tee" | "tap" | "inspect" | "group_apply" => Ok(args.into_iter().next().unwrap_or(Value::Nil)),
        // ── Bio type constructors ──────────────────────────────────
        "gene" => builtin_gene(args),
        "variant" => builtin_variant(args),
        "genome" => builtin_genome(args),
        // ── Type predicates ────────────────────────────────────────
        "is_gene" => Ok(Value::Bool(matches!(&args[0], Value::Gene { .. }))),
        "is_variant" => Ok(Value::Bool(matches!(&args[0], Value::Variant { .. }))),
        "is_genome" => Ok(Value::Bool(matches!(&args[0], Value::Genome { .. }))),
        "is_quality" => Ok(Value::Bool(matches!(&args[0], Value::Quality(_)))),
        "is_aligned_read" => Ok(Value::Bool(matches!(&args[0], Value::AlignedRead(_)))),
        "aligned_read" => builtin_aligned_read(args),
        "flagstat" => builtin_flagstat(&args),
        // ── Variant classification builtins ───────────────────────────
        "is_snp" => builtin_variant_predicate(&args, "is_snp"),
        "is_indel" => builtin_variant_predicate(&args, "is_indel"),
        "is_transition" => builtin_variant_predicate(&args, "is_transition"),
        "is_transversion" => builtin_variant_predicate(&args, "is_transversion"),
        "is_het" => builtin_variant_predicate(&args, "is_het"),
        "is_hom_ref" => builtin_variant_predicate(&args, "is_hom_ref"),
        "is_hom_alt" => builtin_variant_predicate(&args, "is_hom_alt"),
        "is_multiallelic" => builtin_variant_predicate(&args, "is_multiallelic"),
        "variant_type" => builtin_variant_type(&args),
        "variant_summary" => builtin_variant_summary(&args),
        "tstv_ratio" => builtin_tstv_ratio(&args),
        "het_hom_ratio" => builtin_het_hom_ratio(&args),
        "parse_vcf_info" => builtin_parse_vcf_info(&args),
        #[cfg(feature = "native")]
        _ if crate::enrich::is_enrich_builtin(name) => {
            crate::enrich::call_enrich_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::apis::is_apis_builtin(name) => {
            crate::apis::call_apis_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::container::is_container_builtin(name) => {
            crate::container::call_container_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::llm::is_llm_builtin(name) => {
            crate::llm::call_llm_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::transfer::is_transfer_builtin(name) => {
            crate::transfer::call_transfer_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::nf_parse::is_nf_parse_builtin(name) => {
            crate::nf_parse::call_nf_parse_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::notify::is_notify_builtin(name) => {
            crate::notify::call_notify_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ if crate::sqlite::is_sqlite_builtin(name) => {
            crate::sqlite::call_sqlite_builtin(name, args)
        }
        _ if crate::markdown::is_markdown_builtin(name) => {
            crate::markdown::call_markdown_builtin(name, args)
        }
        _ if crate::graph::is_graph_builtin(name) => {
            crate::graph::call_graph_builtin(name, args)
        }
        // Core sequence builtins (WASM-safe — transcribe, translate, gc_content, etc.)
        _ if crate::seq::is_seq_builtin(name) => {
            crate::seq::call_seq_builtin(name, args)
        }
        // NCBI E-utilities (WASM-safe — uses fetch hook for browser API calls)
        // Only reached on WASM builds; on native, apis.rs handles these above.
        #[cfg(not(feature = "native"))]
        _ if crate::ncbi_wasm::is_ncbi_wasm_builtin(name) => {
            crate::ncbi_wasm::call_ncbi_wasm_builtin(name, args)
        }
        #[cfg(feature = "native")]
        _ => {
            let result = bl_bio::call_bio_builtin(name, args);
            if result.is_err() {
                let mut err = result.unwrap_err();
                if err.kind == ErrorKind::NameError {
                    if let Some(suggestion) = suggest_builtin(name) {
                        err = err.with_suggestion(format!("did you mean '{suggestion}'?"));
                    }
                }
                Err(err)
            } else {
                result
            }
        }
        #[cfg(not(feature = "native"))]
        _ if crate::bio_wasm::is_bio_wasm_builtin(name) => {
            crate::bio_wasm::call_bio_wasm_builtin(name, args)
        }
        #[cfg(not(feature = "native"))]
        _ => {
            let mut err = BioLangError::runtime(
                ErrorKind::NameError,
                format!("unknown function: {name}"),
                None,
            );
            if let Some(suggestion) = suggest_builtin(name) {
                err = err.with_suggestion(format!("did you mean '{suggestion}'?"));
            }
            Err(err)
        }
    }
}

// ── Bio type constructors ─────────────────────────────────────────

fn builtin_gene(args: Vec<Value>) -> Result<Value> {
    use std::collections::HashMap;
    match &args[0] {
        Value::Record(map) | Value::Map(map) => {
            let get_str = |m: &HashMap<String, Value>, k: &str| -> String {
                m.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let get_int = |m: &HashMap<String, Value>, k: &str| -> i64 {
                m.get(k).and_then(|v| v.as_int()).unwrap_or(0)
            };
            let symbol = get_str(map, "symbol");
            if symbol.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "gene() symbol must not be empty",
                    None,
                ));
            }
            let start = get_int(map, "start");
            let end = get_int(map, "end");
            if end != 0 && end < start {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("gene() end ({end}) must be >= start ({start})"),
                    None,
                ));
            }
            Ok(Value::Gene {
                symbol,
                gene_id: get_str(map, "gene_id"),
                chrom: get_str(map, "chrom"),
                start,
                end,
                strand: get_str(map, "strand"),
                biotype: get_str(map, "biotype"),
                description: get_str(map, "description"),
            })
        }
        Value::Str(symbol) => {
            if symbol.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "gene() symbol must not be empty",
                    None,
                ));
            }
            // Construct a Gene with just a symbol (lookup can be done via ensembl_gene)
            Ok(Value::Gene {
                symbol: symbol.clone(),
                gene_id: String::new(),
                chrom: String::new(),
                start: 0,
                end: 0,
                strand: "+".into(),
                biotype: String::new(),
                description: String::new(),
            })
        }
        _ => Err(BioLangError::type_error("gene() requires Record or Str", None)),
    }
}

fn builtin_variant(args: Vec<Value>) -> Result<Value> {
    use std::collections::HashMap;
    // Positional form: variant("chr7", 55181378, "T", "A")
    if args.len() >= 2 {
        let chrom = match &args[0] {
            Value::Str(s) => s.clone(),
            other => return Err(BioLangError::type_error(
                format!("variant() chrom must be String, got {}", other.type_of()), None)),
        };
        let pos = match &args[1] {
            Value::Int(n) => *n,
            other => return Err(BioLangError::type_error(
                format!("variant() pos must be Int, got {}", other.type_of()), None)),
        };
        if pos < 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("variant() pos must be non-negative, got {pos}"),
                None,
            ));
        }
        let ref_allele = if args.len() >= 3 {
            match &args[2] {
                Value::Str(s) => s.clone(),
                other => return Err(BioLangError::type_error(
                    format!("variant() ref must be String, got {}", other.type_of()), None)),
            }
        } else { String::new() };
        let alt_allele = if args.len() >= 4 {
            match &args[3] {
                Value::Str(s) => s.clone(),
                other => return Err(BioLangError::type_error(
                    format!("variant() alt must be String, got {}", other.type_of()), None)),
            }
        } else { String::new() };
        // Validate allele characters (IUPAC DNA + . for missing + * for deletion)
        let valid_allele = |s: &str| s.is_empty() || s.chars().all(|c| "ACGTNRYWSMKBDHVacgtnrywsmkbdhv.*".contains(c));
        if !valid_allele(&ref_allele) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("variant() ref allele contains invalid characters: {ref_allele}"),
                None,
            ));
        }
        if !valid_allele(&alt_allele) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("variant() alt allele contains invalid characters: {alt_allele}"),
                None,
            ));
        }
        return Ok(Value::Variant {
            chrom, pos, id: String::new(), ref_allele, alt_allele,
            quality: 0.0, filter: String::new(), info: HashMap::new(),
        });
    }
    // Record form: variant({chrom: "chr7", pos: 55181378, ref: "T", alt: "A", ...})
    match &args[0] {
        Value::Record(map) | Value::Map(map) => {
            let get_str = |m: &HashMap<String, Value>, k: &str| -> String {
                m.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let get_int = |m: &HashMap<String, Value>, k: &str| -> i64 {
                m.get(k).and_then(|v| v.as_int()).unwrap_or(0)
            };
            let get_float = |m: &HashMap<String, Value>, k: &str| -> f64 {
                m.get(k).and_then(|v| v.as_float()).unwrap_or(0.0)
            };
            let info = map.get("info").and_then(|v| match v {
                Value::Record(m) | Value::Map(m) => Some(m.clone()),
                _ => None,
            }).unwrap_or_default();
            Ok(Value::Variant {
                chrom: get_str(map, "chrom"),
                pos: get_int(map, "pos"),
                id: get_str(map, "id"),
                ref_allele: get_str(map, "ref"),
                alt_allele: get_str(map, "alt"),
                quality: get_float(map, "quality"),
                filter: get_str(map, "filter"),
                info,
            })
        }
        _ => Err(BioLangError::type_error("variant() requires Record or positional args (chrom, pos, ref, alt)", None)),
    }
}

fn builtin_genome(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Str(name) => {
            match bl_core::bio_core::Genome::from_name(name) {
                Some(g) => Ok(Value::Genome {
                    name: g.name,
                    species: g.species,
                    assembly: g.assembly,
                    chromosomes: g.chromosomes,
                }),
                None => Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("unknown genome '{name}'. Known: GRCh38, GRCh37, T2T-CHM13, GRCm39"),
                    None,
                )),
            }
        }
        Value::Record(map) | Value::Map(map) => {
            let name = map.get("name").and_then(|v| v.as_str()).unwrap_or("custom").to_string();
            let species = map.get("species").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let assembly = map.get("assembly").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let chroms = map.get("chromosomes").and_then(|v| match v {
                Value::List(items) => {
                    Some(items.iter().filter_map(|item| match item {
                        Value::Record(m) | Value::Map(m) => {
                            let n = m.get("name")?.as_str()?.to_string();
                            let l = m.get("length")?.as_int()?;
                            Some((n, l))
                        }
                        _ => None,
                    }).collect())
                }
                _ => None,
            }).unwrap_or_default();
            Ok(Value::Genome { name, species, assembly, chromosomes: chroms })
        }
        _ => Err(BioLangError::type_error("genome() requires Str or Record", None)),
    }
}

// ── doctor() ─────────────────────────────────────────────────────

#[cfg(feature = "native")]
fn builtin_doctor() -> Result<Value> {
    use std::collections::HashMap;

    let mut checks: Vec<Value> = Vec::new();

    // Helper to push a check row
    let mut add = |category: &str, name: &str, status: &str, value: &str, fix: &str| {
        let mut rec = HashMap::new();
        rec.insert("category".to_string(), Value::Str(category.to_string()));
        rec.insert("check".to_string(), Value::Str(name.to_string()));
        rec.insert("status".to_string(), Value::Str(status.to_string()));
        rec.insert("value".to_string(), Value::Str(value.to_string()));
        rec.insert("fix".to_string(), Value::Str(fix.to_string()));
        checks.push(Value::Record(rec));
    };

    // ── Container Runtime ────────────────────────────────────
    let runtimes = ["docker", "podman", "singularity", "apptainer"];
    let mut any_runtime = false;
    for rt in &runtimes {
        match std::process::Command::new(rt).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                add("container", rt, "ok", &ver, "");
                any_runtime = true;
            }
            _ => {}
        }
    }
    if !any_runtime {
        add(
            "container",
            "runtime",
            "missing",
            "none found",
            "install docker, podman, singularity, or apptainer",
        );
    }

    // ── Image Cache Dir ──────────────────────────────────────
    let image_dir_env = std::env::var("BIOLANG_IMAGE_DIR").ok();
    match &image_dir_env {
        Some(dir) => {
            let exists = std::path::Path::new(dir).is_dir();
            add(
                "container",
                "BIOLANG_IMAGE_DIR",
                if exists { "ok" } else { "warn" },
                dir,
                if exists {
                    ""
                } else {
                    "directory does not exist yet (will be created on first pull)"
                },
            );
        }
        None => add(
            "container",
            "BIOLANG_IMAGE_DIR",
            "info",
            "not set (using default ~/.biolang/images/)",
            "set BIOLANG_IMAGE_DIR to customize image storage location",
        ),
    }

    // ── LLM Providers ────────────────────────────────────────
    let llm_vars = [
        ("ANTHROPIC_API_KEY", "Anthropic (Claude)"),
        ("OPENAI_API_KEY", "OpenAI (GPT)"),
        ("OLLAMA_MODEL", "Ollama (local)"),
        ("LLM_BASE_URL", "OpenAI-compatible"),
    ];
    let mut any_llm = false;
    for (var, label) in &llm_vars {
        if let Ok(val) = std::env::var(var) {
            let display = if var.contains("KEY") {
                // Mask API keys — show first 8 chars + ...
                if val.len() > 8 {
                    format!("{}...", &val[..8])
                } else {
                    "****".to_string()
                }
            } else {
                val.clone()
            };
            add("llm", var, "ok", &display, label);
            any_llm = true;
        }
    }
    if !any_llm {
        add(
            "llm",
            "provider",
            "missing",
            "none configured",
            "set ANTHROPIC_API_KEY, OPENAI_API_KEY, OLLAMA_MODEL, or LLM_BASE_URL",
        );
    }

    // ── Optional LLM Overrides ───────────────────────────────
    let llm_overrides = [
        "ANTHROPIC_MODEL",
        "OPENAI_MODEL",
        "OPENAI_BASE_URL",
        "OLLAMA_BASE_URL",
        "LLM_API_KEY",
        "LLM_MODEL",
    ];
    for var in &llm_overrides {
        if let Ok(val) = std::env::var(var) {
            let display = if var.contains("KEY") {
                if val.len() > 8 {
                    format!("{}...", &val[..8])
                } else {
                    "****".to_string()
                }
            } else {
                val
            };
            add("llm", var, "ok", &display, "");
        }
    }

    // ── Bio API Keys ─────────────────────────────────────────
    if let Ok(val) = std::env::var("NCBI_API_KEY") {
        let display = if val.len() > 8 {
            format!("{}...", &val[..8])
        } else {
            "****".to_string()
        };
        add("api", "NCBI_API_KEY", "ok", &display, "10 req/s rate limit");
    } else {
        add(
            "api",
            "NCBI_API_KEY",
            "info",
            "not set",
            "set NCBI_API_KEY for higher rate limits (10/s vs 3/s)",
        );
    }

    if let Ok(val) = std::env::var("COSMIC_API_KEY") {
        let display = if val.len() > 8 {
            format!("{}...", &val[..8])
        } else {
            "****".to_string()
        };
        add("api", "COSMIC_API_KEY", "ok", &display, "");
    } else {
        add(
            "api",
            "COSMIC_API_KEY",
            "info",
            "not set",
            "required for cosmic_gene() — get key from cancer.sanger.ac.uk",
        );
    }

    // ── System Tools ─────────────────────────────────────────
    let tools_to_check = ["git", "curl", "wget"];
    for tool in &tools_to_check {
        match std::process::Command::new(tool).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let ver = String::from_utf8_lossy(&output.stdout);
                let first_line = ver.lines().next().unwrap_or("").trim().to_string();
                add("system", tool, "ok", &first_line, "");
            }
            _ => {
                add("system", tool, "info", "not found", "");
            }
        }
    }

    // ── BioLang Config ───────────────────────────────────────
    if let Ok(path) = std::env::var("BIOLANG_PATH") {
        add("config", "BIOLANG_PATH", "ok", &path, "");
    } else {
        add(
            "config",
            "BIOLANG_PATH",
            "info",
            "not set",
            "set to add custom import search paths",
        );
    }

    // Build as Table for nice display
    let columns = vec![
        "category".to_string(),
        "check".to_string(),
        "status".to_string(),
        "value".to_string(),
        "fix".to_string(),
    ];
    let mut rows = Vec::new();
    for check in &checks {
        if let Value::Record(rec) = check {
            let row: Vec<Value> = columns
                .iter()
                .map(|col| rec.get(col).cloned().unwrap_or(Value::Str(String::new())))
                .collect();
            rows.push(row);
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

/// config() → Record of .biolang/config.yaml settings
/// config(key) → value for that key
#[cfg(feature = "native")]
fn builtin_config(args: Vec<Value>) -> Result<Value> {
    use std::collections::HashMap;

    // Look for .biolang/config.yaml in cwd or parent dirs
    let mut dir = std::env::current_dir().unwrap_or_default();
    let config_content = loop {
        let config_path = dir.join(".biolang").join("config.yaml");
        if config_path.exists() {
            break std::fs::read_to_string(&config_path).ok();
        }
        if !dir.pop() {
            break None;
        }
    };

    let record = if let Some(content) = config_content {
        // Parse YAML via serde_json round-trip (we have serde_yaml as toml, use toml for config)
        // Actually parse as TOML or simple key: value lines
        let mut map = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim().to_string();
                let val = val.trim();
                let value = if val == "true" {
                    Value::Bool(true)
                } else if val == "false" {
                    Value::Bool(false)
                } else if let Ok(n) = val.parse::<i64>() {
                    Value::Int(n)
                } else if let Ok(f) = val.parse::<f64>() {
                    Value::Float(f)
                } else {
                    Value::Str(val.trim_matches('"').trim_matches('\'').to_string())
                };
                map.insert(key, value);
            }
        }
        Value::Record(map)
    } else {
        Value::Record(HashMap::new())
    };

    if args.is_empty() {
        Ok(record)
    } else {
        let key = match &args[0] {
            Value::Str(s) => s.clone(),
            other => {
                return Err(BioLangError::type_error(
                    format!("config() key must be Str, got {}", other.type_of()),
                    None,
                ))
            }
        };
        match record {
            Value::Record(map) => Ok(map.get(&key).cloned().unwrap_or(Value::Nil)),
            _ => Ok(Value::Nil),
        }
    }
}

fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(arr) => {
            Value::List(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let rec: std::collections::HashMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect();
            Value::Record(rec)
        }
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

/// Convert a Table's rows into a Vec of Record Values.
fn table_to_records(t: &Table) -> Vec<Value> {
    (0..t.rows.len())
        .map(|i| Value::Record(t.row_to_record(i)))
        .collect()
}

fn list_min(items: &[Value]) -> Result<Value> {
    if items.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "min() requires at least one argument",
            None,
        ));
    }
    let mut best = &items[0];
    for item in &items[1..] {
        if val_lt(item, best)? {
            best = item;
        }
    }
    Ok(best.clone())
}

fn list_max(items: &[Value]) -> Result<Value> {
    if items.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "max() requires at least one argument",
            None,
        ));
    }
    let mut best = &items[0];
    for item in &items[1..] {
        if val_lt(best, item)? {
            best = item;
        }
    }
    Ok(best.clone())
}

fn val_lt(a: &Value, b: &Value) -> Result<bool> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(a < b),
        (Value::Float(a), Value::Float(b)) => Ok(a < b),
        (Value::Int(a), Value::Float(b)) => Ok((*a as f64) < *b),
        (Value::Float(a), Value::Int(b)) => Ok(*a < (*b as f64)),
        (Value::Str(a), Value::Str(b)) => Ok(a < b),
        _ => Err(BioLangError::type_error(
            format!(
                "cannot compare {} and {}",
                a.type_of(),
                b.type_of()
            ),
            None,
        )),
    }
}

// ── Set helpers ──────────────────────────────────────────────────

fn require_two_sets(args: &[Value]) -> Result<(Vec<Value>, Vec<Value>)> {
    let a = match &args[0] {
        Value::Set(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("requires Set, got {}", other.type_of()),
                None,
            ))
        }
    };
    let b = match &args[1] {
        Value::Set(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("requires Set, got {}", other.type_of()),
                None,
            ))
        }
    };
    Ok((a, b))
}

// ── Genomic range queries (F13) ─────────────────────────────────

/// Build an interval tree from a Table with chrom, start, end columns.
/// Returns a Record with __type: "interval_tree" and per-chromosome sorted intervals.
fn builtin_interval_tree(args: &[Value]) -> Result<Value> {
    use std::collections::HashMap;

    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("interval_tree() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };

    // Find chrom, start, end column indices
    let chrom_idx = table.columns.iter().position(|c| c == "chrom" || c == "chr")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "interval_tree() requires 'chrom' column", None))?;
    let start_idx = table.columns.iter().position(|c| c == "start")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "interval_tree() requires 'start' column", None))?;
    let end_idx = table.columns.iter().position(|c| c == "end")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "interval_tree() requires 'end' column", None))?;

    // Group intervals by chromosome and sort by start
    let mut chroms: HashMap<String, Vec<(i64, i64, usize)>> = HashMap::new();
    for (row_idx, row) in table.rows.iter().enumerate() {
        let chrom = match &row[chrom_idx] {
            Value::Str(s) => s.clone(),
            other => format!("{other}"),
        };
        let start = match &row[start_idx] {
            Value::Int(n) => *n,
            _ => continue,
        };
        let end = match &row[end_idx] {
            Value::Int(n) => *n,
            _ => continue,
        };
        chroms.entry(chrom).or_default().push((start, end, row_idx));
    }

    // Sort each chromosome's intervals by start
    for intervals in chroms.values_mut() {
        intervals.sort_by_key(|(s, _, _)| *s);
    }

    // Store as Record with parallel arrays per chromosome for fast binary search.
    // { __type: "interval_tree", __table: Table, __chroms: {chr -> {starts: [...], ends: [...], indices: [...]}} }
    let mut tree = HashMap::new();
    tree.insert("__type".to_string(), Value::Str("interval_tree".to_string()));

    let mut chrom_map = HashMap::new();
    for (chr, intervals) in &chroms {
        let starts: Vec<Value> = intervals.iter().map(|(s, _, _)| Value::Int(*s)).collect();
        let ends: Vec<Value> = intervals.iter().map(|(_, e, _)| Value::Int(*e)).collect();
        let indices: Vec<Value> = intervals.iter().map(|(_, _, ri)| Value::Int(*ri as i64)).collect();
        let mut chr_rec = HashMap::new();
        chr_rec.insert("starts".to_string(), Value::List(starts));
        chr_rec.insert("ends".to_string(), Value::List(ends));
        chr_rec.insert("indices".to_string(), Value::List(indices));
        chrom_map.insert(chr.clone(), Value::Record(chr_rec));
    }
    tree.insert("__chroms".to_string(), Value::Record(chrom_map));
    tree.insert("__table".to_string(), Value::Table(table.clone()));
    Ok(Value::Record(tree))
}

/// Extract parallel arrays (starts, ends, indices) from a chromosome record in the interval tree.
fn extract_chr_arrays<'a>(chr_rec: &'a Value) -> Option<(&'a [Value], &'a [Value], &'a [Value])> {
    if let Value::Record(m) = chr_rec {
        let starts = match m.get("starts") { Some(Value::List(l)) => l.as_slice(), _ => return None };
        let ends = match m.get("ends") { Some(Value::List(l)) => l.as_slice(), _ => return None };
        let indices = match m.get("indices") { Some(Value::List(l)) => l.as_slice(), _ => return None };
        Some((starts, ends, indices))
    } else {
        None
    }
}

/// Extract tree components: table, chrom_data record.
fn extract_tree_parts(tree: &std::collections::HashMap<String, Value>) -> Result<(&Table, &std::collections::HashMap<String, Value>)> {
    let table = match tree.get("__table") {
        Some(Value::Table(t)) => t,
        _ => return Err(BioLangError::runtime(ErrorKind::TypeError, "invalid interval_tree", None)),
    };
    let chrom_data = match tree.get("__chroms") {
        Some(Value::Record(m)) => m,
        _ => return Err(BioLangError::runtime(ErrorKind::TypeError, "invalid interval_tree", None)),
    };
    Ok((&table, chrom_data))
}

/// Query overlapping intervals from an interval tree.
fn builtin_query_overlaps(args: &[Value]) -> Result<Value> {
    let tree = match &args[0] {
        Value::Record(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("query_overlaps() requires interval_tree Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let chrom = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => return Err(BioLangError::type_error(format!("query_overlaps() chrom requires Str, got {}", other.type_of()), None)),
    };
    let q_start = require_int(&args[2], "query_overlaps")?;
    let q_end = require_int(&args[3], "query_overlaps")?;

    let (table, chrom_data) = extract_tree_parts(tree)?;
    let chr_rec = match chrom_data.get(chrom) {
        Some(v) => v,
        _ => return Ok(Value::Table(Table::new(table.columns.clone(), vec![]))),
    };
    let (starts, ends, indices) = match extract_chr_arrays(chr_rec) {
        Some(v) => v,
        None => return Ok(Value::Table(Table::new(table.columns.clone(), vec![]))),
    };

    // Binary search: starts sorted. Find cutoff where start >= q_end.
    let cutoff = starts.partition_point(|v| {
        if let Value::Int(n) = v { *n < q_end } else { true }
    });
    let mut result_rows = Vec::new();
    for i in 0..cutoff {
        let end = match &ends[i] { Value::Int(n) => *n, _ => continue };
        if end > q_start {
            let row_idx = match &indices[i] { Value::Int(n) => *n as usize, _ => continue };
            if let Some(row) = table.rows.get(row_idx) {
                result_rows.push(row.clone());
            }
        }
    }
    Ok(Value::Table(Table::new(table.columns.clone(), result_rows)))
}

/// Count overlapping intervals without building result Table.
fn builtin_count_overlaps(args: &[Value]) -> Result<Value> {
    let tree = match &args[0] {
        Value::Record(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("count_overlaps() requires interval_tree Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let chrom = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => return Err(BioLangError::type_error(format!("count_overlaps() chrom requires Str, got {}", other.type_of()), None)),
    };
    let q_start = require_int(&args[2], "count_overlaps")?;
    let q_end = require_int(&args[3], "count_overlaps")?;

    let (_, chrom_data) = extract_tree_parts(tree)?;
    let chr_rec = match chrom_data.get(chrom) {
        Some(v) => v,
        _ => return Ok(Value::Int(0)),
    };
    let (starts, ends, _) = match extract_chr_arrays(chr_rec) {
        Some(v) => v,
        None => return Ok(Value::Int(0)),
    };

    let cutoff = starts.partition_point(|v| {
        if let Value::Int(n) = v { *n < q_end } else { true }
    });
    let mut count: i64 = 0;
    for i in 0..cutoff {
        if let Value::Int(end) = &ends[i] {
            if *end > q_start {
                count += 1;
            }
        }
    }
    Ok(Value::Int(count))
}

/// Bulk overlap counting: count total overlaps between an interval tree and a query Table.
/// Returns total overlap count as Int. Avoids per-query interpreter dispatch overhead.
fn builtin_bulk_overlaps(args: &[Value]) -> Result<Value> {
    let tree = match &args[0] {
        Value::Record(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("bulk_overlaps() requires interval_tree Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let queries = match &args[1] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("bulk_overlaps() requires Table as second argument, got {}", other.type_of()),
                None,
            ))
        }
    };

    let (_, chrom_data) = extract_tree_parts(tree)?;

    // Find chrom, start, end columns in queries table
    let q_chrom_idx = queries.columns.iter().position(|c| c == "chrom" || c == "chr")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "bulk_overlaps() query table requires 'chrom' column", None))?;
    let q_start_idx = queries.columns.iter().position(|c| c == "start")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "bulk_overlaps() query table requires 'start' column", None))?;
    let q_end_idx = queries.columns.iter().position(|c| c == "end")
        .ok_or_else(|| BioLangError::runtime(ErrorKind::TypeError, "bulk_overlaps() query table requires 'end' column", None))?;

    // Pre-extract native arrays per chromosome for zero-overhead inner loop
    let mut native_chroms: std::collections::HashMap<&str, (Vec<i64>, Vec<i64>)> = std::collections::HashMap::new();
    for (chr, chr_rec) in chrom_data {
        if let Some((starts, ends, _)) = extract_chr_arrays(chr_rec) {
            let s: Vec<i64> = starts.iter().filter_map(|v| if let Value::Int(n) = v { Some(*n) } else { None }).collect();
            let e: Vec<i64> = ends.iter().filter_map(|v| if let Value::Int(n) = v { Some(*n) } else { None }).collect();
            native_chroms.insert(chr.as_str(), (s, e));
        }
    }

    let mut total: i64 = 0;
    for row in &queries.rows {
        let chrom = match &row[q_chrom_idx] {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        let q_start = match &row[q_start_idx] { Value::Int(n) => *n, _ => continue };
        let q_end = match &row[q_end_idx] { Value::Int(n) => *n, _ => continue };

        if let Some((starts, ends)) = native_chroms.get(chrom) {
            // Binary search: find cutoff where start >= q_end
            let cutoff = starts.partition_point(|s| *s < q_end);
            for i in 0..cutoff {
                if ends[i] > q_start {
                    total += 1;
                }
            }
        }
    }
    Ok(Value::Int(total))
}

/// Query nearest intervals from an interval tree.
fn builtin_query_nearest(args: &[Value]) -> Result<Value> {
    let tree = match &args[0] {
        Value::Record(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("query_nearest() requires interval_tree Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let chrom = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => return Err(BioLangError::type_error(format!("query_nearest() chrom requires Str, got {}", other.type_of()), None)),
    };
    let pos = require_int(&args[2], "query_nearest")?;
    let k = if args.len() > 3 { require_int(&args[3], "query_nearest")? as usize } else { 1 };

    let (table, chrom_data) = extract_tree_parts(tree)?;
    let chr_rec = match chrom_data.get(chrom) {
        Some(v) => v,
        _ => return Ok(Value::Table(Table::new(table.columns.clone(), vec![]))),
    };
    let (starts, ends, indices) = match extract_chr_arrays(chr_rec) {
        Some(v) => v,
        None => return Ok(Value::Table(Table::new(table.columns.clone(), vec![]))),
    };

    // Binary search for insertion point, then expand outward to find k nearest.
    let insert_pt = starts.partition_point(|v| {
        if let Value::Int(n) = v { *n <= pos } else { true }
    });
    let mut scored: Vec<(i64, usize)> = Vec::new();
    let mut left = insert_pt.saturating_sub(1) as isize;
    let mut right = insert_pt;
    let n = starts.len();
    let scan_limit = n.min(k * 4 + 20);
    let mut scanned = 0usize;
    while scanned < scan_limit && (left >= 0 || right < n) {
        if left >= 0 {
            let li = left as usize;
            let start = match &starts[li] { Value::Int(n) => *n, _ => { left -= 1; scanned += 1; continue; } };
            let end = match &ends[li] { Value::Int(n) => *n, _ => { left -= 1; scanned += 1; continue; } };
            let row_idx = match &indices[li] { Value::Int(n) => *n as usize, _ => { left -= 1; scanned += 1; continue; } };
            let dist = if pos < start { start - pos } else if pos > end { pos - end } else { 0 };
            scored.push((dist, row_idx));
            left -= 1;
        }
        if right < n {
            let start = match &starts[right] { Value::Int(n) => *n, _ => { right += 1; scanned += 1; continue; } };
            let end = match &ends[right] { Value::Int(n) => *n, _ => { right += 1; scanned += 1; continue; } };
            let row_idx = match &indices[right] { Value::Int(n) => *n as usize, _ => { right += 1; scanned += 1; continue; } };
            let dist = if pos < start { start - pos } else if pos > end { pos - end } else { 0 };
            scored.push((dist, row_idx));
            right += 1;
        }
        scanned += 1;
    }
    scored.sort_by_key(|(d, _)| *d);
    let result_rows: Vec<Vec<Value>> = scored
        .into_iter()
        .take(k)
        .filter_map(|(_, ri)| table.rows.get(ri).cloned())
        .collect();
    Ok(Value::Table(Table::new(table.columns.clone(), result_rows)))
}

/// Calculate coverage depth from an interval tree.
fn builtin_coverage(args: &[Value]) -> Result<Value> {
    let tree = match &args[0] {
        Value::Record(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!("coverage() requires interval_tree Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let chrom_data = match tree.get("__chroms") {
        Some(Value::Record(m)) => m,
        _ => return Err(BioLangError::runtime(ErrorKind::TypeError, "invalid interval_tree", None)),
    };

    let mut rows = Vec::new();
    for (chr, chr_rec) in chrom_data {
        if let Some((starts, ends, _)) = extract_chr_arrays(chr_rec) {
            // Sweep line algorithm
            let mut events: Vec<(i64, i32)> = Vec::new();
            for i in 0..starts.len() {
                let start = match &starts[i] { Value::Int(n) => *n, _ => continue };
                let end = match &ends[i] { Value::Int(n) => *n, _ => continue };
                events.push((start, 1));
                events.push((end, -1));
            }
            events.sort_by_key(|(pos, delta)| (*pos, -*delta));

            let mut depth = 0i64;
            let mut prev_pos = 0i64;
            let mut started = false;
            for (pos, delta) in &events {
                if started && *pos != prev_pos && depth > 0 {
                    rows.push(vec![
                        Value::Str(chr.clone()),
                        Value::Int(prev_pos),
                        Value::Int(*pos),
                        Value::Int(depth),
                    ]);
                }
                depth += *delta as i64;
                prev_pos = *pos;
                started = true;
            }
        }
    }
    Ok(Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "depth".into()],
        rows,
    )))
}

// ── AlignedRead builtins ─────────────────────────────────────────

fn builtin_aligned_read(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Record(map) | Value::Map(map) => {
            let get_str = |m: &std::collections::HashMap<String, Value>, k: &str| -> String {
                m.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let get_int = |m: &std::collections::HashMap<String, Value>, k: &str| -> i64 {
                m.get(k).and_then(|v| v.as_int()).unwrap_or(0)
            };
            let flag = get_int(map, "flag");
            if flag < 0 || flag > 4095 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("aligned_read() flag must be 0-4095, got {flag}"),
                    None,
                ));
            }
            let seq = get_str(map, "seq");
            let qual = get_str(map, "qual");
            if !seq.is_empty() && !qual.is_empty() && seq.len() != qual.len() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("aligned_read() seq length ({}) must match qual length ({})", seq.len(), qual.len()),
                    None,
                ));
            }
            Ok(Value::AlignedRead(bl_core::bio_core::AlignedRead {
                qname: get_str(map, "qname"),
                flag: flag as u16,
                rname: get_str(map, "rname"),
                pos: get_int(map, "pos"),
                mapq: get_int(map, "mapq") as u8,
                cigar: get_str(map, "cigar"),
                rnext: get_str(map, "rnext"),
                pnext: get_int(map, "pnext"),
                tlen: get_int(map, "tlen"),
                seq,
                qual,
            }))
        }
        _ => Err(BioLangError::type_error("aligned_read() requires Record", None)),
    }
}

fn builtin_flagstat(args: &[Value]) -> Result<Value> {
    use std::collections::HashMap;
    let reads = match &args[0] {
        Value::List(items) => items,
        other => return Err(BioLangError::type_error(
            format!("flagstat() requires List of AlignedRead, got {}", other.type_of()),
            None,
        )),
    };
    let mut total = 0i64;
    let mut mapped = 0i64;
    let mut paired = 0i64;
    let mut proper_pair = 0i64;
    let mut duplicates = 0i64;
    let mut secondary = 0i64;
    let mut supplementary = 0i64;
    let mut read1 = 0i64;
    let mut read2 = 0i64;
    for item in reads {
        if let Value::AlignedRead(r) = item {
            total += 1;
            if r.is_mapped() { mapped += 1; }
            if r.is_paired() { paired += 1; }
            if r.is_proper_pair() { proper_pair += 1; }
            if r.is_duplicate() { duplicates += 1; }
            if r.is_secondary() { secondary += 1; }
            if r.is_supplementary() { supplementary += 1; }
            if r.is_read1() { read1 += 1; }
            if r.is_read2() { read2 += 1; }
        }
    }
    let mut rec = HashMap::new();
    rec.insert("total".to_string(), Value::Int(total));
    rec.insert("mapped".to_string(), Value::Int(mapped));
    rec.insert("paired".to_string(), Value::Int(paired));
    rec.insert("proper_pair".to_string(), Value::Int(proper_pair));
    rec.insert("duplicates".to_string(), Value::Int(duplicates));
    rec.insert("secondary".to_string(), Value::Int(secondary));
    rec.insert("supplementary".to_string(), Value::Int(supplementary));
    rec.insert("read1".to_string(), Value::Int(read1));
    rec.insert("read2".to_string(), Value::Int(read2));
    if total > 0 {
        rec.insert("mapped_pct".to_string(), Value::Float(mapped as f64 / total as f64 * 100.0));
        rec.insert("duplicate_pct".to_string(), Value::Float(duplicates as f64 / total as f64 * 100.0));
    }
    Ok(Value::Record(rec))
}

// ── Variant classification builtins ──────────────────────────────

fn extract_variant_fields(v: &Value) -> Option<(&str, &str)> {
    match v {
        Value::Variant { ref_allele, alt_allele, .. } => Some((ref_allele.as_str(), alt_allele.as_str())),
        _ => None,
    }
}

fn builtin_variant_predicate(args: &[Value], op: &str) -> Result<Value> {
    let (ref_allele, alt_allele) = match extract_variant_fields(&args[0]) {
        Some(pair) => pair,
        None => return Err(BioLangError::type_error(
            format!("{op}() requires Variant, got {}", args[0].type_of()),
            None,
        )),
    };
    let result = match op {
        "is_snp" => bl_core::bio_core::vcf_ops::classify_variant(ref_allele, alt_allele.split(',').next().unwrap_or("")) == bl_core::bio_core::VariantType::Snp,
        "is_indel" => bl_core::bio_core::vcf_ops::classify_variant(ref_allele, alt_allele.split(',').next().unwrap_or("")) == bl_core::bio_core::VariantType::Indel,
        "is_transition" => {
            let first_alt = alt_allele.split(',').next().unwrap_or("");
            bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Snp
                && bl_core::bio_core::vcf_ops::is_transition(ref_allele, first_alt)
        }
        "is_transversion" => {
            let first_alt = alt_allele.split(',').next().unwrap_or("");
            bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Snp
                && !bl_core::bio_core::vcf_ops::is_transition(ref_allele, first_alt)
        }
        "is_multiallelic" => alt_allele.contains(','),
        "is_het" | "is_hom_ref" | "is_hom_alt" => {
            // Genotype queries need the full Value
            if let Value::Variant { ref info, .. } = &args[0] {
                // Try to extract GT from info field
                let gt_str = info.get("GT").or_else(|| info.get("gt"));
                match gt_str {
                    Some(Value::Str(gt)) => {
                        let sep = if gt.contains('|') { '|' } else { '/' };
                        let alleles: Vec<Option<u8>> = gt.split(sep)
                            .map(|a| if a == "." { None } else { a.parse().ok() })
                            .collect();
                        match op {
                            "is_het" => {
                                let vals: Vec<u8> = alleles.iter().filter_map(|a| *a).collect();
                                vals.len() >= 2 && vals.windows(2).any(|w| w[0] != w[1])
                            }
                            "is_hom_ref" => alleles.iter().all(|a| *a == Some(0)),
                            "is_hom_alt" => {
                                let vals: Vec<u8> = alleles.iter().filter_map(|a| *a).collect();
                                vals.len() >= 2 && vals[0] > 0 && vals.iter().all(|&a| a == vals[0])
                            }
                            _ => false,
                        }
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    };
    Ok(Value::Bool(result))
}

fn builtin_variant_type(args: &[Value]) -> Result<Value> {
    let (ref_allele, alt_allele) = match extract_variant_fields(&args[0]) {
        Some(pair) => pair,
        None => return Err(BioLangError::type_error(
            format!("variant_type() requires Variant, got {}", args[0].type_of()),
            None,
        )),
    };
    let first_alt = alt_allele.split(',').next().unwrap_or("");
    let vt = bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt);
    let name = match vt {
        bl_core::bio_core::VariantType::Snp => "Snp",
        bl_core::bio_core::VariantType::Indel => "Indel",
        bl_core::bio_core::VariantType::Mnp => "Mnp",
        bl_core::bio_core::VariantType::Other => "Other",
    };
    Ok(Value::Str(name.to_string()))
}

fn builtin_variant_summary(args: &[Value]) -> Result<Value> {
    use std::collections::HashMap;
    let table = match &args[0] {
        Value::Table(t) => t,
        Value::List(items) => {
            // List of Variants
            let mut snp = 0u64;
            let mut indel = 0u64;
            let mut mnp = 0u64;
            let mut other = 0u64;
            let mut transitions = 0u64;
            let mut transversions = 0u64;
            let mut multiallelic = 0u64;
            for item in items {
                if let Value::Variant { ref_allele, alt_allele, .. } = item {
                    if alt_allele.contains(',') {
                        multiallelic += 1;
                    }
                    for alt in alt_allele.split(',') {
                        match bl_core::bio_core::vcf_ops::classify_variant(ref_allele, alt) {
                            bl_core::bio_core::VariantType::Snp => {
                                snp += 1;
                                if bl_core::bio_core::vcf_ops::is_transition(ref_allele, alt) {
                                    transitions += 1;
                                } else {
                                    transversions += 1;
                                }
                            }
                            bl_core::bio_core::VariantType::Indel => indel += 1,
                            bl_core::bio_core::VariantType::Mnp => mnp += 1,
                            bl_core::bio_core::VariantType::Other => other += 1,
                        }
                    }
                }
            }
            let ts_tv = if transversions > 0 { transitions as f64 / transversions as f64 } else { 0.0 };
            let mut rec = HashMap::new();
            rec.insert("total".to_string(), Value::Int((snp + indel + mnp + other) as i64));
            rec.insert("snp".to_string(), Value::Int(snp as i64));
            rec.insert("indel".to_string(), Value::Int(indel as i64));
            rec.insert("mnp".to_string(), Value::Int(mnp as i64));
            rec.insert("other".to_string(), Value::Int(other as i64));
            rec.insert("transitions".to_string(), Value::Int(transitions as i64));
            rec.insert("transversions".to_string(), Value::Int(transversions as i64));
            rec.insert("ts_tv_ratio".to_string(), Value::Float(ts_tv));
            rec.insert("multiallelic".to_string(), Value::Int(multiallelic as i64));
            return Ok(Value::Record(rec));
        }
        other => return Err(BioLangError::type_error(
            format!("variant_summary() requires Table or List of Variants, got {}", other.type_of()),
            None,
        )),
    };
    // Table path: find ref/alt columns
    let ref_idx = table.col_index("ref").or_else(|| table.col_index("ref_allele"));
    let alt_idx = table.col_index("alt").or_else(|| table.col_index("alt_allele"));
    let (ref_idx, alt_idx) = match (ref_idx, alt_idx) {
        (Some(r), Some(a)) => (r, a),
        _ => return Err(BioLangError::runtime(ErrorKind::TypeError, "variant_summary() requires 'ref' and 'alt' columns", None)),
    };
    let mut variants: Vec<(&str, Vec<&str>)> = Vec::new();
    for row in &table.rows {
        let ref_a = match &row[ref_idx] { Value::Str(s) => s.as_str(), _ => continue };
        let alt_a = match &row[alt_idx] { Value::Str(s) => s.as_str(), _ => continue };
        let alts: Vec<&str> = alt_a.split(',').collect();
        variants.push((ref_a, alts));
    }
    let pairs: Vec<(&str, &[&str])> = variants.iter().map(|(r, a)| (*r, a.as_slice())).collect();
    let summary = bl_core::bio_core::vcf_ops::summarize_variants(&pairs);
    let mut rec = HashMap::new();
    rec.insert("total".to_string(), Value::Int((summary.snp + summary.indel + summary.mnp + summary.other) as i64));
    rec.insert("snp".to_string(), Value::Int(summary.snp as i64));
    rec.insert("indel".to_string(), Value::Int(summary.indel as i64));
    rec.insert("mnp".to_string(), Value::Int(summary.mnp as i64));
    rec.insert("other".to_string(), Value::Int(summary.other as i64));
    rec.insert("transitions".to_string(), Value::Int(summary.transitions as i64));
    rec.insert("transversions".to_string(), Value::Int(summary.transversions as i64));
    rec.insert("ts_tv_ratio".to_string(), Value::Float(summary.ts_tv_ratio));
    rec.insert("multiallelic".to_string(), Value::Int(summary.multiallelic as i64));
    Ok(Value::Record(rec))
}

fn builtin_tstv_ratio(args: &[Value]) -> Result<Value> {
    let items = match &args[0] {
        Value::List(items) => items,
        other => return Err(BioLangError::type_error(
            format!("tstv_ratio() requires List of Variants, got {}", other.type_of()),
            None,
        )),
    };
    let mut transitions = 0u64;
    let mut transversions = 0u64;
    for item in items {
        if let Value::Variant { ref_allele, alt_allele, .. } = item {
            for alt in alt_allele.split(',') {
                if bl_core::bio_core::vcf_ops::classify_variant(ref_allele, alt) == bl_core::bio_core::VariantType::Snp {
                    if bl_core::bio_core::vcf_ops::is_transition(ref_allele, alt) {
                        transitions += 1;
                    } else {
                        transversions += 1;
                    }
                }
            }
        }
    }
    let ratio = if transversions > 0 { transitions as f64 / transversions as f64 } else { 0.0 };
    Ok(Value::Float(ratio))
}

fn builtin_het_hom_ratio(args: &[Value]) -> Result<Value> {
    let items = match &args[0] {
        Value::List(items) => items,
        other => return Err(BioLangError::type_error(
            format!("het_hom_ratio() requires List of Variants, got {}", other.type_of()),
            None,
        )),
    };
    let mut het = 0u64;
    let mut hom = 0u64;
    for item in items {
        if let Value::Variant { ref info, .. } = item {
            let gt_str = info.get("GT").or_else(|| info.get("gt"));
            if let Some(Value::Str(gt)) = gt_str {
                let sep = if gt.contains('|') { '|' } else { '/' };
                let alleles: Vec<Option<u8>> = gt.split(sep)
                    .map(|a| if a == "." { None } else { a.parse().ok() })
                    .collect();
                let vals: Vec<u8> = alleles.iter().filter_map(|a| *a).collect();
                if vals.len() >= 2 {
                    if vals.windows(2).any(|w| w[0] != w[1]) {
                        het += 1;
                    } else if vals[0] > 0 {
                        hom += 1;
                    }
                }
            }
        }
    }
    let ratio = if hom > 0 { het as f64 / hom as f64 } else { 0.0 };
    Ok(Value::Float(ratio))
}

fn builtin_parse_vcf_info(args: &[Value]) -> Result<Value> {
    use std::collections::HashMap;
    let info_str = match &args[0] {
        Value::Str(s) => s.as_str(),
        other => return Err(BioLangError::type_error(
            format!("parse_vcf_info() requires Str, got {}", other.type_of()),
            None,
        )),
    };
    if info_str == "." || info_str.is_empty() {
        return Ok(Value::Record(HashMap::new()));
    }
    let mut rec = HashMap::new();
    for part in info_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((key, val)) = part.split_once('=') {
            if val.contains(',') {
                // Multi-value: parse each element
                let items: Vec<Value> = val.split(',').map(|v| {
                    if let Ok(n) = v.parse::<i64>() { Value::Int(n) }
                    else if let Ok(f) = v.parse::<f64>() { Value::Float(f) }
                    else { Value::Str(v.to_string()) }
                }).collect();
                rec.insert(key.to_string(), Value::List(items));
            } else if let Ok(n) = val.parse::<i64>() {
                rec.insert(key.to_string(), Value::Int(n));
            } else if let Ok(f) = val.parse::<f64>() {
                rec.insert(key.to_string(), Value::Float(f));
            } else {
                rec.insert(key.to_string(), Value::Str(val.to_string()));
            }
        } else {
            // Flag field (e.g., "PASS")
            rec.insert(part.to_string(), Value::Bool(true));
        }
    }
    Ok(Value::Record(rec))
}

// ── Sequence pattern matching (F14) ─────────────────────────────

/// Convert IUPAC ambiguity code to regex character class.
fn iupac_to_regex(c: char) -> &'static str {
    match c.to_ascii_uppercase() {
        'A' => "A", 'T' => "T", 'C' => "C", 'G' => "G", 'U' => "U",
        'R' => "[AG]", 'Y' => "[CT]", 'W' => "[AT]", 'S' => "[GC]",
        'M' => "[AC]", 'K' => "[GT]", 'B' => "[CGT]", 'D' => "[AGT]",
        'H' => "[ACT]", 'V' => "[ACG]", 'N' => "[ACGT]",
        _ => ".",
    }
}

fn iupac_pattern_to_regex(pattern: &str) -> String {
    pattern.chars().map(|c| iupac_to_regex(c).to_string()).collect()
}

fn builtin_motif_find(args: &[Value]) -> Result<Value> {
    use std::collections::HashMap;

    let seq = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => &s.data,
        Value::Str(s) => s,
        other => return Err(BioLangError::type_error(format!("motif_find() requires DNA/RNA/Str, got {}", other.type_of()), None)),
    };
    let pattern = match &args[1] {
        Value::Str(s) => s.clone(),
        other => return Err(BioLangError::type_error(format!("motif_find() pattern requires Str, got {}", other.type_of()), None)),
    };

    let regex_pat = iupac_pattern_to_regex(&pattern);
    let re = regex::Regex::new(&format!("(?i){regex_pat}")).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("motif_find() invalid pattern: {e}"), None)
    })?;

    let results: Vec<Value> = re.find_iter(seq).map(|m| {
        let mut rec = HashMap::new();
        rec.insert("start".to_string(), Value::Int(m.start() as i64));
        rec.insert("end".to_string(), Value::Int(m.end() as i64));
        rec.insert("match".to_string(), Value::Str(m.as_str().to_string()));
        Value::Record(rec)
    }).collect();
    Ok(Value::List(results))
}

fn builtin_motif_count(args: &[Value]) -> Result<Value> {
    let seq = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => &s.data,
        Value::Str(s) => s,
        other => return Err(BioLangError::type_error(format!("motif_count() requires DNA/RNA/Str, got {}", other.type_of()), None)),
    };
    let pattern = match &args[1] {
        Value::Str(s) => s.clone(),
        other => return Err(BioLangError::type_error(format!("motif_count() pattern requires Str, got {}", other.type_of()), None)),
    };

    let regex_pat = iupac_pattern_to_regex(&pattern);
    let re = regex::Regex::new(&format!("(?i){regex_pat}")).map_err(|e| {
        BioLangError::runtime(ErrorKind::TypeError, format!("motif_count() invalid pattern: {e}"), None)
    })?;

    Ok(Value::Int(re.find_iter(seq).count() as i64))
}

fn builtin_consensus(args: &[Value]) -> Result<Value> {
    let sequences: Vec<&str> = match &args[0] {
        Value::List(items) => {
            items.iter().map(|v| match v {
                Value::DNA(s) | Value::RNA(s) => Ok(s.data.as_str()),
                Value::Str(s) => Ok(s.as_str()),
                other => Err(BioLangError::type_error(format!("consensus() requires List of DNA/RNA/Str, got {}", other.type_of()), None)),
            }).collect::<Result<Vec<_>>>()?
        }
        other => return Err(BioLangError::type_error(format!("consensus() requires List, got {}", other.type_of()), None)),
    };

    if sequences.is_empty() {
        return Ok(Value::Str(String::new()));
    }

    let max_len = sequences.iter().map(|s| s.len()).max().unwrap_or(0);
    let mut result = String::with_capacity(max_len);

    for pos in 0..max_len {
        let mut counts = [0u32; 4]; // A, C, G, T
        for seq in &sequences {
            if let Some(base) = seq.as_bytes().get(pos) {
                match base.to_ascii_uppercase() {
                    b'A' => counts[0] += 1,
                    b'C' => counts[1] += 1,
                    b'G' => counts[2] += 1,
                    b'T' | b'U' => counts[3] += 1,
                    _ => {}
                }
            }
        }
        let max_idx = counts.iter().enumerate().max_by_key(|(_, &c)| c).map(|(i, _)| i).unwrap_or(0);
        result.push(['A', 'C', 'G', 'T'][max_idx]);
    }
    Ok(Value::Str(result))
}

fn builtin_pwm(args: &[Value]) -> Result<Value> {
    let sequences: Vec<&str> = match &args[0] {
        Value::List(items) => {
            items.iter().map(|v| match v {
                Value::DNA(s) | Value::RNA(s) => Ok(s.data.as_str()),
                Value::Str(s) => Ok(s.as_str()),
                other => Err(BioLangError::type_error(format!("pwm() requires List of DNA/RNA/Str, got {}", other.type_of()), None)),
            }).collect::<Result<Vec<_>>>()?
        }
        other => return Err(BioLangError::type_error(format!("pwm() requires List, got {}", other.type_of()), None)),
    };

    if sequences.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let max_len = sequences.iter().map(|s| s.len()).max().unwrap_or(0);
    let n = sequences.len() as f64;
    let mut matrix = Vec::with_capacity(max_len);

    for pos in 0..max_len {
        let mut counts = [0.0f64; 4]; // A, C, G, T
        for seq in &sequences {
            if let Some(base) = seq.as_bytes().get(pos) {
                match base.to_ascii_uppercase() {
                    b'A' => counts[0] += 1.0,
                    b'C' => counts[1] += 1.0,
                    b'G' => counts[2] += 1.0,
                    b'T' | b'U' => counts[3] += 1.0,
                    _ => {}
                }
            }
        }
        let mut rec = std::collections::HashMap::new();
        rec.insert("A".to_string(), Value::Float(counts[0] / n));
        rec.insert("C".to_string(), Value::Float(counts[1] / n));
        rec.insert("G".to_string(), Value::Float(counts[2] / n));
        rec.insert("T".to_string(), Value::Float(counts[3] / n));
        matrix.push(Value::Record(rec));
    }
    Ok(Value::List(matrix))
}

fn builtin_pwm_scan(args: &[Value]) -> Result<Value> {
    let seq = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => &s.data,
        Value::Str(s) => s,
        other => return Err(BioLangError::type_error(format!("pwm_scan() requires DNA/RNA/Str, got {}", other.type_of()), None)),
    };
    let pwm = match &args[1] {
        Value::List(l) => l,
        other => return Err(BioLangError::type_error(format!("pwm_scan() requires List (PWM), got {}", other.type_of()), None)),
    };
    let threshold = if args.len() > 2 {
        match &args[2] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 0.0,
        }
    } else {
        0.0
    };

    let pwm_len = pwm.len();
    if pwm_len == 0 || seq.len() < pwm_len {
        return Ok(Value::List(vec![]));
    }

    let mut hits = Vec::new();
    let seq_bytes = seq.as_bytes();
    for i in 0..=(seq.len() - pwm_len) {
        let mut score = 0.0f64;
        for (j, pos_rec) in pwm.iter().enumerate() {
            if let Value::Record(rec) = pos_rec {
                let base = (seq_bytes[i + j] as char).to_ascii_uppercase().to_string();
                if let Some(Value::Float(f)) = rec.get(&base) {
                    // Log-odds score: log2(freq / 0.25)
                    let freq = f.max(0.001); // pseudocount
                    score += (freq / 0.25).log2();
                }
            }
        }
        if score >= threshold {
            let mut rec = std::collections::HashMap::new();
            rec.insert("pos".to_string(), Value::Int(i as i64));
            rec.insert("score".to_string(), Value::Float(score));
            hits.push(Value::Record(rec));
        }
    }
    Ok(Value::List(hits))
}

// ── Pipeline steps (F15) ────────────────────────────────────────

fn builtin_pipeline_steps(args: &[Value]) -> Result<Value> {
    // Look up pipeline definition from the value
    match &args[0] {
        Value::Record(m) => {
            // If it's a pipeline record with steps
            if let Some(Value::List(steps)) = m.get("steps") {
                let mut rows = Vec::new();
                for (i, step) in steps.iter().enumerate() {
                    if let Value::Record(s) = step {
                        let name = s.get("name").map(|v| format!("{v}")).unwrap_or_else(|| format!("step_{i}"));
                        let plugin = s.get("plugin").map(|v| format!("{v}")).unwrap_or_default();
                        let params = s.get("params").map(|v| format!("{v}")).unwrap_or_default();
                        let depends = s.get("depends_on").map(|v| format!("{v}")).unwrap_or_default();
                        rows.push(vec![
                            Value::Int(i as i64),
                            Value::Str(name),
                            Value::Str(plugin),
                            Value::Str(params),
                            Value::Str(depends),
                        ]);
                    }
                }
                Ok(Value::Table(Table::new(
                    vec!["step".into(), "name".into(), "plugin".into(), "params".into(), "depends_on".into()],
                    rows,
                )))
            } else {
                Ok(Value::Table(Table::new(
                    vec!["step".into(), "name".into(), "plugin".into(), "params".into(), "depends_on".into()],
                    vec![],
                )))
            }
        }
        other => Err(BioLangError::type_error(
            format!("pipeline_steps() requires Record (pipeline), got {}", other.type_of()),
            None,
        )),
    }
}

// ── GAP 1: Coordinate system builtins ───────────────────────────

fn builtin_coord_tag(args: Vec<Value>, system: &str) -> Result<Value> {
    use std::collections::HashMap;
    match args.into_iter().next().unwrap() {
        Value::Record(mut map) => {
            map.insert("__coord_system".to_string(), Value::Str(system.to_string()));
            Ok(Value::Record(map))
        }
        Value::Interval(iv) => {
            let mut map = HashMap::new();
            map.insert("chrom".to_string(), Value::Str(iv.chrom.clone()));
            map.insert("start".to_string(), Value::Int(iv.start));
            map.insert("end".to_string(), Value::Int(iv.end));
            map.insert("strand".to_string(), Value::Str(format!("{}", iv.strand)));
            map.insert("__coord_system".to_string(), Value::Str(system.to_string()));
            Ok(Value::Record(map))
        }
        other => Err(BioLangError::type_error(
            format!("coord_{}() requires Record or Interval, got {}", system, other.type_of()),
            None,
        )),
    }
}

fn builtin_coord_convert(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::CoordSystem;
    let mut rec = match &args[0] {
        Value::Record(map) => map.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("coord_convert() requires Record, got {}", other.type_of()),
                None,
            ))
        }
    };
    let to_system = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("coord_convert() target must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    let from = rec
        .get("__coord_system")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    let from_sys = CoordSystem::from_str_lossy(&from);
    let to_sys = CoordSystem::from_str_lossy(to_system);

    let start = rec.get("start").and_then(|v| v.as_int()).unwrap_or(0);
    let end = rec.get("end").and_then(|v| v.as_int()).unwrap_or(0);

    let (new_start, new_end) = CoordSystem::convert(from_sys, to_sys, start, end);
    rec.insert("start".to_string(), Value::Int(new_start));
    rec.insert("end".to_string(), Value::Int(new_end));
    rec.insert("__coord_system".to_string(), Value::Str(to_system.to_string()));
    Ok(Value::Record(rec))
}

fn builtin_coord_system(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Record(map) => {
            let sys = map
                .get("__coord_system")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            Ok(Value::Str(sys))
        }
        _ => Ok(Value::Str("unknown".to_string())),
    }
}

fn builtin_coord_check(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::CoordSystem;
    let sys_a = match &args[0] {
        Value::Record(map) => map
            .get("__coord_system")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };
    let sys_b = match &args[1] {
        Value::Record(map) => map
            .get("__coord_system")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };
    let a = CoordSystem::from_str_lossy(&sys_a);
    let b = CoordSystem::from_str_lossy(&sys_b);
    Ok(Value::Bool(a.is_compatible(&b)))
}

// ── Chromosome name normalization ────────────────────────────────

/// strip_chr("chr1") → "1", strip_chr("chrX") → "X", strip_chr("1") → "1"
fn builtin_strip_chr(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.as_str(),
        Value::Interval(iv) => {
            let mut iv = iv.clone();
            iv.chrom = iv.chrom.strip_prefix("chr").unwrap_or(&iv.chrom).to_string();
            return Ok(Value::Interval(iv));
        }
        other => {
            return Err(BioLangError::type_error(
                format!("strip_chr() requires Str or Interval, got {}", other.type_of()),
                None,
            ))
        }
    };
    let stripped = name.strip_prefix("chr").unwrap_or(name);
    Ok(Value::Str(stripped.to_string()))
}

/// add_chr("1") → "chr1", add_chr("X") → "chrX", add_chr("chr1") → "chr1" (idempotent)
fn builtin_add_chr(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.as_str(),
        Value::Interval(iv) => {
            let mut iv = iv.clone();
            if !iv.chrom.starts_with("chr") {
                iv.chrom = format!("chr{}", iv.chrom);
            }
            return Ok(Value::Interval(iv));
        }
        other => {
            return Err(BioLangError::type_error(
                format!("add_chr() requires Str or Interval, got {}", other.type_of()),
                None,
            ))
        }
    };
    if name.starts_with("chr") {
        Ok(Value::Str(name.to_string()))
    } else {
        Ok(Value::Str(format!("chr{name}")))
    }
}

/// normalize_chrom("chr1") → "chr1", normalize_chrom("Chr1") → "chr1",
/// normalize_chrom("CHR1") → "chr1", normalize_chrom("MT") → "chrM",
/// normalize_chrom("chrMT") → "chrM"
fn builtin_normalize_chrom(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.clone(),
        Value::Interval(iv) => {
            let mut iv = iv.clone();
            iv.chrom = normalize_chrom_str(&iv.chrom);
            return Ok(Value::Interval(iv));
        }
        other => {
            return Err(BioLangError::type_error(
                format!("normalize_chrom() requires Str or Interval, got {}", other.type_of()),
                None,
            ))
        }
    };
    Ok(Value::Str(normalize_chrom_str(&name)))
}

fn normalize_chrom_str(name: &str) -> String {
    // Strip any "chr"/"Chr"/"CHR" prefix, then re-add lowercase "chr"
    let bare = if name.len() > 3 && name[..3].eq_ignore_ascii_case("chr") {
        &name[3..]
    } else {
        name
    };
    // Normalize common aliases
    let canonical = match bare {
        "MT" | "Mt" | "mt" => "M",
        _ => bare,
    };
    format!("chr{canonical}")
}

// ── GAP 2: K-mer builtins ───────────────────────────────────────

fn builtin_kmer_encode(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::Kmer;
    let seq = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => s.data.clone(),
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_encode() requires DNA/RNA/Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let k = match &args[1] {
        Value::Int(n) => *n as u8,
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_encode() k must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    if seq.len() == k as usize {
        // Single k-mer
        match Kmer::from_str(&seq, k) {
            Some(km) => Ok(Value::Kmer(km)),
            None => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                &format!("kmer_encode(): invalid sequence for k={k}"),
                None,
            )),
        }
    } else {
        // Extract all k-mers
        let kmers = Kmer::extract_all(&seq, k);
        Ok(Value::List(kmers.into_iter().map(Value::Kmer).collect()))
    }
}

fn builtin_kmer_decode(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Kmer(km) => Ok(Value::Str(km.decode())),
        other => Err(BioLangError::type_error(
            format!("kmer_decode() requires Kmer, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_kmer_rc(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Kmer(km) => Ok(Value::Kmer(km.reverse_complement())),
        other => Err(BioLangError::type_error(
            format!("kmer_rc() requires Kmer, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_kmer_canonical(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Kmer(km) => Ok(Value::Kmer(km.canonical())),
        other => Err(BioLangError::type_error(
            format!("kmer_canonical() requires Kmer, got {}", other.type_of()),
            None,
        )),
    }
}

fn extract_seq_str(val: &Value) -> Option<String> {
    match val {
        Value::DNA(s) | Value::RNA(s) => Some(s.data.clone()),
        Value::Str(s) => Some(s.clone()),
        Value::Record(map) => {
            // Extract "seq" field from record (FASTQ stream records)
            map.get("seq").and_then(|v| extract_seq_str(v))
        }
        _ => None,
    }
}

/// Maximum k-mer entries to keep in memory before spilling to disk.
/// ~2M entries ≈ 300MB with 21-mers. Beyond this, auto-spill to SQLite temp DB.
const KMER_SPILL_THRESHOLD: usize = 2_000_000;
/// Size of the write-back buffer after spilling.
/// Accumulate this many entries in memory before flushing to SQLite.
const KMER_FLUSH_BUFFER: usize = 500_000;
/// Minimum free disk space (in bytes) required before spilling to disk.
/// If less than this is available, fall back to top-N pruning instead.
/// 500 MB — a conservative estimate for large k-mer DBs.
const KMER_MIN_DISK_BYTES: u64 = 500 * 1024 * 1024;
/// Default top-N to use when disk space is insufficient for spilling.
const KMER_DISK_FALLBACK_TOP_N: usize = 100_000;

fn builtin_kmer_count(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::Kmer;
    let k = match &args[1] {
        Value::Int(n) => *n as u8,
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_count() k must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };

    // Optional 3rd arg: top N (bounded memory mode)
    let top_n: Option<usize> = if args.len() >= 3 {
        match &args[2] {
            Value::Int(n) if *n > 0 => Some(*n as usize),
            _ => None,
        }
    } else {
        None
    };

    // Stream mode: iterate without collecting into memory — fast u64 path
    if let Value::Stream(stream) = &args[0] {
        let mut fast_counts: rustc_hash::FxHashMap<u64, i64> = rustc_hash::FxHashMap::default();
        let mut seq_count = 0usize;
        loop {
            let item = match stream.next() {
                Some(v) => v,
                None => break,
            };
            if let Some(seq) = extract_seq_str(&item) {
                Kmer::count_into(&seq, k, &mut fast_counts);
            }
            seq_count += 1;
            if seq_count >= 1000 && seq_count % 10000 == 0 {
                eprint!("\r\x1b[2Kkmer_count: {} sequences, {} unique k-mers...",
                    seq_count, fast_counts.len());
            }
        }
        if seq_count >= 1000 {
            eprint!("\r\x1b[2K");
        }
        return fast_counts_to_value(fast_counts, k, top_n);
    }

    // Non-stream: collect sequences first
    let seqs: Vec<String> = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => vec![s.data.clone()],
        Value::Str(s) => vec![s.clone()],
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match extract_seq_str(item) {
                    Some(s) => out.push(s),
                    None => {
                        return Err(BioLangError::type_error(
                            format!("kmer_count() list items must be DNA/RNA/Str/Record, got {}", item.type_of()),
                            None,
                        ))
                    }
                }
            }
            out
        }
        Value::Table(tbl) => {
            let mut out = Vec::new();
            if let Some(col_idx) = tbl.columns.iter().position(|c| c == "seq") {
                for row in &tbl.rows {
                    if col_idx < row.len() {
                        if let Some(s) = extract_seq_str(&row[col_idx]) {
                            out.push(s);
                        }
                    }
                }
            } else {
                return Err(BioLangError::type_error(
                    "kmer_count() table must have a 'seq' column",
                    None,
                ));
            }
            out
        }
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_count() requires DNA/RNA/Str/List/Table/Stream, got {}", other.type_of()),
                None,
            ))
        }
    };

    // Fast path: count directly into FxHashMap<u64, i64> (no String allocation per kmer)
    let total = seqs.len();
    let show_progress = total >= 1000;
    let mut fast_counts: rustc_hash::FxHashMap<u64, i64> = rustc_hash::FxHashMap::default();
    for (i, seq) in seqs.iter().enumerate() {
        if show_progress && (i % 10000 == 0 || i == total - 1) {
            eprint!("\r\x1b[2Kkmer_count: {}/{} sequences ({:.0}%), {} unique k-mers...",
                i + 1, total, (i + 1) as f64 / total as f64 * 100.0,
                fast_counts.len());
        }
        Kmer::count_into(seq, k, &mut fast_counts);
    }
    if show_progress {
        eprint!("\r\x1b[2K");
    }

    // Convert u64 keys to String only for the final output
    fast_counts_to_value(fast_counts, k, top_n)
}

fn builtin_kmer_distinct(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::Kmer;
    let k = match &args[1] {
        Value::Int(n) => *n as u8,
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_distinct() k must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };

    // Stream mode
    if let Value::Stream(stream) = &args[0] {
        let mut fast_counts: rustc_hash::FxHashMap<u64, i64> = rustc_hash::FxHashMap::default();
        let mut seq_count = 0usize;
        loop {
            let item = match stream.next() {
                Some(v) => v,
                None => break,
            };
            if let Some(seq) = extract_seq_str(&item) {
                Kmer::count_into(&seq, k, &mut fast_counts);
            }
            seq_count += 1;
            if seq_count >= 1000 && seq_count % 10000 == 0 {
                eprint!("\r\x1b[2Kkmer_distinct: {} sequences, {} unique k-mers...",
                    seq_count, fast_counts.len());
            }
        }
        if seq_count >= 1000 {
            eprint!("\r\x1b[2K");
        }
        return Ok(Value::Int(fast_counts.len() as i64));
    }

    // Non-stream: collect sequences first
    let seqs: Vec<String> = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => vec![s.data.clone()],
        Value::Str(s) => vec![s.clone()],
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match extract_seq_str(item) {
                    Some(s) => out.push(s),
                    None => {
                        return Err(BioLangError::type_error(
                            format!("kmer_distinct() list items must be DNA/RNA/Str/Record, got {}", item.type_of()),
                            None,
                        ))
                    }
                }
            }
            out
        }
        Value::Table(tbl) => {
            let mut out = Vec::new();
            if let Some(col_idx) = tbl.columns.iter().position(|c| c == "seq") {
                for row in &tbl.rows {
                    if col_idx < row.len() {
                        if let Some(s) = extract_seq_str(&row[col_idx]) {
                            out.push(s);
                        }
                    }
                }
            } else {
                return Err(BioLangError::type_error(
                    "kmer_distinct() table must have a 'seq' column",
                    None,
                ));
            }
            out
        }
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_distinct() requires DNA/RNA/Str/List/Table/Stream, got {}", other.type_of()),
                None,
            ))
        }
    };

    let total = seqs.len();
    let show_progress = total >= 1000;
    let mut fast_counts: rustc_hash::FxHashMap<u64, i64> = rustc_hash::FxHashMap::default();
    for (i, seq) in seqs.iter().enumerate() {
        if show_progress && (i % 10000 == 0 || i == total - 1) {
            eprint!("\r\x1b[2Kkmer_distinct: {}/{} sequences ({:.0}%)...",
                i + 1, total, (i + 1) as f64 / total as f64 * 100.0);
        }
        Kmer::count_into(seq, k, &mut fast_counts);
    }
    if show_progress {
        eprint!("\r\x1b[2K");
    }
    Ok(Value::Int(fast_counts.len() as i64))
}

// ── K-mer Counter with auto disk spill ─────────────────────────────

/// Two-tier k-mer counter: starts in-memory, auto-spills to SQLite temp DB
/// when memory threshold is exceeded. Transparent to callers.
struct KmerCounter {
    /// In-memory HashMap — u64 encoded keys for speed, decode only at output
    mem: Option<std::collections::HashMap<String, i64>>,
    /// Fast integer-keyed counter for direct counting path
    fast_mem: Option<std::collections::HashMap<u64, i64>>,
    /// k value for decoding at the end
    k: u8,
    /// SQLite temp DB — activated after spill
    #[cfg(feature = "native")]
    db: Option<KmerDiskStore>,
    /// Total unique k-mers (tracked across both tiers)
    unique_count: usize,
    /// Whether we've spilled to disk
    spilled: bool,
    /// Top-N mode: if set, use bounded memory with pruning instead of disk spill
    top_n: Option<usize>,
}

impl KmerCounter {
    fn new(top_n: Option<usize>) -> Self {
        Self {
            mem: Some(std::collections::HashMap::new()),
            fast_mem: None,
            k: 0,
            #[cfg(feature = "native")]
            db: None,
            unique_count: 0,
            spilled: false,
            top_n,
        }
    }

    fn new_fast(k: u8, top_n: Option<usize>) -> Self {
        Self {
            mem: None,
            fast_mem: Some(std::collections::HashMap::new()),
            k,
            #[cfg(feature = "native")]
            db: None,
            unique_count: 0,
            spilled: false,
            top_n,
        }
    }

    /// Get mutable ref to fast counter
    fn fast_counter(&mut self) -> &mut std::collections::HashMap<u64, i64> {
        self.fast_mem.as_mut().unwrap()
    }

    fn len(&self) -> usize {
        self.unique_count
    }

    fn mode_label(&self) -> &'static str {
        if self.spilled {
            " [disk]"
        } else if self.top_n.is_some() {
            " [top-N]"
        } else {
            ""
        }
    }

    /// Try to create a SQLite temp DB and flush the in-memory map to it.
    /// Returns the DB on success, or a human-readable reason on failure.
    #[cfg(feature = "native")]
    fn try_spill_to_disk(mem: &mut std::collections::HashMap<String, i64>) -> std::result::Result<KmerDiskStore, String> {
        // Check available disk space before creating DB
        let tmp = std::env::temp_dir();
        match check_disk_space(&tmp) {
            Some(avail) if avail < KMER_MIN_DISK_BYTES => {
                return Err(format!(
                    "only {:.0} MB free in temp dir, need at least {:.0} MB",
                    avail as f64 / (1024.0 * 1024.0),
                    KMER_MIN_DISK_BYTES as f64 / (1024.0 * 1024.0),
                ));
            }
            _ => {} // Unknown or sufficient — proceed
        }

        let mut db = KmerDiskStore::new().map_err(|e| format!("{e}"))?;
        db.flush_map(mem).map_err(|e| format!("{e}"))?;
        Ok(db)
    }

    fn add_batch(&mut self, counts: &std::collections::HashMap<bl_core::bio_core::Kmer, u64>) -> Result<()> {
        // Top-N mode: use in-memory with pruning
        if let Some(top_n) = self.top_n {
            let mem = self.mem.as_mut().unwrap();
            for (km, cnt) in counts {
                *mem.entry(km.decode()).or_insert(0) += *cnt as i64;
            }
            // Prune: when HashMap grows to 10x top_n, keep only entries with
            // count above the median. This is approximate but bounded memory.
            let prune_at = std::cmp::max(top_n * 10, 100_000);
            if mem.len() > prune_at {
                let mut counts_vec: Vec<i64> = mem.values().copied().collect();
                counts_vec.sort_unstable();
                // Keep entries above the median count
                let cutoff = counts_vec[counts_vec.len() / 2];
                mem.retain(|_, v| *v > cutoff);
                self.unique_count = mem.len();
            } else {
                self.unique_count = mem.len();
            }
            return Ok(());
        }

        // Normal mode: in-memory with auto-spill to disk
        #[cfg(feature = "native")]
        {
            // Always accumulate into in-memory buffer first
            let mem = self.mem.as_mut().unwrap();
            for (km, cnt) in counts {
                *mem.entry(km.decode()).or_insert(0) += *cnt as i64;
            }

            if self.spilled {
                // Write-back buffer mode: flush to SQLite when buffer is large enough
                if mem.len() >= KMER_FLUSH_BUFFER {
                    let db = self.db.as_mut().unwrap();
                    if let Err(e) = db.flush_map(mem) {
                        // Disk full mid-operation — switch to top-N pruning
                        eprintln!("\x1b[33mWarning:\x1b[0m disk write failed ({e}), switching to top-{KMER_DISK_FALLBACK_TOP_N} pruning mode.");
                        // Drop the DB (cleans up temp file)
                        self.db = None;
                        self.spilled = false;
                        self.top_n = Some(KMER_DISK_FALLBACK_TOP_N);
                        // Prune to manageable size
                        let prune_at = KMER_DISK_FALLBACK_TOP_N * 10;
                        if mem.len() > prune_at {
                            let mut counts_vec: Vec<i64> = mem.values().copied().collect();
                            counts_vec.sort_unstable();
                            let cutoff = counts_vec[counts_vec.len() / 2];
                            mem.retain(|_, v| *v > cutoff);
                        }
                        self.unique_count = mem.len();
                        return Ok(());
                    }
                }
                // unique_count is approximate in buffered mode — that's fine for progress display
                self.unique_count += counts.len();
                return Ok(());
            }

            self.unique_count = mem.len();

            // Check threshold — spill to disk if exceeded
            if mem.len() > KMER_SPILL_THRESHOLD {
                eprintln!("\r\x1b[2K\x1b[33mNote:\x1b[0m {} unique k-mers exceed memory threshold — switching to disk-backed counting...", mem.len());
                match Self::try_spill_to_disk(mem) {
                    Ok(db) => {
                        self.db = Some(db);
                        self.spilled = true;
                    }
                    Err(reason) => {
                        // Disk spill failed — fall back to top-N pruning
                        eprintln!("\x1b[33mWarning:\x1b[0m disk spill failed ({reason}), falling back to top-{KMER_DISK_FALLBACK_TOP_N} pruning mode.");
                        eprintln!("  Results will be approximate — only the highest-count k-mers are retained.");
                        self.top_n = Some(KMER_DISK_FALLBACK_TOP_N);
                        // Immediately prune the current map
                        let prune_at = KMER_DISK_FALLBACK_TOP_N * 10;
                        if mem.len() > prune_at {
                            let mut counts_vec: Vec<i64> = mem.values().copied().collect();
                            counts_vec.sort_unstable();
                            let cutoff = counts_vec[counts_vec.len() / 2];
                            mem.retain(|_, v| *v > cutoff);
                        }
                        self.unique_count = mem.len();
                    }
                }
            }
        }

        #[cfg(not(feature = "native"))]
        {
            let mem = self.mem.as_mut().unwrap();
            for (km, cnt) in counts {
                *mem.entry(km.decode()).or_insert(0) += *cnt as i64;
            }
            self.unique_count = mem.len();
        }

        Ok(())
    }

    fn into_value(mut self, top_n: Option<usize>) -> Result<Value> {
        #[cfg(feature = "native")]
        if self.spilled {
            // Flush any remaining in-memory buffer to disk
            if let Some(ref mut mem) = self.mem {
                if !mem.is_empty() {
                    let db = self.db.as_mut().unwrap();
                    db.flush_map(mem)?;
                }
            }
            let limit = top_n.unwrap_or(usize::MAX);
            let db = self.db.as_ref().unwrap();
            // For small top-N, materialize into Table directly
            if limit <= 50_000 {
                return db.to_table(limit);
                // KmerDiskStore dropped → temp file cleaned up
            }
            // Large result: query sorted rows as compact Vec<(String, i64)>,
            // drop DB immediately (cleans up temp file), then stream from Vec.
            // This uses ~40 bytes/entry vs ~200 bytes/entry for a full Table.
            let rows = db.query_sorted_compact(limit)?;
            let n = rows.len();
            eprintln!("\x1b[2m  {} unique k-mers → streaming result (use head(n) or collect)\x1b[0m", n);
            drop(self); // drops KmerDiskStore → temp file cleaned up
            let iter = rows.into_iter().map(|(km, cnt)| {
                let mut rec = std::collections::HashMap::new();
                rec.insert("kmer".to_string(), Value::Str(km));
                rec.insert("count".to_string(), Value::Int(cnt));
                Value::Record(rec)
            });
            return Ok(Value::Stream(bl_core::value::StreamValue::new(
                format!("kmer_count({n} disk)"),
                Box::new(iter),
            )));
        }

        let merged = self.mem.unwrap();
        let limit = top_n.unwrap_or(usize::MAX);

        // For large results without a top-N limit, return a Stream to avoid
        // doubling memory by creating a full Table. head(10) will only
        // consume 10 rows instead of materializing millions.
        const STREAM_THRESHOLD: usize = 50_000;
        if limit >= merged.len() && merged.len() > STREAM_THRESHOLD {
            let n = merged.len();
            eprintln!("\x1b[2m  {} unique k-mers → streaming result (use head(n) or collect to materialize)\x1b[0m", n);
            // Sort into Vec<(count, kmer)> descending by count
            let mut sorted: Vec<(i64, String)> = merged.into_iter().map(|(k, c)| (c, k)).collect();
            sorted.sort_unstable_by(|a, b| b.0.cmp(&a.0));
            // Return a Stream that yields {kmer, count} records
            let iter = sorted.into_iter().map(|(cnt, km)| {
                let mut rec = std::collections::HashMap::new();
                rec.insert("kmer".to_string(), Value::Str(km));
                rec.insert("count".to_string(), Value::Int(cnt));
                Value::Record(rec)
            });
            return Ok(Value::Stream(bl_core::value::StreamValue::new(
                format!("kmer_count({n})"),
                Box::new(iter),
            )));
        }

        kmer_counts_to_table(merged, limit)
    }
}

/// Check available disk space at the given path. Returns bytes available, or None if unknown.
#[cfg(feature = "native")]
fn check_disk_space(path: &std::path::Path) -> Option<u64> {
    // Use Rust's fs::metadata + platform-specific disk space query
    // On all platforms, we can try to use the `available_space` from fs4 crate,
    // but to avoid adding a dependency, we shell out briefly:
    #[cfg(target_os = "windows")]
    {
        // PowerShell one-liner to get free bytes on the drive
        let drive = path.to_string_lossy();
        let drive_letter = drive.chars().next()?;
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command",
                &format!("(Get-PSDrive {drive_letter}).Free")])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&output.stdout);
        s.trim().parse::<u64>().ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("df")
            .args(["-P", "-B1", &path.to_string_lossy()])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&output.stdout);
        // df -P -B1 output: second line, 4th column is available bytes
        let line = s.lines().nth(1)?;
        let cols: Vec<&str> = line.split_whitespace().collect();
        cols.get(3)?.parse::<u64>().ok()
    }
}

#[cfg(feature = "native")]
struct KmerDiskStore {
    conn: rusqlite::Connection,
    tmp_path: std::path::PathBuf,
}

#[cfg(feature = "native")]
impl Drop for KmerDiskStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.tmp_path);
        crate::tempfiles::unregister(&self.tmp_path);
    }
}

#[cfg(feature = "native")]
impl KmerDiskStore {
    fn new() -> Result<Self> {
        let tmp_path = crate::tempfiles::temp_path("kmer");
        crate::tempfiles::register(tmp_path.clone());
        let conn = rusqlite::Connection::open(&tmp_path).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("failed to open SQLite temp DB: {e}"), None)
        })?;
        conn.execute_batch("
            PRAGMA journal_mode = OFF;
            PRAGMA synchronous = OFF;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA page_size = 8192;
            CREATE TABLE kmers (kmer TEXT PRIMARY KEY, count INTEGER NOT NULL) WITHOUT ROWID;
        ").map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite setup failed: {e}"), None)
        })?;
        Ok(Self { conn, tmp_path })
    }

    fn flush_map(&mut self, mem: &mut std::collections::HashMap<String, i64>) -> Result<()> {
        let tx = self.conn.transaction().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite transaction: {e}"), None)
        })?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO kmers (kmer, count) VALUES (?1, ?2)
                 ON CONFLICT(kmer) DO UPDATE SET count = count + ?2"
            ).map_err(|e| {
                BioLangError::runtime(ErrorKind::IOError, format!("SQLite prepare: {e}"), None)
            })?;
            for (kmer, count) in mem.drain() {
                stmt.execute(rusqlite::params![kmer, count]).map_err(|e| {
                    BioLangError::runtime(ErrorKind::IOError, format!("SQLite insert: {e}"), None)
                })?;
            }
        }
        tx.commit().map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite commit: {e}"), None)
        })?;
        Ok(())
    }

    fn to_table(&self, limit: usize) -> Result<Value> {
        let mut stmt = self.conn.prepare(
            "SELECT kmer, count FROM kmers ORDER BY count DESC LIMIT ?1"
        ).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite query: {e}"), None)
        })?;
        let rows: Vec<Vec<Value>> = stmt.query_map([limit as i64], |row| {
            let kmer: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok(vec![Value::Str(kmer), Value::Int(count)])
        }).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite query: {e}"), None)
        })?.filter_map(|r| r.ok()).collect();

        Ok(Value::Table(Table::new(
            vec!["kmer".into(), "count".into()],
            rows,
        )))
    }

    /// Query sorted rows as compact (String, i64) pairs — much less memory than Value rows.
    /// The DB can be dropped immediately after this call; the Vec owns all data.
    fn query_sorted_compact(&self, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT kmer, count FROM kmers ORDER BY count DESC LIMIT ?1"
        ).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite query: {e}"), None)
        })?;
        let rows: Vec<(String, i64)> = stmt.query_map([limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SQLite query: {e}"), None)
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }
}

fn kmer_counts_to_table(merged: std::collections::HashMap<String, i64>, limit: usize) -> Result<Value> {
    if limit < merged.len() {
        // Partial sort: only need top `limit` entries
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;
        let mut heap: BinaryHeap<Reverse<(i64, String)>> = BinaryHeap::with_capacity(limit + 1);
        for (km, cnt) in merged {
            heap.push(Reverse((cnt, km)));
            if heap.len() > limit {
                heap.pop();
            }
        }
        let mut rows: Vec<Vec<Value>> = heap
            .into_iter()
            .map(|Reverse((cnt, km))| vec![Value::Str(km), Value::Int(cnt)])
            .collect();
        rows.sort_by(|a, b| {
            if let (Value::Int(ca), Value::Int(cb)) = (&a[1], &b[1]) {
                cb.cmp(ca)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Ok(Value::Table(Table::new(
            vec!["kmer".into(), "count".into()],
            rows,
        )))
    } else {
        let mut rows: Vec<Vec<Value>> = merged
            .into_iter()
            .map(|(km, cnt)| vec![Value::Str(km), Value::Int(cnt)])
            .collect();
        rows.sort_by(|a, b| {
            if let (Value::Int(ca), Value::Int(cb)) = (&a[1], &b[1]) {
                cb.cmp(ca)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Ok(Value::Table(Table::new(
            vec!["kmer".into(), "count".into()],
            rows,
        )))
    }
}

/// Convert u64-keyed kmer counts to Value output.
/// Decodes kmer integers to strings only at the final output stage.
fn fast_counts_to_value<S: std::hash::BuildHasher>(counts: std::collections::HashMap<u64, i64, S>, k: u8, top_n: Option<usize>) -> Result<Value> {
    let limit = top_n.unwrap_or(usize::MAX);

    // Streaming mode for large unbounded results — lazy heap-pop iterator
    // Heapify is O(n), then each pop is O(log n). take(20) = O(n + 20 log n) ≈ O(n)
    // vs full sort O(n log n). Massive win when only top-K items consumed.
    const STREAM_THRESHOLD: usize = 50_000;
    if limit >= counts.len() && counts.len() > STREAM_THRESHOLD {
        let n = counts.len();
        eprintln!("\x1b[2m  {} unique k-mers → streaming result (use head(n) or collect to materialize)\x1b[0m", n);
        let heap: std::collections::BinaryHeap<(i64, u64)> =
            counts.into_iter().map(|(enc, c)| (c, enc)).collect();
        // Lazy iterator: pops from max-heap one at a time
        let iter = HeapPopIter { heap }.map(move |(cnt, enc)| {
            let kmer_str = (bl_core::bio_core::Kmer { encoded: enc, k }).decode();
            let mut rec = std::collections::HashMap::new();
            rec.insert("kmer".to_string(), Value::Str(kmer_str));
            rec.insert("count".to_string(), Value::Int(cnt));
            Value::Record(rec)
        });
        return Ok(Value::Stream(bl_core::value::StreamValue::new(
            format!("kmer_count({n})"),
            Box::new(iter),
        )));
    }

    // Table mode with top-N: BinaryHeap partial sort O(n log limit)
    if limit < counts.len() {
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;
        let mut heap: BinaryHeap<Reverse<(i64, u64)>> = BinaryHeap::with_capacity(limit + 1);
        for (enc, cnt) in &counts {
            heap.push(Reverse((*cnt, *enc)));
            if heap.len() > limit {
                heap.pop();
            }
        }
        let mut rows: Vec<Vec<Value>> = heap
            .into_iter()
            .map(|Reverse((cnt, enc))| {
                let kmer_str = (bl_core::bio_core::Kmer { encoded: enc, k }).decode();
                vec![Value::Str(kmer_str), Value::Int(cnt)]
            })
            .collect();
        rows.sort_by(|a, b| {
            if let (Value::Int(ca), Value::Int(cb)) = (&a[1], &b[1]) {
                cb.cmp(ca)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Ok(Value::Table(Table::new(
            vec!["kmer".into(), "count".into()],
            rows,
        )))
    } else {
        let mut rows: Vec<Vec<Value>> = counts
            .into_iter()
            .map(|(enc, cnt)| {
                let kmer_str = (bl_core::bio_core::Kmer { encoded: enc, k }).decode();
                vec![Value::Str(kmer_str), Value::Int(cnt)]
            })
            .collect();
        rows.sort_by(|a, b| {
            if let (Value::Int(ca), Value::Int(cb)) = (&a[1], &b[1]) {
                cb.cmp(ca)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Ok(Value::Table(Table::new(
            vec!["kmer".into(), "count".into()],
            rows,
        )))
    }
}

/// Lazy iterator that pops from a BinaryHeap one item at a time (max first).
struct HeapPopIter<T: Ord> {
    heap: std::collections::BinaryHeap<T>,
}

impl<T: Ord> Iterator for HeapPopIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.heap.pop()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.heap.len(), Some(self.heap.len()))
    }
}

fn builtin_kmer_spectrum(args: Vec<Value>) -> Result<Value> {
    let counts_table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("kmer_spectrum() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    // Extract count column (second column)
    let count_idx = if counts_table.columns.len() > 1 { 1 } else { 0 };
    let mut freq_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for row in &counts_table.rows {
        if let Some(Value::Int(cnt)) = row.get(count_idx) {
            *freq_map.entry(*cnt).or_insert(0) += 1;
        }
    }
    let mut rows: Vec<Vec<Value>> = freq_map
        .into_iter()
        .map(|(freq, cnt)| vec![Value::Int(freq), Value::Int(cnt)])
        .collect();
    rows.sort_by_key(|r| {
        if let Value::Int(f) = &r[0] { *f } else { 0 }
    });
    Ok(Value::Table(Table::new(
        vec!["frequency".into(), "count".into()],
        rows,
    )))
}

fn builtin_minimizers(args: Vec<Value>) -> Result<Value> {
    use bl_core::bio_core::Kmer;
    let seq = match &args[0] {
        Value::DNA(s) | Value::RNA(s) => &s.data,
        Value::Str(s) => s,
        other => {
            return Err(BioLangError::type_error(
                format!("minimizers() requires DNA/RNA/Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let k = match &args[1] {
        Value::Int(n) => *n as u8,
        other => {
            return Err(BioLangError::type_error(
                format!("minimizers() k must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    let w = match &args[2] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("minimizers() w must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };

    let mins = Kmer::minimizers(seq, k, w);
    let result: Vec<Value> = mins
        .into_iter()
        .map(|(km, pos)| {
            let mut rec = std::collections::HashMap::new();
            rec.insert("kmer".to_string(), Value::Kmer(km));
            rec.insert("pos".to_string(), Value::Int(pos as i64));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(result))
}

// ── GAP 3: Streaming builtins ───────────────────────────────────

fn builtin_stream_chunks(args: Vec<Value>) -> Result<Value> {
    let stream = match &args[0] {
        Value::Stream(s) => s.clone(),
        Value::List(items) => StreamValue::from_list("list", items.clone()),
        other => {
            return Err(BioLangError::type_error(
                format!("stream_chunks() requires Stream or List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n = require_int(&args[1], "stream_chunks")? as usize;
    if n == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError, "stream_chunks() chunk size must be > 0", None,
        ));
    }

    let inner = Arc::new(Mutex::new(stream));
    let chunk_iter = std::iter::from_fn(move || {
        let s = inner.lock().unwrap();
        let mut chunk = Vec::with_capacity(n);
        for _ in 0..n {
            match s.next() {
                Some(v) => chunk.push(v),
                None => break,
            }
        }
        if chunk.is_empty() {
            None
        } else {
            Some(Value::List(chunk))
        }
    });

    Ok(Value::Stream(StreamValue::new(
        "chunks",
        Box::new(chunk_iter),
    )))
}

fn builtin_stream_take(args: Vec<Value>) -> Result<Value> {
    let stream = match &args[0] {
        Value::Stream(s) => s.clone(),
        Value::List(items) => StreamValue::from_list("list", items.clone()),
        other => {
            return Err(BioLangError::type_error(
                format!("stream_take() requires Stream or List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n = require_int(&args[1], "stream_take")? as usize;
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        match stream.next() {
            Some(v) => result.push(v),
            None => break,
        }
    }
    Ok(Value::List(result))
}

fn builtin_stream_skip(args: Vec<Value>) -> Result<Value> {
    let stream = match &args[0] {
        Value::Stream(s) => s.clone(),
        Value::List(items) => StreamValue::from_list("list", items.clone()),
        other => {
            return Err(BioLangError::type_error(
                format!("stream_skip() requires Stream or List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n = require_int(&args[1], "stream_skip")? as usize;
    // Skip n items
    for _ in 0..n {
        if stream.next().is_none() {
            break;
        }
    }
    // Return remaining as stream
    let inner = Arc::new(Mutex::new(stream));
    let rest_iter = std::iter::from_fn(move || inner.lock().unwrap().next());
    Ok(Value::Stream(StreamValue::new("skip", Box::new(rest_iter))))
}

fn builtin_memory_usage() -> Result<Value> {
    // Approximate: report process-level info where available
    let mut rec = std::collections::HashMap::new();
    // We can't easily get heap size in pure Rust without allocator hooks
    // Report what we can: placeholder for system-level info
    rec.insert("heap_bytes".to_string(), Value::Int(0));
    rec.insert("note".to_string(), Value::Str("approximate — use OS tools for precise measurement".to_string()));
    Ok(Value::Record(rec))
}

// ── GAP 6: Typed table columns ──────────────────────────────────

fn builtin_table_col_types(args: Vec<Value>) -> Result<Value> {
    let t = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("table_col_types() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut types = std::collections::HashMap::new();
    for (ci, col) in t.columns.iter().enumerate() {
        let mut type_name = "Nil";
        for row in &t.rows {
            if let Some(val) = row.get(ci) {
                match val {
                    Value::Nil => {}
                    _ => {
                        type_name = match val {
                            Value::Int(_) => "Int",
                            Value::Float(_) => "Float",
                            Value::Str(_) => "Str",
                            Value::Bool(_) => "Bool",
                            _ => "Mixed",
                        };
                        break;
                    }
                }
            }
        }
        types.insert(col.clone(), Value::Str(type_name.to_string()));
    }
    Ok(Value::Record(types))
}

fn builtin_table_set_col_type(args: Vec<Value>) -> Result<Value> {
    let t = match &args[0] {
        Value::Table(t) => t.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_set_col_type() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let col = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_set_col_type() col must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let type_str = match &args[2] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_set_col_type() type must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    let mut schema = std::collections::HashMap::new();
    for c in &t.columns {
        schema.insert(c.clone(), Value::Str("Any".to_string()));
    }
    schema.insert(col, Value::Str(type_str));

    let mut result = std::collections::HashMap::new();
    result.insert("table".to_string(), Value::Table(t));
    result.insert("schema".to_string(), Value::Record(schema));
    Ok(Value::Record(result))
}

fn builtin_table_validate(args: Vec<Value>) -> Result<Value> {
    let rec = match &args[0] {
        Value::Record(r) => r,
        other => {
            return Err(BioLangError::type_error(
                format!("table_validate() requires Record, got {}", other.type_of()),
                None,
            ))
        }
    };

    let table = match rec.get("table") {
        Some(Value::Table(t)) => t,
        _ => {
            let mut result = std::collections::HashMap::new();
            result.insert("valid".to_string(), Value::Bool(false));
            result.insert("errors".to_string(), Value::List(vec![Value::Str("missing 'table' field".into())]));
            return Ok(Value::Record(result));
        }
    };
    let schema = match rec.get("schema") {
        Some(Value::Record(s)) => s,
        _ => {
            let mut result = std::collections::HashMap::new();
            result.insert("valid".to_string(), Value::Bool(true));
            result.insert("errors".to_string(), Value::List(vec![]));
            return Ok(Value::Record(result));
        }
    };

    let mut errors = Vec::new();
    for (col, type_val) in schema {
        let type_str = match type_val {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        if type_str == "Any" {
            continue;
        }
        if let Some(ci) = table.col_index(col) {
            for (ri, row) in table.rows.iter().enumerate() {
                if let Some(val) = row.get(ci) {
                    let actual = format!("{}", val.type_of());
                    if actual != type_str && !matches!(val, Value::Nil) {
                        errors.push(Value::Str(format!(
                            "row {ri}, col '{col}': expected {type_str}, got {actual}"
                        )));
                    }
                }
            }
        }
    }

    let mut result = std::collections::HashMap::new();
    result.insert("valid".to_string(), Value::Bool(errors.is_empty()));
    result.insert("errors".to_string(), Value::List(errors));
    Ok(Value::Record(result))
}

fn builtin_table_schema(args: Vec<Value>) -> Result<Value> {
    let t = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("table_schema() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut result = std::collections::HashMap::new();
    result.insert(
        "columns".to_string(),
        Value::List(t.columns.iter().map(|c| Value::Str(c.clone())).collect()),
    );
    // Infer types
    let mut types = Vec::new();
    for ci in 0..t.columns.len() {
        let mut type_name = "Nil".to_string();
        for row in &t.rows {
            if let Some(val) = row.get(ci) {
                if !matches!(val, Value::Nil) {
                    type_name = format!("{}", val.type_of());
                    break;
                }
            }
        }
        types.push(Value::Str(type_name));
    }
    result.insert("types".to_string(), Value::List(types));
    result.insert("nrow".to_string(), Value::Int(t.num_rows() as i64));
    result.insert("ncol".to_string(), Value::Int(t.num_cols() as i64));
    Ok(Value::Record(result))
}

fn builtin_table_cast(args: Vec<Value>) -> Result<Value> {
    let t = match &args[0] {
        Value::Table(t) => t.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_cast() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let col = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_cast() col must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let target_type = match &args[2] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("table_cast() type must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    let ci = t.col_index(&col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, &format!("column '{col}' not found"), None)
    })?;

    let mut new_rows = t.rows.clone();
    for row in &mut new_rows {
        if let Some(val) = row.get(ci).cloned() {
            row[ci] = match target_type.as_str() {
                "Int" => match &val {
                    Value::Int(_) => val,
                    Value::Float(f) => Value::Int(*f as i64),
                    Value::Str(s) => s.parse::<i64>().map(Value::Int).unwrap_or(Value::Nil),
                    _ => Value::Nil,
                },
                "Float" => match &val {
                    Value::Float(_) => val,
                    Value::Int(n) => Value::Float(*n as f64),
                    Value::Str(s) => s.parse::<f64>().map(Value::Float).unwrap_or(Value::Nil),
                    _ => Value::Nil,
                },
                "Str" => Value::Str(format!("{val}")),
                _ => val,
            };
        }
    }
    Ok(Value::Table(Table::new(t.columns.clone(), new_rows)))
}

// ── GAP 7: Pipe fusion (explicit builtin) ───────────────────────

fn builtin_pipe_fuse(args: Vec<Value>) -> Result<Value> {
    // pipe_fuse(list, op1, op2, ...)
    // Each op is a NativeFunction (map/filter) — apply in single pass
    let items = match &args[0] {
        Value::List(l) => l.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("pipe_fuse() first arg must be List, got {}", other.type_of()),
                None,
            ))
        }
    };

    // For now, pipe_fuse just returns the list (actual fusion done in interpreter for pipe chains)
    // This serves as documentation and optimization hint
    Ok(Value::List(items))
}

// ── GAP 8: Data provenance builtins ─────────────────────────────

fn builtin_with_provenance(args: Vec<Value>) -> Result<Value> {
    let value = args[0].clone();
    let meta = match &args[1] {
        Value::Record(m) => m.clone(),
        Value::Str(s) => {
            let mut m = std::collections::HashMap::new();
            m.insert("operation".to_string(), Value::Str(s.clone()));
            m
        }
        other => {
            return Err(BioLangError::type_error(
                format!("with_provenance() meta must be Record or Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    let mut prov = std::collections::HashMap::new();
    prov.insert("timestamp".to_string(), Value::Str(chrono::Utc::now().to_rfc3339()));
    for (k, v) in meta {
        prov.insert(k, v);
    }

    let mut result = std::collections::HashMap::new();
    result.insert("__value".to_string(), value);
    result.insert("__provenance".to_string(), Value::Record(prov));
    Ok(Value::Record(result))
}

fn builtin_provenance(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Record(map) => match map.get("__provenance") {
            Some(prov) => Ok(prov.clone()),
            None => Ok(Value::Nil),
        },
        _ => Ok(Value::Nil),
    }
}

fn builtin_provenance_chain(args: Vec<Value>) -> Result<Value> {
    let mut chain = Vec::new();
    let mut current = args[0].clone();
    for _ in 0..100 {
        // Safety limit
        match &current {
            Value::Record(map) => {
                if let Some(prov) = map.get("__provenance") {
                    chain.push(prov.clone());
                    if let Value::Record(p) = prov {
                        if let Some(parent) = p.get("parent") {
                            current = parent.clone();
                            continue;
                        }
                    }
                }
                break;
            }
            _ => break,
        }
    }
    Ok(Value::List(chain))
}

fn builtin_checkpoint(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("checkpoint() name must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let value = &args[1];

    // Save to ~/.biolang/checkpoints/{name}.json
    #[cfg(feature = "native")]
    {
        if let Some(home) = dirs::home_dir() {
            let dir = home.join(".biolang").join("checkpoints");
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join(format!("{name}.json"));
            let json = format!("{value}");
            let _ = std::fs::write(path, json);
        }
    }
    Ok(args[1].clone())
}

fn builtin_resume_checkpoint(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("resume_checkpoint() name must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    #[cfg(feature = "native")]
    {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".biolang").join("checkpoints").join(format!("{name}.json"));
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    return Ok(Value::Str(contents));
                }
            }
        }
    }
    Ok(Value::Nil)
}
