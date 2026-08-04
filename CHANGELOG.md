# Changelog

All notable changes to BioLang will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [1.1.0] - 2026-08-04

### Added
- **Rosalind Stronghold complete (105/105)**, bringing all four tracks to
  278 problems — Algorithmic Heights 34, Armory 15, Stronghold 105,
  Textbook 124. 277 are solved and 276 carry assertions. The exceptions:
  Armory MEME is `partial`, since it finds exact shared substrings rather than
  a position-weight motif, and 4 problems fetch from remote databases, so they
  are asserted but excluded from the hermetic CI gate. 274 are additionally
  verified to run in the browser against the real WASM module.
- A short context note on every Rosalind example saying what the problem is for,
  rather than only how it is solved.
- `reversal_distance(a, b)` and `sorting_reversals(a, b)` — bidirectional BFS
  over permutations.
- `chars(text)` — splits a string into characters. `split(text, "")` was the
  workaround and it yields spurious empty strings at both ends.
- Guide to embedding the WASM module in other applications, at
  `/docs/tools/embedding.html`. Every claim in it is checked against the module
  actually shipped.
- Container image for the benchmark suite (`bench/Dockerfile`) carrying BioLang,
  Python and R/Bioconductor together, so the three are measured by one kernel
  rather than compared across machines.

### Changed
- `edit_distance` now uses Myers' bit-parallel algorithm, which computes 64
  cells of the DP table per word operation. The previous quadratic
  implementation is kept as `edit_distance_dp` and is the oracle the new one is
  tested against, over 2352 random pairs at lengths straddling the word
  boundary (63, 64, 65, 127, 128, 129, 257).
- The published benchmark figures were re-measured on this release. They had
  been labelled v0.3.0, were actually from 0.2.1, and were understating current
  performance: ENCODE overlap 7.1x → 17.0x, protein k-mers 7.0x → 14.0x. See
  #4 for the five separate faults that had kept the suite from being re-run.
- The Playwright Run-button test now discovers all 45 documentation section
  pages instead of three hardcoded ones, and reads `network = true` from pack
  manifests so a legitimately disabled button is not reported as a failure.

### Fixed
- A `@compile`-decorated function reported "Function is not callable" the moment
  it was called by name. Two call-dispatch paths exist and only one handled
  `CompiledClosure`; the unhandled one is the path every ordinary named call
  takes. The bytecode feature is off by default, so nothing had exercised it.
- `eulerian_cycle` accepted a walk that did not close, and `eulerian_path`
  missed the wrap-around edge because it inspected the path with `windows(2)`.
- `.sh` files were committed with CRLF from a Windows checkout, which makes them
  unrunnable on Linux — the shebang resolves to `bash\r`. `.gitattributes` now
  pins them to LF.
- `benchmarks/run_all.sh` never wrote to `results/latest/`, the directory the
  website quotes, so re-running the suite could not change any published number.

## [1.0.0] - 2026-08-03

### Added
- **Rosalind Textbook Track complete (124/124)** and the language work it
  needed: hidden Markov models (Viterbi, forward, forward-backward, Baum-Welch,
  profile HMMs with silent deletion states), Eulerian cycles and paths via
  Hierholzer's algorithm, cyclopeptide sequencing and spectral convolution,
  suffix arrays by prefix doubling with Kasai LCP, and motif profile builtins.
- Rosalind example packs for the Armory and Algorithmic Heights tracks, with
  the pack manifest format (`packs/<id>/pack.toml`) and the CI gates that verify
  every example still runs.

## [0.3.1] - 2026-03-16

### Added
- BioGist extension v1.1 — 18 entity types (clinical trials, funding,
  repositories, p-values), 500+ drugs, 450+ cell lines, 43 species; inline
  PubMed search, database links, compare tabs, co-occurrence matrix.

## [0.3.0] - 2026-03-12

### Added
- Benchmark suite with native table builtins and I/O optimizations.
- Correctness validation across 9 tasks, including R/Bioconductor comparisons.
- WASM inline SVG rendering.
- BLViewer Chrome extension; biostatistics book.

### Fixed
- Parser handling of `if`/`else` across newlines.
- Builtin count corrected to 750+; plugin test isolation.

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
- **750+ builtins** covering:
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
