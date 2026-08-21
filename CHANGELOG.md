# Changelog

All notable changes to BioLang will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- `ecdf_plot(list, opts?)` and `density_plot(list, opts?)`. Both show a
  distribution without a histogram's bin width deciding what it looks like: the
  ECDF has no smoothing parameter at all and is drawn as the step function it
  is, and the density's bandwidth is stated rather than implied by where the bin
  edges happened to fall. The default bandwidth is Silverman's rule computed the
  way R's `bw.nrd0` computes it, fallback chain included, and is checked against
  R 4.6.1 on five cases including the tied and constant columns where every
  measure of spread is zero.

### Fixed
- Plot builtins hardcoded their axis labels. `histogram` drew `"Value"` and
  `"Count"` and accepted an `xlabel` option it then ignored, so a figure had to
  string-replace the rendered SVG to say what had been measured. `plot`,
  `histogram`, `volcano`, `ma_plot` and `genome_track` now honour `xlabel` and
  `ylabel`.

## [1.4.0] - 2026-08-16

### Added
- **Guided, explainable statistics** (`packages/statistics`). Around forty
  functions — `explore`, `compare`, `relationship`, `scan`, `report`,
  `linear_diagnostics`, `distribution_clues`, `associations` and the rest —
  that return the calculated facts, clues, alternatives and limitations as
  ordinary records. They never delete outliers, transform values, or pick a
  test: `input_modified` and `model_selected` are asserted fields, not
  promises in a README.
- **GLM, random-intercept and Cox diagnostics**, validated against
  `stats::glm`, `nlme::lme` (REML) and `survival::coxph` (Breslow ties) on
  `mtcars`, `warpbreaks`, `ChickWeight` and `survival::lung`. The external
  oracle now compares 147 scale-sensitive metrics against R.
- **Frozen model conformance** (`crates/bl-runtime/tests/stats_model_conformance.rs`).
  The R oracle needs R and is run by hand, so it could not fail a refactor,
  and every other automated check on these fitters asserted shape rather than
  arithmetic. R's answers are now pinned for fixtures small enough to inline,
  so they run under `cargo test` anywhere without R and without redistributing
  an R dataset. Mutating the IRLS tolerance, the Cox information matrix, the
  reported AIC or the REML bracket each fails a pinned value.
- `glm_diagnostics` reports `converged` and `iterations`. IRLS returned its
  final iterate either way, so an unconverged fit was indistinguishable from a
  converged one; R's `glm()` warns and sets `$converged`. A fit that did not
  settle is now disclosed as a blocking `fit_not_converged` issue and marked in
  the rendered summary.
- **Practical Biostatistics in 30 Days**, with an audit that parses all 318
  BioLang blocks across the 30 chapters on every run.
- **Modern Statistics for Modern Biology**, a BioLang companion to Holmes &
  Huber, and the **HBC single-cell course** as its own book worked on the
  course's own data rather than a synthetic stand-in.
- Seurat-style single-cell plots, matrix interchange for external SCTransform
  providers, and an exact optional Seurat integration boundary.
- **Seurat conformance fixtures in core CI.** The parity harness lives in
  `biolang-workflows`, so this repository could not run it, and two of its
  results were bit-identical equivalences with Seurat's MIT C++ — `ComputeSNN`
  and `LogNorm` — that nothing here would have noticed drifting. Reference
  outputs are checked in, so the guarantee holds with no R, no network and no
  oracle. Both were mutation-tested: flipping the SNN prune comparison moves
  the edge count from 9,085 to 9,934 and fails.
- An internal link checker that fails CI on a broken site link — 8,205 links
  across 367 tracked pages in about a second.

### Changed
- **`find_all_markers` now matches Seurat 5.5.1 exactly.** Differential-tested
  on a 3-cluster fixture, BioLang returned 156 markers where Seurat returned
  72. Four separate causes, none of them the threshold mismatch expected:
  Seurat adds the pseudocount to the *sum* rather than the mean; `return.thresh`
  discards tests above p = 0.01; multiple testing is Bonferroni over every gene
  in the assay, not Benjamini-Hochberg; and the Wilcoxon test was missing tie
  and continuity corrections. Now `avg_log2FC` 4.9e-15, `p_val` 1.4e-07,
  `p_val_adj` 1.1e-05, membership identical.
- `mann_whitney_u` takes the continuity correction as a choice rather than
  assuming one. R's `wilcox.test` applies it and Scanpy does not, so it is a
  genuine fork; `mann_whitney_test` keeps the existing behaviour.
- **lang.bio is published solely by `oriclabs/biolang-website`.** Both
  repositories deployed Pages, so a deploy from here could only ever publish
  the part of the site it could see. `books/hbc-scrnaseq-validated` is the
  proof: built from the workflows repository, it was absent from every deploy
  this one made. The build steps stay as verification, without the deploy.
- The site's books page lists only the language reference; the practical books
  and courses are hidden rather than removed, and their URLs keep working.
- The count matrix is no longer copied on every value clone, and SCTransform
  no longer pays for its output three times.

### Fixed
- Three stale site links, none of which looked broken: a missing
  `js/components.js` 404'd on every load of three documentation pages that
  already loaded the script doing the work, and two book chapters pointed at
  `lang.bio/playground` rather than `playground.html`.
- The sitemap advertised `studio.html` and `docs/tools/studio-help.html`.
  Neither is built — the pages are `workbench.html` and `workbench-help.html`,
  and the sitemap was the only place still carrying the Studio name.
- UMAP was missing its repulsion term, and axis ticks were drawn too close to
  tell apart.
- Exported notebooks escaped their figures instead of rendering them.
- Large scatters render as raster rather than one node per cell, and plot
  options are no longer discarded.
- Generated AnnData `.zarr` stores are no longer tracked. The example that
  reads `pbmc_synth.zarr` writes it first, so the committed store was a
  byproduct; it was tracked twice, and the ignore rule was anchored to the root
  so the copy under `packages/` never matched.

## [1.3.0] - 2026-08-06

### Added
- **The single-cell analysis path, end to end.** The Harvard Chan scRNA-seq
  curriculum goes from 10 of its 14 lessons represented to all 14:
  `variable_feature_plot` (mean vs dispersion), `dot_plot` (genes × clusters,
  area = detection, colour = z-score), `find_all_markers` (each cluster against
  the rest, Mann-Whitney), `harmony_integrate` (per-cluster batch correction)
  and `cca`. `find_all_markers` and `variable_feature_plot` share their
  selection logic with `highly_variable_genes` rather than reimplementing the
  ranking, so a figure cannot drift from the pipeline it illustrates.
- `violin_plot`, `elbow_plot` and `feature_plot` — the figures QC, dimension
  selection and marker inspection are read from. A boxplot draws five numbers
  and cannot show bimodality, which is the whole reason to reach for a violin.
- The **MSMB companion**: nine chapters of statistics worked in BioLang.
- PNG export, and `:paste` and `die()` in the REPL.
- **A gate that detects a committed browser runtime falling behind its
  source.** `check-generated` compared the two committed copies against each
  other, which passes when both are equally stale — the common case. The new
  check compares the module against the source two ways: its own
  `list_builtins()` against the crate, and a fingerprint of the sources it is
  compiled from. It found six builtins already missing from the browser, and a
  module whose manifest still read 1.1.0.

### Changed
- Scatter plots rasterise above 5,000 cells. One `<circle>` per cell is fine at
  PBMC3k's 2,700 and ruinous at atlas scale: a million cells measured 65.5 MB
  and 1,000,039 DOM nodes, now 37 KB and 39. Axes and labels stay vector.
- Variable genes are ranked against expression level rather than in absolute
  terms, and cluster plots carry enough colours to name their clusters.
- The pipe operator is cheaper, and deep recursion no longer kills the process.

### Fixed
- Harmony corrected each cluster in place, so later clusters regressed against
  data earlier ones had already moved. Clusters overlap, so the corrections
  compounded and the batch effect got *worse*, 2.52 → 3.55. Every cluster now
  regresses against one snapshot and the shifts are summed.
- `read_pdf` killed the process. `pdf_extract` unwraps its own parse failures,
  so a PDF it could not follow panicked rather than returning an error, taking
  any unsaved REPL or notebook work with it.
- Five statistics functions that returned confident wrong answers.
- Multi-line paste in the REPL.

## [1.2.0] - 2026-08-05

### Added
- **A table-driven one-liner correctness harness** (#20, #21, #22), recording
  what each language produced rather than only whether it matched (#23), with
  Python and R equivalents generated from the verified set and shown as tabs
  (#24). Correctness gets its own section on the home page (#25).
- f-string format specifiers.
- One-line installers, in the documentation that tells people how to install
  them (#18).

### Changed
- Studio renamed to Workbench.
- Notebooks export in a form that stays runnable, and the playground is shown
  to be working.
- Benchmarks re-measured on Windows after fixing a runner that was timing
  itself (#15).

### Fixed
- Six API clients that had drifted, found by testing them against the real
  APIs.
- Four features that were documented but did not work.
- A UTF-8 BOM at the start of a script is now accepted (#17).
- The documented install paths (#16), five broken links found by checking them
  rather than reading them (#12), hollow references on the equivalents page
  (#26), and every GitHub link pointed at an org that exists (#13).
- mdBook pinned so the Pages deploy stops failing on a rate limit (#11).
- The playground no longer ligates `|>`.
- Published counts corrected to match the code, with a gate on the ones that
  drift (#7, #8, #9).

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
