# Changelog

All notable changes to BioLang will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.2.1] - 2026-03-09

### Added
- `node_count()` and `edge_count()` graph builtins
- Tutorials: Knowledge Graphs, Enrichment Analysis, LLM Chat, Notebooks
- Tutorial `.bl` scripts in `examples/tutorials/` for all documentation chapters
- CHANGELOG.md for release tracking

### Fixed
- API return type mismatches across 90+ example scripts, website docs, and book chapters
- `string_network()` examples now pass List argument (was incorrectly passing single String)
- `ncbi_search()` documented as returning `List[Str]` (was incorrectly shown as Record with `.ids`/`.count`)
- `ncbi_fetch()`/`ncbi_summary()` examples use correct ids-first argument order
- `uniprot_entry()` field names corrected: `.name`, `.sequence_length`, `.gene_names`
- `ensembl_vep()` examples index into returned List with `[0]`
- `kegg_find()` field names corrected: `.id`, `.description`
- Fabricated function references removed from tutorials and docs
- Added `is_record()` guards for polymorphic `ncbi_gene()` returns
- REPL history for multi-line inputs

---

## [0.2.0] - 2026-03-09

### Added
- **Literate notebooks** — mixed markdown + code cells, export to HTML/PDF
- **Knowledge graphs** — `graph()`, `add_node()`, `add_edge()`, `shortest_path()`, `connected_components()`
- **PDB enrichment** — `pdb_entry()`, `pdb_search()`, `pdb_sequence()` builtins
- **Enrichment analysis** — `go_enrichment()`, `kegg_enrichment()`, `pathway_enrichment()`
- **Self-update** — `bl update` checks GitHub releases and updates in-place
- **IUPAC validation** for bio literals (`dna"..."`, `rna"..."`, `protein"..."`) and type constructors
- CARGO_PKG_VERSION used for CLI and REPL version strings (no more hardcoded versions)

---

## [0.1.0] - 2026-03-08

### Added
- **Core language**: pipe-first syntax (`|>`), lambdas (`|x| expr`), pattern matching
- **Bio literals**: `dna"ATCG"`, `rna"AUGC"`, `protein"MKT..."` with compile-time type
- **270+ builtins** covering:
  - Sequence ops: `complement`, `reverse_complement`, `translate`, `gc_content`, `kmer_count`
  - File I/O: `read_fasta`, `read_fastq`, `read_vcf`, `read_bed`, `read_gff`, `write_fasta`
  - Statistics: `mean`, `median`, `sd`, `cor`, `t_test`, `chi_sq`, `p_adjust`, `anova`
  - Linear algebra: `matrix`, `dot`, `transpose`, `solve`, `eigenvalues`
  - Tables: `select`, `filter`, `mutate`, `group_by`, `summarize`, `join`, `pivot`
  - Intervals: `interval_tree`, `query_overlaps`, `query_nearest`, `coverage`
  - Plotting: `plot`, `histogram`, `scatter`, `heatmap`, `volcano_plot`, `genome_track`
  - Parallel: `par_map`, `par_filter`
  - Streams: lazy evaluation with `stream`, `take`, `collect`
- **12 API clients**: NCBI, Ensembl, UniProt, UCSC, BioMart, KEGG, STRING, PDB, Reactome, GO, COSMIC, NCBI Datasets
- **Plugin system**: subprocess JSON protocol, supports Python/Deno/R/native plugins
- **REPL**: `:env`, `:reset`, `:load`, `:save`, `:time`, `:type`, `:plugins`, `:profile`
- **LSP**: diagnostics, completion, hover (via `bl-lsp` / `bl lsp`)
- **CLI**: `bl run`, `bl repl`, `bl lsp`, `bl init`, `bl plugins`, `bl add`, `bl remove`
- **Bytecode compiler** (experimental) and **JIT** (Cranelift, feature-gated)
- **mdBook documentation** — 18 chapters covering language, APIs, and workflows
- **Website** with getting started guide, API docs, tutorials, and examples
- **GitHub Actions** — CI, release builds (Linux/macOS/Windows), GitHub Pages deployment
- **bio-core** shared types: `BioSequence`, `GenomicInterval`, `Variant`, `Gene`, `Genome`, `AlignedRead`

---

[Unreleased]: https://github.com/oriclabs/biolang/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/oriclabs/biolang/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/oriclabs/biolang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oriclabs/biolang/releases/tag/v0.1.0
