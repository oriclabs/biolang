//! BioLang builtins wrapping bl-apis typed clients.
//!
//! Uses thread_local! to reuse HTTP clients across calls within the same thread,
//! avoiding per-call TLS handshake overhead.

use std::cell::RefCell;
use std::collections::HashMap;

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

// ---------------------------------------------------------------------------
// Thread-local shared clients
// ---------------------------------------------------------------------------

thread_local! {
    static NCBI: bl_apis::NcbiClient = bl_apis::NcbiClient::new();
    static ENSEMBL: bl_apis::EnsemblClient = bl_apis::EnsemblClient::new();
    static UNIPROT: bl_apis::UniProtClient = bl_apis::UniProtClient::new();
    static UCSC: bl_apis::UcscClient = bl_apis::UcscClient::new();
    static BIOMART: bl_apis::BioMartClient = bl_apis::BioMartClient::new();
    static KEGG: bl_apis::KeggClient = bl_apis::KeggClient::new();
    static STRING_DB: bl_apis::StringDbClient = bl_apis::StringDbClient::new();
    static PDB: bl_apis::PdbClient = bl_apis::PdbClient::new();
    static REACTOME: bl_apis::ReactomeClient = bl_apis::ReactomeClient::new();
    static GO: bl_apis::GoClient = bl_apis::GoClient::new();
    static NCBI_DATASETS: bl_apis::NcbiDatasetsClient = bl_apis::NcbiDatasetsClient::new();
    // COSMIC requires API key — lazy-init on first use
    static COSMIC: RefCell<Option<bl_apis::CosmicClient>> = const { RefCell::new(None) };
    static BIOCONTAINERS: bl_apis::BioContainersClient = bl_apis::BioContainersClient::new();
    static NFCORE: bl_apis::NfCoreClient = bl_apis::NfCoreClient::new();
    static GALAXY: bl_apis::GalaxyToolShedClient = bl_apis::GalaxyToolShedClient::new();
}

fn with_cosmic<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&bl_apis::CosmicClient) -> bl_apis::Result<T>,
{
    COSMIC.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            *opt = Some(bl_apis::CosmicClient::new().map_err(api_err)?);
        }
        f(opt.as_ref().unwrap()).map_err(api_err)
    })
}

// ---------------------------------------------------------------------------
// Three-function registration pattern
// ---------------------------------------------------------------------------

pub fn apis_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // NCBI
        ("ncbi_search", Arity::Range(2, 3)),
        ("ncbi_fetch", Arity::Range(2, 3)),
        ("ncbi_summary", Arity::Exact(2)),
        ("ncbi_gene", Arity::Range(1, 2)),
        ("ncbi_pubmed", Arity::Range(1, 2)),
        ("ncbi_sequence", Arity::Exact(1)),
        // Ensembl
        ("ensembl_gene", Arity::Exact(1)),
        ("ensembl_symbol", Arity::Exact(2)),
        ("ensembl_sequence", Arity::Range(1, 2)),
        ("ensembl_vep", Arity::Exact(1)),
        // UniProt
        ("uniprot_search", Arity::Range(1, 2)),
        ("uniprot_entry", Arity::Exact(1)),
        ("uniprot_fasta", Arity::Exact(1)),
        ("uniprot_features", Arity::Exact(1)),
        ("uniprot_go", Arity::Exact(1)),
        // UCSC
        ("ucsc_genomes", Arity::Exact(0)),
        ("ucsc_sequence", Arity::Exact(4)),
        ("ucsc_tracks", Arity::Exact(1)),
        // BioMart
        ("biomart_query", Arity::Exact(3)),
        // KEGG
        ("kegg_get", Arity::Exact(1)),
        ("kegg_find", Arity::Exact(2)),
        ("kegg_link", Arity::Exact(2)),
        // STRING
        ("string_network", Arity::Exact(2)),
        ("string_enrichment", Arity::Exact(2)),
        // PDB
        ("pdb_entry", Arity::Exact(1)),
        ("pdb_search", Arity::Exact(1)),
        ("pdb_entity", Arity::Exact(2)),
        ("pdb_sequence", Arity::Exact(2)),
        ("pdb_chains", Arity::Exact(1)),
        // PubMed
        ("pubmed_search", Arity::Range(1, 2)),
        ("pubmed_abstract", Arity::Exact(1)),
        // Reactome
        ("reactome_search", Arity::Exact(1)),
        ("reactome_pathways", Arity::Range(1, 2)),
        // GO
        ("go_term", Arity::Exact(1)),
        ("go_annotations", Arity::Range(1, 2)),
        ("go_children", Arity::Exact(1)),
        ("go_parents", Arity::Exact(1)),
        ("go_ancestors", Arity::Exact(1)),
        ("go_descendants", Arity::Exact(1)),
        // COSMIC
        ("cosmic_gene", Arity::Exact(1)),
        // NCBI Datasets
        ("datasets_gene", Arity::Range(1, 2)),
        // BioContainers
        ("biocontainers_search", Arity::Range(1, 2)),
        ("biocontainers_popular", Arity::Range(0, 1)),
        ("biocontainers_info", Arity::Exact(1)),
        ("biocontainers_versions", Arity::Exact(1)),
        // nf-core
        ("nfcore_list", Arity::Range(0, 2)),
        ("nfcore_search", Arity::Range(1, 2)),
        ("nfcore_info", Arity::Exact(1)),
        ("nfcore_releases", Arity::Exact(1)),
        ("nfcore_params", Arity::Exact(1)),
        // Galaxy ToolShed
        ("galaxy_search", Arity::Range(1, 2)),
        ("galaxy_popular", Arity::Range(0, 1)),
        ("galaxy_categories", Arity::Exact(0)),
        ("galaxy_tool", Arity::Exact(2)),
        // Config
        ("api_endpoints", Arity::Exact(0)),
        // Utility
        ("bio_icon", Arity::Exact(1)),
        ("paper_score", Arity::Range(1, 2)),
    ]
}

pub fn is_apis_builtin(name: &str) -> bool {
    matches!(
        name,
        "ncbi_search"
            | "ncbi_fetch"
            | "ncbi_summary"
            | "ncbi_gene"
            | "ncbi_pubmed"
            | "ncbi_sequence"
            | "ensembl_gene"
            | "ensembl_symbol"
            | "ensembl_sequence"
            | "ensembl_vep"
            | "uniprot_search"
            | "uniprot_entry"
            | "uniprot_fasta"
            | "uniprot_features"
            | "uniprot_go"
            | "ucsc_genomes"
            | "ucsc_sequence"
            | "ucsc_tracks"
            | "biomart_query"
            | "kegg_get"
            | "kegg_find"
            | "kegg_link"
            | "string_network"
            | "string_enrichment"
            | "pdb_entry"
            | "pdb_search"
            | "pdb_entity"
            | "pdb_sequence"
            | "pdb_chains"
            | "pubmed_search"
            | "pubmed_abstract"
            | "reactome_search"
            | "reactome_pathways"
            | "go_term"
            | "go_annotations"
            | "go_children"
            | "go_parents"
            | "go_ancestors"
            | "go_descendants"
            | "cosmic_gene"
            | "datasets_gene"
            | "biocontainers_search"
            | "biocontainers_popular"
            | "biocontainers_info"
            | "biocontainers_versions"
            | "nfcore_list"
            | "nfcore_search"
            | "nfcore_info"
            | "nfcore_releases"
            | "nfcore_params"
            | "galaxy_search"
            | "galaxy_popular"
            | "galaxy_categories"
            | "galaxy_tool"
            | "api_endpoints"
            | "bio_icon"
            | "paper_score"
    )
}

pub fn call_apis_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        // NCBI
        "ncbi_search" => builtin_ncbi_search(args),
        "ncbi_fetch" => builtin_ncbi_fetch(args),
        "ncbi_summary" => builtin_ncbi_summary(args),
        "ncbi_gene" => builtin_ncbi_gene(args),
        "ncbi_pubmed" => builtin_ncbi_pubmed(args),
        "ncbi_sequence" => builtin_ncbi_sequence(args),
        // Ensembl
        "ensembl_gene" => builtin_ensembl_gene(args),
        "ensembl_symbol" => builtin_ensembl_symbol(args),
        "ensembl_sequence" => builtin_ensembl_sequence(args),
        "ensembl_vep" => builtin_ensembl_vep(args),
        // UniProt
        "uniprot_search" => builtin_uniprot_search(args),
        "uniprot_entry" => builtin_uniprot_entry(args),
        "uniprot_fasta" => builtin_uniprot_fasta(args),
        "uniprot_features" => builtin_uniprot_features(args),
        "uniprot_go" => builtin_uniprot_go(args),
        // UCSC
        "ucsc_genomes" => builtin_ucsc_genomes(args),
        "ucsc_sequence" => builtin_ucsc_sequence(args),
        "ucsc_tracks" => builtin_ucsc_tracks(args),
        // BioMart
        "biomart_query" => builtin_biomart_query(args),
        // KEGG
        "kegg_get" => builtin_kegg_get(args),
        "kegg_find" => builtin_kegg_find(args),
        "kegg_link" => builtin_kegg_link(args),
        // STRING
        "string_network" => builtin_string_network(args),
        "string_enrichment" => builtin_string_enrichment(args),
        // PDB
        "pdb_entry" => builtin_pdb_entry(args),
        "pdb_search" => builtin_pdb_search(args),
        "pdb_entity" => builtin_pdb_entity(args),
        "pdb_sequence" => builtin_pdb_sequence(args),
        "pdb_chains" => builtin_pdb_chains(args),
        "pubmed_search" => builtin_pubmed_search(args),
        "pubmed_abstract" => builtin_pubmed_abstract(args),
        // Reactome
        "reactome_search" => builtin_reactome_search(args),
        "reactome_pathways" => builtin_reactome_pathways(args),
        // GO
        "go_term" => builtin_go_term(args),
        "go_annotations" => builtin_go_annotations(args),
        "go_children" => builtin_go_traversal(args, "children"),
        "go_parents" => builtin_go_traversal(args, "parents"),
        "go_ancestors" => builtin_go_traversal(args, "ancestors"),
        "go_descendants" => builtin_go_traversal(args, "descendants"),
        // COSMIC
        "cosmic_gene" => builtin_cosmic_gene(args),
        // NCBI Datasets
        "datasets_gene" => builtin_datasets_gene(args),
        // BioContainers
        "biocontainers_search" => builtin_biocontainers_search(args),
        "biocontainers_popular" => builtin_biocontainers_popular(args),
        "biocontainers_info" => builtin_biocontainers_info(args),
        "biocontainers_versions" => builtin_biocontainers_versions(args),
        // nf-core
        "nfcore_list" => builtin_nfcore_list(args),
        "nfcore_search" => builtin_nfcore_search(args),
        "nfcore_info" => builtin_nfcore_info(args),
        "nfcore_releases" => builtin_nfcore_releases(args),
        "nfcore_params" => builtin_nfcore_params(args),
        // Galaxy ToolShed
        "galaxy_search" => builtin_galaxy_search(args),
        "galaxy_popular" => builtin_galaxy_popular(args),
        "galaxy_categories" => builtin_galaxy_categories(args),
        "galaxy_tool" => builtin_galaxy_tool(args),
        // Config
        "api_endpoints" => builtin_api_endpoints(),
        // Utility
        "bio_icon" => builtin_bio_icon(args),
        "paper_score" => builtin_paper_score(args),

        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown API builtin '{name}'"),
            None,
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
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

fn require_list<'a>(val: &'a Value, func: &str) -> Result<&'a Vec<Value>> {
    match val {
        Value::List(l) => Ok(l),
        other => Err(BioLangError::type_error(
            format!("{func}() requires List, got {}", other.type_of()),
            None,
        )),
    }
}

fn api_err(e: bl_apis::ApiError) -> BioLangError {
    BioLangError::runtime(ErrorKind::IOError, e.to_string(), None)
}

/// Run a blocking API call with a status indicator on stderr.
/// Shows "⏳ message..." while running, clears when done.
fn with_progress<T>(message: &str, f: impl FnOnce() -> T) -> T {
    use std::io::Write;
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    if is_tty {
        eprint!("\x1b[90m{message}...\x1b[0m");
        let _ = std::io::stderr().flush();
    }
    let result = f();
    if is_tty {
        eprint!("\r\x1b[K");
        let _ = std::io::stderr().flush();
    }
    result
}

/// Convenience: run an API call with progress, then map_err.
fn api_call<T>(
    label: &str,
    f: impl FnOnce() -> std::result::Result<T, bl_apis::ApiError>,
) -> Result<T> {
    with_progress(label, f).map_err(api_err)
}

/// Convert a serde_json::Value to a BioLang Value.
fn json_to_value(j: &serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v));
            }
            Value::Record(map)
        }
    }
}

fn list_of_str_refs(list: &[Value], func: &str) -> Result<Vec<String>> {
    list.iter()
        .map(|v| match v {
            Value::Str(s) => Ok(s.clone()),
            other => Err(BioLangError::type_error(
                format!("{func}() list items must be Str, got {}", other.type_of()),
                None,
            )),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// NCBI builtins
// ---------------------------------------------------------------------------

fn builtin_ncbi_search(args: Vec<Value>) -> Result<Value> {
    let db = require_str(&args[0], "ncbi_search")?;
    let term = require_str(&args[1], "ncbi_search")?;
    let max = if args.len() > 2 {
        require_int(&args[2], "ncbi_search")? as usize
    } else {
        20
    };
    let result = api_call(
        &format!("ncbi_search({db}, \"{term}\")"),
        || NCBI.with(|c| c.esearch(db, term, max)),
    )?;
    // Return list of IDs directly for pipe-friendly usage:
    //   ncbi_search("gene", "BRCA1") |> first() |> ncbi_gene()
    Ok(Value::List(result.ids.into_iter().map(Value::Str).collect()))
}

/// `ncbi_fetch(ids, db, rettype?)` — pipe-friendly: ids first so
/// `ncbi_search("gene", "BRCA1") |> first() |> ncbi_fetch("gene")` works.
fn builtin_ncbi_fetch(args: Vec<Value>) -> Result<Value> {
    // Accept either a single ID string or a list of IDs as first arg
    let ids_owned: Vec<String> = match &args[0] {
        Value::Str(s) => vec![s.clone()],
        Value::List(items) => list_of_str_refs(items, "ncbi_fetch")?,
        Value::Int(n) => vec![n.to_string()],
        other => {
            return Err(BioLangError::type_error(
                format!("ncbi_fetch() ids must be Str, Int, or List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let db = require_str(&args[1], "ncbi_fetch")?;
    // Default rettype based on database
    let rettype = if args.len() > 2 {
        require_str(&args[2], "ncbi_fetch")?
    } else {
        match db {
            "nucleotide" | "protein" => "fasta",
            "gene" => "xml",
            "pubmed" => "abstract",
            _ => "xml",
        }
    };
    let id_refs: Vec<&str> = ids_owned.iter().map(|s| s.as_str()).collect();
    let text = api_call(
        &format!("ncbi_fetch({db}, ...)"),
        || NCBI.with(|c| c.efetch_text(db, &id_refs, rettype)),
    )?;
    Ok(Value::Str(text))
}

/// `ncbi_summary(ids, db)` — pipe-friendly: ids first.
fn builtin_ncbi_summary(args: Vec<Value>) -> Result<Value> {
    let ids: Vec<String> = match &args[0] {
        Value::Str(s) => vec![s.clone()],
        Value::Int(n) => vec![n.to_string()],
        Value::List(items) => list_of_str_refs(items, "ncbi_summary")?,
        other => {
            return Err(BioLangError::type_error(
                format!("ncbi_summary() ids must be Str, Int, or List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let db = require_str(&args[1], "ncbi_summary")?;
    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let docs = api_call(
        &format!("ncbi_summary({db}, ...)"),
        || NCBI.with(|c| c.esummary(db, &id_refs)),
    )?;
    let items: Vec<Value> = docs
        .into_iter()
        .map(|d| {
            let mut map = HashMap::new();
            map.insert("uid".to_string(), Value::Str(d.uid));
            for (k, v) in d.fields {
                map.insert(k, json_to_value(&v));
            }
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_ncbi_gene(args: Vec<Value>) -> Result<Value> {
    let term = require_str(&args[0], "ncbi_gene")?;
    let max = if args.len() > 1 {
        require_int(&args[1], "ncbi_gene")? as usize
    } else {
        10
    };
    let result = api_call(
        &format!("ncbi_gene(\"{term}\")"),
        || NCBI.with(|c| c.search_gene(term, max)),
    )?;
    // If single result, return gene summary directly; otherwise return ID list
    if result.ids.len() == 1 {
        let id = &result.ids[0];
        let summaries = api_call(
            &format!("ncbi_gene(\"{term}\") summary"),
            || NCBI.with(|c| c.fetch_gene_summary(&[id.as_str()])),
        )?;
        if let Some(gene) = summaries.into_iter().next() {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(gene.id));
            map.insert("symbol".to_string(), Value::Str(gene.symbol));
            map.insert("name".to_string(), Value::Str(gene.name));
            map.insert("description".to_string(), Value::Str(gene.description));
            map.insert("organism".to_string(), Value::Str(gene.organism));
            map.insert("chromosome".to_string(), Value::Str(gene.chromosome));
            map.insert("location".to_string(), Value::Str(gene.location));
            map.insert("summary".to_string(), Value::Str(gene.summary));
            return Ok(Value::Record(map));
        }
    }
    Ok(Value::List(result.ids.into_iter().map(Value::Str).collect()))
}

fn builtin_ncbi_pubmed(args: Vec<Value>) -> Result<Value> {
    let term = require_str(&args[0], "ncbi_pubmed")?;
    let max = if args.len() > 1 {
        require_int(&args[1], "ncbi_pubmed")? as usize
    } else {
        10
    };
    let result = api_call(
        &format!("ncbi_pubmed(\"{term}\")"),
        || NCBI.with(|c| c.search_pubmed(term, max)),
    )?;
    Ok(Value::List(result.ids.into_iter().map(Value::Str).collect()))
}

fn builtin_ncbi_sequence(args: Vec<Value>) -> Result<Value> {
    let id = require_str(&args[0], "ncbi_sequence")?;
    let fasta = api_call(
        &format!("ncbi_sequence(\"{id}\")"),
        || NCBI.with(|c| c.fetch_sequence(id)),
    )?;
    Ok(Value::Str(fasta))
}

// ---------------------------------------------------------------------------
// Ensembl builtins
// ---------------------------------------------------------------------------

fn builtin_ensembl_gene(args: Vec<Value>) -> Result<Value> {
    let id = require_str(&args[0], "ensembl_gene")?;
    let gene = api_call(
        &format!("ensembl_gene(\"{id}\")"),
        || ENSEMBL.with(|c| c.gene_by_id(id)),
    )?;
    Ok(gene_to_record(&gene))
}

fn builtin_ensembl_symbol(args: Vec<Value>) -> Result<Value> {
    let species = require_str(&args[0], "ensembl_symbol")?;
    let symbol = require_str(&args[1], "ensembl_symbol")?;
    let gene = api_call(
        &format!("ensembl_symbol(\"{species}\", \"{symbol}\")"),
        || ENSEMBL.with(|c| c.gene_by_symbol(species, symbol)),
    )?;
    Ok(gene_to_record(&gene))
}

fn gene_to_record(gene: &bl_apis::ensembl::Gene) -> Value {
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Str(gene.id.clone()));
    map.insert("symbol".to_string(), Value::Str(gene.symbol.clone()));
    map.insert("description".to_string(), Value::Str(gene.description.clone()));
    map.insert("species".to_string(), Value::Str(gene.species.clone()));
    map.insert("biotype".to_string(), Value::Str(gene.biotype.clone()));
    map.insert("start".to_string(), Value::Int(gene.start as i64));
    map.insert("end".to_string(), Value::Int(gene.end as i64));
    map.insert("strand".to_string(), Value::Int(gene.strand as i64));
    map.insert("chromosome".to_string(), Value::Str(gene.chromosome.clone()));
    Value::Record(map)
}

fn builtin_ensembl_sequence(args: Vec<Value>) -> Result<Value> {
    let id = require_str(&args[0], "ensembl_sequence")?;
    let seq_type = if args.len() > 1 {
        require_str(&args[1], "ensembl_sequence")?
    } else {
        "genomic"
    };
    let seq = api_call(
        &format!("ensembl_sequence(\"{id}\")"),
        || ENSEMBL.with(|c| c.sequence_by_id(id, seq_type)),
    )?;
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Str(seq.id));
    map.insert("seq".to_string(), Value::Str(seq.seq));
    map.insert("molecule".to_string(), Value::Str(seq.molecule));
    Ok(Value::Record(map))
}

fn builtin_ensembl_vep(args: Vec<Value>) -> Result<Value> {
    let hgvs = require_str(&args[0], "ensembl_vep")?;
    let results = api_call(
        &format!("ensembl_vep(\"{hgvs}\")"),
        || ENSEMBL.with(|c| c.vep_hgvs(hgvs)),
    )?;
    let items: Vec<Value> = results
        .into_iter()
        .map(|r| {
            let mut map = HashMap::new();
            map.insert("allele_string".to_string(), Value::Str(r.allele_string));
            map.insert(
                "most_severe_consequence".to_string(),
                Value::Str(r.most_severe_consequence),
            );
            let tcs: Vec<Value> = r
                .transcript_consequences
                .into_iter()
                .map(|tc| {
                    let mut m = HashMap::new();
                    m.insert("gene_id".to_string(), Value::Str(tc.gene_id));
                    m.insert("transcript_id".to_string(), Value::Str(tc.transcript_id));
                    m.insert("impact".to_string(), Value::Str(tc.impact));
                    m.insert(
                        "consequences".to_string(),
                        Value::List(
                            tc.consequence_terms.into_iter().map(Value::Str).collect(),
                        ),
                    );
                    Value::Record(m)
                })
                .collect();
            map.insert("transcript_consequences".to_string(), Value::List(tcs));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// UniProt builtins
// ---------------------------------------------------------------------------

fn builtin_uniprot_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "uniprot_search")?;
    let limit = if args.len() > 1 {
        require_int(&args[1], "uniprot_search")? as usize
    } else {
        10
    };
    let entries = api_call(
        &format!("uniprot_search(\"{query}\")"),
        || UNIPROT.with(|c| c.search(query, limit)),
    )?;
    let items: Vec<Value> = entries.into_iter().map(|e| protein_to_record(&e)).collect();
    Ok(Value::List(items))
}

fn builtin_uniprot_entry(args: Vec<Value>) -> Result<Value> {
    let acc = require_str(&args[0], "uniprot_entry")?;
    let entry = api_call(
        &format!("uniprot_entry(\"{acc}\")"),
        || UNIPROT.with(|c| c.entry(acc)),
    )?;
    Ok(protein_to_record(&entry))
}

fn protein_to_record(e: &bl_apis::uniprot::ProteinEntry) -> Value {
    let mut map = HashMap::new();
    map.insert("accession".to_string(), Value::Str(e.accession.clone()));
    map.insert("name".to_string(), Value::Str(e.name.clone()));
    map.insert("organism".to_string(), Value::Str(e.organism.clone()));
    map.insert("sequence_length".to_string(), Value::Int(e.sequence_length as i64));
    map.insert(
        "gene_names".to_string(),
        Value::List(e.gene_names.iter().map(|s| Value::Str(s.clone())).collect()),
    );
    map.insert("function".to_string(), Value::Str(e.function.clone()));
    Value::Record(map)
}

fn builtin_uniprot_fasta(args: Vec<Value>) -> Result<Value> {
    let acc = require_str(&args[0], "uniprot_fasta")?;
    let fasta = api_call(
        &format!("uniprot_fasta(\"{acc}\")"),
        || UNIPROT.with(|c| c.entry_fasta(acc)),
    )?;
    Ok(Value::Str(fasta))
}

fn builtin_uniprot_features(args: Vec<Value>) -> Result<Value> {
    let acc = require_str(&args[0], "uniprot_features")?;
    let feats = api_call(
        &format!("uniprot_features(\"{acc}\")"),
        || UNIPROT.with(|c| c.features(acc)),
    )?;
    let items: Vec<Value> = feats
        .into_iter()
        .map(|f| {
            let mut map = HashMap::new();
            map.insert("type".to_string(), Value::Str(f.type_));
            map.insert("location".to_string(), Value::Str(f.location));
            map.insert("description".to_string(), Value::Str(f.description));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_uniprot_go(args: Vec<Value>) -> Result<Value> {
    let acc = require_str(&args[0], "uniprot_go")?;
    let terms = api_call(
        &format!("uniprot_go(\"{acc}\")"),
        || UNIPROT.with(|c| c.go_terms(acc)),
    )?;
    let items: Vec<Value> = terms
        .into_iter()
        .map(|t| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(t.id));
            map.insert("term".to_string(), Value::Str(t.term));
            map.insert("aspect".to_string(), Value::Str(t.aspect));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// UCSC builtins
// ---------------------------------------------------------------------------

fn builtin_ucsc_genomes(_args: Vec<Value>) -> Result<Value> {
    let genomes = api_call("ucsc_genomes()", || UCSC.with(|c| c.list_genomes()))?;
    let items: Vec<Value> = genomes
        .into_iter()
        .map(|g| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::Str(g.name));
            map.insert("description".to_string(), Value::Str(g.description));
            map.insert("organism".to_string(), Value::Str(g.organism));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_ucsc_sequence(args: Vec<Value>) -> Result<Value> {
    let genome = require_str(&args[0], "ucsc_sequence")?;
    let chrom = require_str(&args[1], "ucsc_sequence")?;
    let start = require_int(&args[2], "ucsc_sequence")? as u64;
    let end = require_int(&args[3], "ucsc_sequence")? as u64;
    let seq = api_call(
        &format!("ucsc_sequence(\"{genome}\", \"{chrom}\")"),
        || UCSC.with(|c| c.get_sequence(genome, chrom, start, end)),
    )?;
    Ok(Value::Str(seq))
}

fn builtin_ucsc_tracks(args: Vec<Value>) -> Result<Value> {
    let genome = require_str(&args[0], "ucsc_tracks")?;
    let tracks = api_call(
        &format!("ucsc_tracks(\"{genome}\")"),
        || UCSC.with(|c| c.list_tracks(genome)),
    )?;
    let items: Vec<Value> = tracks
        .into_iter()
        .map(|t| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::Str(t.name));
            map.insert("short_label".to_string(), Value::Str(t.short_label));
            map.insert("long_label".to_string(), Value::Str(t.long_label));
            map.insert("type".to_string(), Value::Str(t.type_));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// BioMart builtins
// ---------------------------------------------------------------------------

fn builtin_biomart_query(args: Vec<Value>) -> Result<Value> {
    let dataset = require_str(&args[0], "biomart_query")?;
    let attrs_val = require_list(&args[1], "biomart_query")?;
    let filters_val = require_list(&args[2], "biomart_query")?;

    let attrs = list_of_str_refs(attrs_val, "biomart_query")?;
    let attr_refs: Vec<&str> = attrs.iter().map(|s| s.as_str()).collect();

    // Filters: list of [key, value] pairs
    let mut filter_pairs: Vec<(String, String)> = Vec::new();
    for item in filters_val {
        if let Value::List(pair) = item {
            if pair.len() >= 2 {
                let k = require_str(&pair[0], "biomart_query")?;
                let v = require_str(&pair[1], "biomart_query")?;
                filter_pairs.push((k.to_string(), v.to_string()));
            }
        }
    }
    let filter_refs: Vec<(&str, &str)> = filter_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let rows = api_call(
        &format!("biomart_query(\"{dataset}\")"),
        || BIOMART.with(|c| c.query(dataset, &attr_refs, &filter_refs)),
    )?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|row| Value::List(row.into_iter().map(Value::Str).collect()))
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// KEGG builtins
// ---------------------------------------------------------------------------

fn builtin_kegg_get(args: Vec<Value>) -> Result<Value> {
    let entry = require_str(&args[0], "kegg_get")?;
    let text = api_call(&format!("kegg_get(\"{entry}\")"), || KEGG.with(|c| c.get(entry)))?;
    Ok(Value::Str(text))
}

fn builtin_kegg_find(args: Vec<Value>) -> Result<Value> {
    let db = require_str(&args[0], "kegg_find")?;
    let query = require_str(&args[1], "kegg_find")?;
    let entries = api_call(&format!("kegg_find(\"{db}\", \"{query}\")"), || KEGG.with(|c| c.find(db, query)))?;
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(e.id));
            map.insert("description".to_string(), Value::Str(e.description));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_kegg_link(args: Vec<Value>) -> Result<Value> {
    let target = require_str(&args[0], "kegg_link")?;
    let source = require_str(&args[1], "kegg_link")?;
    let links = api_call(&format!("kegg_link(\"{target}\", \"{source}\")"), || KEGG.with(|c| c.link(target, source)))?;
    let items: Vec<Value> = links
        .into_iter()
        .map(|l| {
            let mut map = HashMap::new();
            map.insert("source".to_string(), Value::Str(l.source));
            map.insert("target".to_string(), Value::Str(l.target));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// STRING builtins
// ---------------------------------------------------------------------------

fn builtin_string_network(args: Vec<Value>) -> Result<Value> {
    let ids_val = require_list(&args[0], "string_network")?;
    let species = require_int(&args[1], "string_network")? as u32;
    let ids = list_of_str_refs(ids_val, "string_network")?;
    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let interactions = api_call("string_network(...)", || STRING_DB.with(|c| c.network(&id_refs, species)))?;
    let items: Vec<Value> = interactions
        .into_iter()
        .map(|i| {
            let mut map = HashMap::new();
            map.insert("protein_a".to_string(), Value::Str(i.protein_a));
            map.insert("protein_b".to_string(), Value::Str(i.protein_b));
            map.insert("score".to_string(), Value::Float(i.score));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_string_enrichment(args: Vec<Value>) -> Result<Value> {
    let ids_val = require_list(&args[0], "string_enrichment")?;
    let species = require_int(&args[1], "string_enrichment")? as u32;
    let ids = list_of_str_refs(ids_val, "string_enrichment")?;
    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let enrichments = api_call("string_enrichment(...)", || STRING_DB.with(|c| c.enrichment(&id_refs, species)))?;
    let items: Vec<Value> = enrichments
        .into_iter()
        .map(|e| {
            let mut map = HashMap::new();
            map.insert("category".to_string(), Value::Str(e.category));
            map.insert("term".to_string(), Value::Str(e.term));
            map.insert("description".to_string(), Value::Str(e.description));
            map.insert("gene_count".to_string(), Value::Int(e.gene_count as i64));
            map.insert("p_value".to_string(), Value::Float(e.p_value));
            map.insert("fdr".to_string(), Value::Float(e.fdr));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// PDB builtins
// ---------------------------------------------------------------------------

fn builtin_pdb_entry(args: Vec<Value>) -> Result<Value> {
    let id = require_str(&args[0], "pdb_entry")?;
    let entry = api_call(&format!("pdb_entry(\"{id}\")"), || PDB.with(|c| c.entry(id)))?;
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Str(entry.id));
    map.insert("title".to_string(), Value::Str(entry.title));
    map.insert("method".to_string(), Value::Str(entry.method));
    map.insert(
        "resolution".to_string(),
        entry
            .resolution
            .map(Value::Float)
            .unwrap_or(Value::Nil),
    );
    map.insert("release_date".to_string(), Value::Str(entry.release_date));
    map.insert("organism".to_string(), Value::Str(entry.organism));
    Ok(Value::Record(map))
}

fn builtin_pdb_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "pdb_search")?;
    let ids = api_call(&format!("pdb_search(\"{query}\")"), || PDB.with(|c| c.search(query)))?;
    Ok(Value::List(ids.into_iter().map(Value::Str).collect()))
}

fn builtin_pdb_entity(args: Vec<Value>) -> Result<Value> {
    let pdb_id = require_str(&args[0], "pdb_entity")?;
    let entity_id = require_int(&args[1], "pdb_entity")? as u32;
    let ent = api_call(&format!("pdb_entity(\"{pdb_id}\", {entity_id})"), || {
        PDB.with(|c| c.entity(pdb_id, entity_id))
    })?;
    let mut map = HashMap::new();
    map.insert("entity_id".into(), Value::Int(ent.entity_id as i64));
    map.insert("description".into(), Value::Str(ent.description));
    map.insert("entity_type".into(), Value::Str(ent.entity_type));
    map.insert("sequence".into(), Value::Str(ent.sequence));
    Ok(Value::Record(map))
}

fn builtin_pdb_sequence(args: Vec<Value>) -> Result<Value> {
    let pdb_id = require_str(&args[0], "pdb_sequence")?;
    let entity_id = require_int(&args[1], "pdb_sequence")? as u32;
    let seq = api_call(&format!("pdb_sequence(\"{pdb_id}\", {entity_id})"), || {
        PDB.with(|c| c.sequence(pdb_id, entity_id))
    })?;
    Ok(Value::Protein(bl_core::value::BioSequence { data: seq.to_uppercase() }))
}

fn builtin_pdb_chains(args: Vec<Value>) -> Result<Value> {
    let pdb_id = require_str(&args[0], "pdb_chains")?;
    // Fetch entities 1..10 and collect those that exist
    let mut chains = Vec::new();
    for i in 1..=10u32 {
        match PDB.with(|c| c.entity(pdb_id, i)) {
            Ok(ent) => {
                let mut map = HashMap::new();
                map.insert("entity_id".into(), Value::Int(ent.entity_id as i64));
                map.insert("description".into(), Value::Str(ent.description));
                map.insert("entity_type".into(), Value::Str(ent.entity_type));
                map.insert("sequence".into(), Value::Str(ent.sequence));
                chains.push(Value::Record(map));
            }
            Err(_) => break,
        }
    }
    Ok(Value::List(chains))
}

// ---------------------------------------------------------------------------
// PubMed builtins
// ---------------------------------------------------------------------------

fn builtin_pubmed_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "pubmed_search")?;
    let max = if args.len() > 1 { require_int(&args[1], "pubmed_search")? as usize } else { 10 };
    let result = api_call(&format!("pubmed_search(\"{query}\")"), || {
        NCBI.with(|c| c.search_pubmed(query, max))
    })?;
    let ids: Vec<Value> = result.ids.into_iter().map(Value::Str).collect();
    let mut map = HashMap::new();
    map.insert("count".into(), Value::Int(result.count as i64));
    map.insert("ids".into(), Value::List(ids));
    Ok(Value::Record(map))
}

fn builtin_pubmed_abstract(args: Vec<Value>) -> Result<Value> {
    let pmid = require_str(&args[0], "pubmed_abstract")?;
    let text = api_call(&format!("pubmed_abstract(\"{pmid}\")"), || {
        NCBI.with(|c| c.efetch_text("pubmed", &[pmid], "abstract"))
    })?;
    Ok(Value::Str(text))
}

// ---------------------------------------------------------------------------
// Reactome builtins
// ---------------------------------------------------------------------------

fn builtin_reactome_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "reactome_search")?;
    let entries = api_call(&format!("reactome_search(\"{query}\")"), || REACTOME.with(|c| c.search(query)))?;
    let items: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(e.id));
            map.insert("name".to_string(), Value::Str(e.name));
            map.insert("schema_class".to_string(), Value::Str(e.schema_class));
            map.insert("species".to_string(), Value::Str(e.species));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_reactome_pathways(args: Vec<Value>) -> Result<Value> {
    let gene = require_str(&args[0], "reactome_pathways")?;
    let species = if args.len() > 1 {
        require_str(&args[1], "reactome_pathways")?
    } else {
        "Homo sapiens"
    };
    let pathways = api_call(
        &format!("reactome_pathways(\"{gene}\")"),
        || REACTOME.with(|c| c.pathways_for_gene(gene, species)),
    )?;
    let items: Vec<Value> = pathways
        .into_iter()
        .map(|p| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(p.id));
            map.insert("name".to_string(), Value::Str(p.name));
            map.insert("species".to_string(), Value::Str(p.species));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// GO builtins
// ---------------------------------------------------------------------------

fn builtin_go_term(args: Vec<Value>) -> Result<Value> {
    let go_id = require_str(&args[0], "go_term")?;
    let term = api_call(&format!("go_term(\"{go_id}\")"), || GO.with(|c| c.term(go_id)))?;
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Str(term.id));
    map.insert("name".to_string(), Value::Str(term.name));
    map.insert("aspect".to_string(), Value::Str(term.aspect));
    map.insert("definition".to_string(), Value::Str(term.definition));
    map.insert("is_obsolete".to_string(), Value::Bool(term.is_obsolete));
    Ok(Value::Record(map))
}

fn builtin_go_annotations(args: Vec<Value>) -> Result<Value> {
    let gene = require_str(&args[0], "go_annotations")?;
    let limit = if args.len() > 1 {
        require_int(&args[1], "go_annotations")? as usize
    } else {
        25
    };
    let annotations = api_call(&format!("go_annotations(\"{gene}\")"), || GO.with(|c| c.annotations(gene, limit)))?;
    let items: Vec<Value> = annotations
        .into_iter()
        .map(|a| {
            let mut map = HashMap::new();
            map.insert("go_id".to_string(), Value::Str(a.go_id));
            map.insert("go_name".to_string(), Value::Str(a.go_name));
            map.insert("aspect".to_string(), Value::Str(a.aspect));
            map.insert("evidence".to_string(), Value::Str(a.evidence));
            map.insert("gene_product_id".to_string(), Value::Str(a.gene_product_id));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_go_traversal(args: Vec<Value>, method: &str) -> Result<Value> {
    let go_id = require_str(&args[0], &format!("go_{method}"))?;
    let terms = match method {
        "children" => api_call(&format!("go_children(\"{go_id}\")"), || GO.with(|c| c.children(go_id)))?,
        "parents" => api_call(&format!("go_parents(\"{go_id}\")"), || GO.with(|c| c.parents(go_id)))?,
        "ancestors" => api_call(&format!("go_ancestors(\"{go_id}\")"), || GO.with(|c| c.ancestors(go_id)))?,
        "descendants" => api_call(&format!("go_descendants(\"{go_id}\")"), || GO.with(|c| c.descendants(go_id)))?,
        _ => unreachable!(),
    };
    let items: Vec<Value> = terms
        .into_iter()
        .map(|t| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(t.id));
            map.insert("name".to_string(), Value::Str(t.name));
            map.insert("aspect".to_string(), Value::Str(t.aspect));
            map.insert("definition".to_string(), Value::Str(t.definition));
            map.insert("is_obsolete".to_string(), Value::Bool(t.is_obsolete));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// COSMIC builtins
// ---------------------------------------------------------------------------

fn builtin_cosmic_gene(args: Vec<Value>) -> Result<Value> {
    let gene = require_str(&args[0], "cosmic_gene")?;
    let mutations = with_cosmic(|c| c.search_gene(gene))?;
    let items: Vec<Value> = mutations
        .into_iter()
        .map(|m| {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Value::Str(m.id));
            map.insert("gene".to_string(), Value::Str(m.gene));
            map.insert("cds".to_string(), Value::Str(m.cds));
            map.insert("aa".to_string(), Value::Str(m.aa));
            map.insert("primary_site".to_string(), Value::Str(m.primary_site));
            map.insert("mutation_type".to_string(), Value::Str(m.mutation_type));
            map.insert("count".to_string(), Value::Int(m.count as i64));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// NCBI Datasets builtins
// ---------------------------------------------------------------------------

fn builtin_datasets_gene(args: Vec<Value>) -> Result<Value> {
    let symbol = require_str(&args[0], "datasets_gene")?;
    let taxon = if args.len() > 1 {
        require_str(&args[1], "datasets_gene")?
    } else {
        "human"
    };
    let genes = api_call(
        &format!("datasets_gene(\"{symbol}\")"),
        || NCBI_DATASETS.with(|c| c.gene_by_symbol(&[symbol], taxon)),
    )?;
    let items: Vec<Value> = genes
        .into_iter()
        .map(|g| {
            let mut map = HashMap::new();
            map.insert("gene_id".to_string(), Value::Str(g.gene_id));
            map.insert("symbol".to_string(), Value::Str(g.symbol));
            map.insert("description".to_string(), Value::Str(g.description));
            map.insert("taxname".to_string(), Value::Str(g.taxname));
            map.insert("chromosome".to_string(), Value::Str(g.chromosome));
            map.insert("gene_type".to_string(), Value::Str(g.gene_type));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// BioContainers builtins
// ---------------------------------------------------------------------------

fn bc_image_uri(img: &bl_apis::biocontainers::ContainerImage) -> String {
    if img.registry_host.is_empty() {
        img.image_name.clone()
    } else {
        format!("{}/{}", img.registry_host, img.image_name)
    }
}

fn bc_tool_to_summary(t: &bl_apis::biocontainers::Tool) -> Value {
    let mut rec = HashMap::new();
    rec.insert("name".into(), Value::Str(t.name.clone()));
    rec.insert("description".into(), Value::Str(t.description.clone()));
    rec.insert("organization".into(), Value::Str(t.organization.clone()));
    rec.insert("version_count".into(), Value::Int(t.versions.len() as i64));
    let latest = t.versions.first();
    rec.insert(
        "latest_version".into(),
        latest
            .map(|v| Value::Str(v.name.clone()))
            .unwrap_or(Value::Nil),
    );
    let latest_image = latest
        .and_then(|v| v.images.first())
        .map(|img| Value::Str(bc_image_uri(img)))
        .unwrap_or(Value::Nil);
    rec.insert("latest_image".into(), latest_image);
    Value::Record(rec)
}

fn builtin_biocontainers_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "biocontainers_search")?;
    let limit = if args.len() > 1 {
        require_int(&args[1], "biocontainers_search")? as usize
    } else {
        25
    };
    let tools = api_call(
        &format!("biocontainers_search(\"{query}\")"),
        || BIOCONTAINERS.with(|c| c.search(query, limit)),
    )?;
    let items: Vec<Value> = tools.iter().map(bc_tool_to_summary).collect();
    Ok(Value::List(items))
}

fn builtin_biocontainers_popular(args: Vec<Value>) -> Result<Value> {
    let limit = if !args.is_empty() {
        require_int(&args[0], "biocontainers_popular")? as usize
    } else {
        20
    };
    let tools = api_call("biocontainers_popular()", || {
        BIOCONTAINERS.with(|c| c.popular(limit))
    })?;
    let items: Vec<Value> = tools.iter().map(bc_tool_to_summary).collect();
    Ok(Value::List(items))
}

fn builtin_biocontainers_info(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "biocontainers_info")?;
    let tool = api_call(&format!("biocontainers_info(\"{name}\")"), || {
        BIOCONTAINERS.with(|c| c.tool_info(name))
    })?;
    let mut rec = HashMap::new();
    rec.insert("name".into(), Value::Str(tool.name.clone()));
    rec.insert("description".into(), Value::Str(tool.description.clone()));
    rec.insert("organization".into(), Value::Str(tool.organization.clone()));
    rec.insert(
        "aliases".into(),
        Value::List(tool.aliases.iter().map(|a| Value::Str(a.clone())).collect()),
    );
    let versions: Vec<Value> = tool
        .versions
        .iter()
        .map(|v| {
            let mut vm = HashMap::new();
            vm.insert("version".into(), Value::Str(v.name.clone()));
            let images: Vec<Value> = v
                .images
                .iter()
                .map(|img| {
                    let mut im = HashMap::new();
                    im.insert("registry".into(), Value::Str(img.registry_host.clone()));
                    im.insert("image".into(), Value::Str(img.image_name.clone()));
                    im.insert("type".into(), Value::Str(img.image_type.clone()));
                    im.insert(
                        "size".into(),
                        img.size
                            .map(|s| Value::Int(s as i64))
                            .unwrap_or(Value::Nil),
                    );
                    Value::Record(im)
                })
                .collect();
            vm.insert("images".into(), Value::List(images));
            Value::Record(vm)
        })
        .collect();
    rec.insert("versions".into(), Value::List(versions));
    Ok(Value::Record(rec))
}

fn builtin_biocontainers_versions(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "biocontainers_versions")?;
    let versions = api_call(&format!("biocontainers_versions(\"{name}\")"), || {
        BIOCONTAINERS.with(|c| c.tool_versions(name))
    })?;
    let items: Vec<Value> = versions
        .iter()
        .map(|v| {
            let mut vm = HashMap::new();
            vm.insert("version".into(), Value::Str(v.name.clone()));
            let images: Vec<Value> = v
                .images
                .iter()
                .map(|img| Value::Str(bc_image_uri(img)))
                .collect();
            vm.insert("images".into(), Value::List(images));
            Value::Record(vm)
        })
        .collect();
    Ok(Value::List(items))
}

// ---------------------------------------------------------------------------
// nf-core builtins
// ---------------------------------------------------------------------------

fn pipeline_summary_to_value(p: &bl_apis::nfcore::PipelineSummary) -> Value {
    let mut rec = HashMap::new();
    rec.insert("name".into(), Value::Str(p.name.clone()));
    rec.insert("description".into(), Value::Str(p.description.clone()));
    rec.insert("stars".into(), Value::Int(p.stargazers_count as i64));
    rec.insert(
        "topics".into(),
        Value::List(p.topics.iter().map(|t| Value::Str(t.clone())).collect()),
    );
    let latest = p
        .releases
        .first()
        .map(|r| Value::Str(r.tag_name.clone()))
        .unwrap_or(Value::Nil);
    rec.insert("latest_release".into(), latest);
    Value::Record(rec)
}

fn builtin_nfcore_list(args: Vec<Value>) -> Result<Value> {
    let sort_by = if !args.is_empty() {
        require_str(&args[0], "nfcore_list")?
    } else {
        "stars"
    };
    let limit = if args.len() > 1 {
        require_int(&args[1], "nfcore_list")? as usize
    } else {
        0 // 0 means no limit
    };

    let mut pipelines = api_call("nfcore_list()", || {
        NFCORE.with(|c| c.list_pipelines())
    })?;

    match sort_by {
        "name" => pipelines.sort_by(|a, b| a.name.cmp(&b.name)),
        "release" => pipelines.sort_by(|a, b| {
            let a_date = a
                .releases
                .first()
                .map(|r| r.published_at.as_str())
                .unwrap_or("");
            let b_date = b
                .releases
                .first()
                .map(|r| r.published_at.as_str())
                .unwrap_or("");
            b_date.cmp(a_date)
        }),
        _ => pipelines.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count)),
    }

    if limit > 0 {
        pipelines.truncate(limit);
    }

    let items: Vec<Value> = pipelines.iter().map(pipeline_summary_to_value).collect();
    Ok(Value::List(items))
}

fn builtin_nfcore_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "nfcore_search")?;
    let limit = if args.len() > 1 {
        require_int(&args[1], "nfcore_search")? as usize
    } else {
        0
    };

    let mut pipelines = api_call(
        &format!("nfcore_search(\"{query}\")"),
        || NFCORE.with(|c| c.search_pipelines(query)),
    )?;

    // Default sort by stars descending
    pipelines.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count));

    if limit > 0 {
        pipelines.truncate(limit);
    }

    let items: Vec<Value> = pipelines.iter().map(pipeline_summary_to_value).collect();
    Ok(Value::List(items))
}

fn builtin_nfcore_info(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "nfcore_info")?;
    let detail = api_call(
        &format!("nfcore_info(\"{name}\")"),
        || NFCORE.with(|c| c.pipeline_info(name)),
    )?;

    let mut map = HashMap::new();
    map.insert("name".into(), Value::Str(detail.name));
    map.insert("full_name".into(), Value::Str(detail.full_name));
    map.insert("description".into(), Value::Str(detail.description));
    map.insert("stars".into(), Value::Int(detail.stargazers_count as i64));
    map.insert("url".into(), Value::Str(detail.html_url));
    map.insert(
        "license".into(),
        detail
            .license
            .map(|l| Value::Str(l.spdx_id))
            .unwrap_or(Value::Nil),
    );
    map.insert(
        "topics".into(),
        Value::List(detail.topics.into_iter().map(Value::Str).collect()),
    );
    map.insert("open_issues".into(), Value::Int(detail.open_issues_count as i64));
    map.insert("created".into(), Value::Str(detail.created_at));
    map.insert("updated".into(), Value::Str(detail.updated_at));
    Ok(Value::Record(map))
}

fn builtin_nfcore_releases(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "nfcore_releases")?;
    let releases = api_call(
        &format!("nfcore_releases(\"{name}\")"),
        || NFCORE.with(|c| c.pipeline_releases(name)),
    )?;

    let items: Vec<Value> = releases
        .into_iter()
        .map(|r| {
            let mut map = HashMap::new();
            map.insert("tag".into(), Value::Str(r.tag_name));
            map.insert("published_at".into(), Value::Str(r.published_at));
            Value::Record(map)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_nfcore_params(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "nfcore_params")?;
    let params = api_call(
        &format!("nfcore_params(\"{name}\")"),
        || NFCORE.with(|c| c.pipeline_params(name)),
    )?;

    // Parse the nextflow_schema.json structure.
    // The schema has "definitions" (or "defs"/"$defs") containing parameter groups.
    // Each group has "properties" mapping param names to their schema.
    let schema = &params.schema;

    let defs = schema
        .get("definitions")
        .or_else(|| schema.get("defs"))
        .or_else(|| schema.get("$defs"));

    let Some(defs) = defs else {
        // No definitions found — return the raw schema as a nested Record
        return Ok(json_to_value(schema));
    };

    let Some(defs_obj) = defs.as_object() else {
        return Ok(json_to_value(schema));
    };

    let mut groups = HashMap::new();
    for (group_name, group_val) in defs_obj {
        let Some(props) = group_val.get("properties").and_then(|p| p.as_object()) else {
            continue;
        };
        let mut param_list = Vec::new();
        for (param_name, param_val) in props {
            let mut rec = HashMap::new();
            rec.insert("name".into(), Value::Str(param_name.clone()));
            rec.insert(
                "type".into(),
                Value::Str(
                    param_val["type"]
                        .as_str()
                        .unwrap_or("string")
                        .to_string(),
                ),
            );
            rec.insert(
                "description".into(),
                Value::Str(
                    param_val["description"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                ),
            );
            rec.insert("default".into(), json_to_value(&param_val["default"]));
            param_list.push(Value::Record(rec));
        }
        groups.insert(group_name.clone(), Value::List(param_list));
    }
    Ok(Value::Record(groups))
}

// ---------------------------------------------------------------------------
// Galaxy ToolShed builtins
// ---------------------------------------------------------------------------

fn galaxy_repo_to_value(r: &bl_apis::galaxy::Repository) -> Value {
    let mut rec = HashMap::new();
    rec.insert("name".into(), Value::Str(r.name.clone()));
    rec.insert("owner".into(), Value::Str(r.owner.clone()));
    rec.insert("description".into(), Value::Str(r.description.clone()));
    rec.insert("downloads".into(), Value::Int(r.times_downloaded as i64));
    rec.insert("approved".into(), Value::Str(r.approved.clone()));
    rec.insert("last_updated".into(), Value::Str(r.last_updated.clone()));
    let url = if r.remote_repository_url.is_empty() {
        format!("https://toolshed.g2.bx.psu.edu/view/{}/{}", r.owner, r.name)
    } else {
        r.remote_repository_url.clone()
    };
    rec.insert("url".into(), Value::Str(url));
    Value::Record(rec)
}

fn builtin_galaxy_search(args: Vec<Value>) -> Result<Value> {
    let query = require_str(&args[0], "galaxy_search")?;
    let limit = if args.len() > 1 {
        require_int(&args[1], "galaxy_search")? as usize
    } else {
        25
    };
    let repos = api_call(
        &format!("galaxy_search(\"{query}\")"),
        || GALAXY.with(|c| c.search(query, limit)),
    )?;
    let items: Vec<Value> = repos.iter().map(galaxy_repo_to_value).collect();
    Ok(Value::List(items))
}

fn builtin_galaxy_popular(args: Vec<Value>) -> Result<Value> {
    let limit = if !args.is_empty() {
        require_int(&args[0], "galaxy_popular")? as usize
    } else {
        20
    };
    let repos = api_call("galaxy_popular()", || {
        GALAXY.with(|c| c.popular(limit))
    })?;
    let items: Vec<Value> = repos.iter().map(galaxy_repo_to_value).collect();
    Ok(Value::List(items))
}

fn builtin_galaxy_categories(_args: Vec<Value>) -> Result<Value> {
    let cats = api_call("galaxy_categories()", || {
        GALAXY.with(|c| c.categories())
    })?;
    let items: Vec<Value> = cats
        .iter()
        .map(|cat| {
            let mut rec = HashMap::new();
            rec.insert("name".into(), Value::Str(cat.name.clone()));
            rec.insert("description".into(), Value::Str(cat.description.clone()));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

fn builtin_galaxy_tool(args: Vec<Value>) -> Result<Value> {
    let owner = require_str(&args[0], "galaxy_tool")?;
    let name = require_str(&args[1], "galaxy_tool")?;
    let repo = api_call(
        &format!("galaxy_tool(\"{owner}\", \"{name}\")"),
        || GALAXY.with(|c| c.repository_info(owner, name)),
    )?;
    let mut rec = HashMap::new();
    rec.insert("name".into(), Value::Str(repo.name.clone()));
    rec.insert("owner".into(), Value::Str(repo.owner.clone()));
    rec.insert("description".into(), Value::Str(repo.description.clone()));
    rec.insert("downloads".into(), Value::Int(repo.times_downloaded as i64));
    rec.insert("approved".into(), Value::Str(repo.approved.clone()));
    let url = if repo.remote_repository_url.is_empty() {
        format!("https://toolshed.g2.bx.psu.edu/view/{}/{}", repo.owner, repo.name)
    } else {
        repo.remote_repository_url.clone()
    };
    rec.insert("url".into(), Value::Str(url));
    rec.insert("created".into(), Value::Str(repo.create_time.clone()));
    rec.insert("last_updated".into(), Value::Str(repo.last_updated.clone()));
    Ok(Value::Record(rec))
}

// ---------------------------------------------------------------------------
// Config: api_endpoints()
// ---------------------------------------------------------------------------

fn builtin_api_endpoints() -> Result<Value> {
    use bl_apis::config::resolve_url;
    let mut rec = HashMap::new();
    rec.insert("ncbi".into(), Value::Str(resolve_url("ncbi", "https://eutils.ncbi.nlm.nih.gov/entrez/eutils")));
    rec.insert("ensembl".into(), Value::Str(resolve_url("ensembl", "https://rest.ensembl.org")));
    rec.insert("uniprot".into(), Value::Str(resolve_url("uniprot", "https://rest.uniprot.org")));
    rec.insert("ucsc".into(), Value::Str(resolve_url("ucsc", "https://api.genome.ucsc.edu")));
    rec.insert("biomart".into(), Value::Str(resolve_url("biomart", "https://www.ensembl.org/biomart/martservice")));
    rec.insert("kegg".into(), Value::Str(resolve_url("kegg", "https://rest.kegg.jp")));
    rec.insert("string_db".into(), Value::Str(resolve_url("string_db", "https://string-db.org/api")));
    rec.insert("pdb_data".into(), Value::Str(resolve_url("pdb_data", "https://data.rcsb.org/rest/v1/core")));
    rec.insert("pdb_search".into(), Value::Str(resolve_url("pdb_search", "https://search.rcsb.org/rcsbsearch/v2/query")));
    rec.insert("reactome".into(), Value::Str(resolve_url("reactome", "https://reactome.org/ContentService")));
    rec.insert("go".into(), Value::Str(resolve_url("go", "https://www.ebi.ac.uk/QuickGO/services")));
    rec.insert("cosmic".into(), Value::Str(resolve_url("cosmic", "https://cancer.sanger.ac.uk/api/v1")));
    rec.insert("ncbi_datasets".into(), Value::Str(resolve_url("ncbi_datasets", "https://api.ncbi.nlm.nih.gov/datasets/v2")));
    rec.insert("nfcore_catalog".into(), Value::Str(resolve_url("nfcore_catalog", "https://nf-co.re/pipelines.json")));
    rec.insert("nfcore_github".into(), Value::Str(resolve_url("nfcore_github", "https://api.github.com")));
    rec.insert("biocontainers".into(), Value::Str(resolve_url("biocontainers", "https://api.biocontainers.pro/ga4gh/trs/v2")));
    rec.insert("galaxy_toolshed".into(), Value::Str(resolve_url("galaxy_toolshed", "https://toolshed.g2.bx.psu.edu")));
    Ok(Value::Record(rec))
}

// ---------------------------------------------------------------------------
// Utility builtins
// ---------------------------------------------------------------------------

/// `bio_icon(name)` — Return BioIcons search URL and category info for an icon keyword.
fn builtin_bio_icon(args: Vec<Value>) -> Result<Value> {
    let name = require_str(&args[0], "bio_icon")?;
    let encoded = name.replace(' ', "+");
    let url = format!("https://bioicons.com/?search={encoded}");

    let categories = vec![
        "cell_biology",
        "chemistry",
        "genetics",
        "microbiology",
        "molecular_biology",
        "neuroscience",
        "oncology",
        "organisms",
        "physiology",
        "safety_symbols",
        "tissues",
        "equipment",
    ];

    let mut map = HashMap::new();
    map.insert("name".into(), Value::Str(name.to_string()));
    map.insert("url".into(), Value::Str(url));
    map.insert(
        "categories".into(),
        Value::List(categories.into_iter().map(|c| Value::Str(c.into())).collect()),
    );
    map.insert(
        "tip".into(),
        Value::Str("Visit the URL to browse and download SVG icons".into()),
    );
    Ok(Value::Record(map))
}

/// `paper_score(query, db?)` — Search PubMed and return study metadata with quality indicators.
///
/// If the first argument looks like a numeric PMID, fetches that paper directly.
/// Otherwise searches PubMed for the query and summarizes the top result.
fn builtin_paper_score(args: Vec<Value>) -> Result<Value> {
    // Determine PMID: either directly given as Int/numeric string, or search first
    let pmid = match &args[0] {
        Value::Int(n) => n.to_string(),
        Value::Str(s) if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() => s.clone(),
        Value::Str(query) => {
            // Search PubMed for the query, take the top result
            let db = if args.len() > 1 {
                require_str(&args[1], "paper_score")?
            } else {
                "pubmed"
            };
            let result = api_call(&format!("paper_score: esearch({db}, \"{query}\")"), || {
                NCBI.with(|c| c.esearch(db, query, 1))
            })?;
            if result.ids.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("paper_score(): no results found for \"{query}\""),
                    None,
                ));
            }
            result.ids.into_iter().next().unwrap()
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "paper_score() requires Int (PMID) or Str (query), got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };

    // Fetch summary for this PMID
    let summaries = api_call(&format!("paper_score: esummary(pubmed, {pmid})"), || {
        NCBI.with(|c| c.esummary("pubmed", &[pmid.as_str()]))
    })?;

    if summaries.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("paper_score(): no summary found for PMID {pmid}"),
            None,
        ));
    }

    let doc = &summaries[0];
    let fields = &doc.fields;

    // Extract metadata
    let title = fields
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source = fields
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let pub_date = fields
        .get("pubdate")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let doi = fields
        .get("elocationid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Authors list
    let authors: Vec<Value> = fields
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                .map(|s| Value::Str(s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Publication types
    let pub_types: Vec<Value> = fields
        .get("pubtype")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| Value::Str(s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Quality indicators: scan title and pub_types for study design keywords
    let title_lower = title.to_lowercase();
    let pub_types_lower: Vec<String> = pub_types
        .iter()
        .filter_map(|v| {
            if let Value::Str(s) = v {
                Some(s.to_lowercase())
            } else {
                None
            }
        })
        .collect();
    let combined = format!("{} {}", title_lower, pub_types_lower.join(" "));

    let quality_keywords = [
        "meta-analysis",
        "systematic review",
        "randomized",
        "randomised",
        "cohort",
        "case-control",
        "double-blind",
        "placebo-controlled",
        "prospective",
        "retrospective",
        "cross-sectional",
        "longitudinal",
        "multicenter",
        "multicentre",
        "genome-wide",
        "gwas",
        "clinical trial",
        "sample size",
        "n =",
    ];

    let indicators: Vec<Value> = quality_keywords
        .iter()
        .filter(|kw| combined.contains(**kw))
        .map(|kw| Value::Str(kw.to_string()))
        .collect();

    let mut map = HashMap::new();
    map.insert("pmid".into(), Value::Str(pmid));
    map.insert("title".into(), Value::Str(title));
    map.insert("journal".into(), Value::Str(source));
    map.insert("pub_date".into(), Value::Str(pub_date));
    map.insert("doi".into(), Value::Str(doi));
    map.insert("authors".into(), Value::List(authors));
    map.insert("pub_types".into(), Value::List(pub_types));
    map.insert("quality_indicators".into(), Value::List(indicators));
    Ok(Value::Record(map))
}
