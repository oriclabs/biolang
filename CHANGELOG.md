# Changelog

All notable changes to BioLang will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- **Native contingency-table mosaic plots.** `mosaic_plot()` draws category
  rectangles whose area is exactly proportional to observed counts, while
  `mosaic_data()` exposes replayable counts, expected values, proportions,
  Pearson residuals, colours and rectangle boundaries. Count/row/column/total
  labels, residual shading, SVG, HTML/Canvas and terminal output share the same
  renderer-neutral specification.

- **Verified dataset registry client.** `bl data search`, `info`, `fetch`, and
  `path` consume versioned dataset/provider entries from the separate BioLang
  Registry. Downloads are explicit, size-bounded, SHA-256 verified, cached
  outside the executable, and activated atomically. Verified manifests are
  cached with their digest so warm fetches avoid registry round trips and
  `bl data path` works fully offline. Cache identity/version segments are
  traversal-safe, malformed registry entries are isolated, versions use one
  semantic ordering, and cross-origin redirects are refused. Manifests disclose
  source, licence, citation, access conditions, formats, and suggested BioLang
  readers.

- **Reproducible recorded CLI runs.** `bl run --record` writes a versioned JSON
  manifest with SHA-256 hashes for the script, imported modules, executable,
  package manifest, declared inputs and declared outputs; typed `--param`
  values exposed through `run_param()`; random seed; CPU/GPU decision; elapsed
  time; and peak resident memory. Missing inputs fail before execution and
  missing expected outputs fail the postflight check.

- **Single-cell object and axis validation.** `sc.validate()` returns a
  structured report and `sc.assert_valid()` stops at invalid object boundaries.
  Checks cover matrix dimensions, exact HVG/ranked-feature identity mappings,
  layers, assays, active assay, annotations, cell-aligned values, reductions,
  and graph endpoints. Constructors and `sc.standard()` now validate
  automatically, including compatible foreign objects using `cells` rather
  than `barcodes`.

- **Opt-in publication plotting theme.** Generic Cartesian plots, UMAP,
  FeaturePlot, violin and density plots, PCA, single-cell dot plots, heatmaps,
  clustered marker panels, Hi-C maps, OncoPrints, Venn/UpSet diagrams, sequence
  logos and phylogenetic trees accept `theme: "publication"`, which adds deterministic
  text-aware margins, consistent typography, subtle grids, external legends,
  subtitles and captions without changing the historical default theme. UMAP
  uses equal coordinate units in this theme; FeaturePlot adds a perceptually
  ordered continuous scale, explicit missing-value colour, high-values-on-top
  draw order, and numeric or `q05`/`q95` cutoffs. Violin labels adapt to narrow
  figures without changing KDE geometry. Dot plots use area for detection rate
  and a zero-centred diverging scale for expression z-scores. Both are checked
  at notebook, 85 mm and 180 mm logical widths.

- **Generated biological plot gallery.** The website catalogue is generated
  by executing all 21 documented biological plot examples in the current WASM
  runtime. Every preview records the renderer's actual theme, links to its
  builtin documentation, opens in a keyboard-accessible large preview, and
  provides SVG plus high-resolution PNG downloads. Structural and browser
  tests reject missing, stale, blank or low-resolution gallery artifacts.

- **Publication heatmaps without analytical drift.** Generic heatmaps retain
  input order, or their documented row-mean order with `cluster: true`.
  `clustered_heatmap` retains its deterministic nearest-neighbour traversal and
  now discloses it in accessible metadata. Publication heatmaps add adaptive
  row/column labels, subtitles, captions, named colour guides, automatic
  zero-centred diverging colour for signed data, and perceptually ordered
  sequential colour otherwise. Single-cell marker helpers now pass explicitly
  named numeric matrices instead of mixing a gene-name string column into the
  heatmap values.

- **Explicit R-validated hierarchical heatmaps.** `clustered_heatmap` now
  accepts `order: "hierarchical"` with Euclidean or Manhattan distance,
  complete/average/single/Ward D2 linkage and selectable row, column or both
  dendrograms. The default nearest-neighbour order is unchanged. Leaf order and
  merge heights are checked against base R 4.5.2 for all four linkages, and SVG
  metadata records the exact method and geometry. An in-place cluster-slot
  distance grid uses roughly one quarter of the memory of a `2n × 2n` design.

- **Inspectable UMAP and FeaturePlot specifications.** Both embedding plots can
  return `biolang.plot.spec/v1` with `format: "spec"`. The specification keeps
  every source coordinate, group, point label and feature value together with
  resolved quantile cutoffs, publication draw rank, aspect choice and raster
  decision. `render_plot()` replays it to byte-identical SVG or standalone
  HTML/Canvas, while dense embeddings retain the bounded embedded-PNG point
  layer. Non-finite coordinate pairs are disclosed and excluded from rendered
  geometry instead of emitting invalid SVG coordinates.

- **Replayable PCA, volcano and MA specifications.** `pca_plot()`, `volcano()`
  and `ma_plot()` now accept `format: "spec"` and replay through
  `render_plot()`. PCA stores the computed PC1/PC2 scores, groups, labels and
  explained-variance percentages, so replay never recomputes the decomposition.
  Differential-expression specs retain raw and transformed coordinates, gene
  labels, resolved thresholds and per-row classification. Direct and replayed
  SVGs are byte-identical, dense gene clouds retain their bounded raster mark
  layer, and non-finite geometry is reported rather than written into SVG. PCA
  now identifies genuinely numeric table columns, so text sample or group
  columns can no longer silently become NaN scores, and rejects non-finite
  numeric input explicitly.

- **Inspectable violin, dot and heatmap specifications.** `violin()`,
  `violin_plot()`, `dot_plot()`, `heatmap()` and `clustered_heatmap()` accept
  `format: "spec"` and replay through `render_plot()`. Violin specifications
  retain the resolved KDE grid, bandwidth, sample count and median. Dot plots
  expose mean expression, detected-cell fraction and clipped per-gene z-score
  for every gene-cluster pair. Heatmaps retain source and display order,
  resolved colour domains and, for hierarchical clustering, every merge and
  height. SVG replay is byte-identical and HTML includes the Canvas fallback.

- **Inspectable survival, diagnostic and effect-size specifications.**
  `kaplan_meier()`, `roc_curve()` and `forest_plot()` accept `format: "spec"`
  and replay through `render_plot()`. Kaplan-Meier records each tied risk set,
  event/censor count, product-limit estimate and Greenwood standard error. ROC
  records one point per distinct score threshold with TP/FP/TN/FN counts and a
  frozen trapezoidal AUC, removing the previous input-order dependence for tied
  scores. Forest plots retain every estimate, interval, weight, reference line,
  linear/log scale and resolved display domain. Dense survival and ROC curves
  use bounded SVG path elements, and HTML includes the Canvas fallback.

- **Inspectable genomic-association specifications.** `manhattan()`, the
  genetic `qq_plot()` and `rainfall()` now accept `format: "spec"` and replay
  through `render_plot()`. Manhattan records first-observed chromosome order,
  cumulative offsets, raw/transformed p-values, resolved significance and
  optional highlighting. Genetic Q-Q records expected plotting positions,
  observed p-values, genomic inflation factor λGC and an opt-in exact beta
  order-statistic confidence envelope. Rainfall records stable
  within-chromosome ordering, raw and plotted distances, and duplicate-position
  floors separately. Dense marks rasterise without discarding analytical rows.

- **Inspectable genomic-track specifications.** `ideogram()`, `cnv_plot()` and
  `coverage_track()` now accept `format: "spec"` and replay through
  `render_plot()`. Ideograms retain cytobands and standard stain classes on one
  shared chromosome-length scale. CNV profiles retain real genomic segment
  bounds, log2 ratios, thresholds and gain/loss states. Coverage tracks preserve
  point versus interval geometry, require an explicit chromosome for mixed
  inputs and clip half-open overlapping intervals to requested regions. Dense
  inputs use bounded path layers without dropping analytical rows; coordinate
  geometry is checked independently against base R.

- **Replayable regional annotation and splicing plots.** `genome_track()`,
  `lollipop()` and `sashimi()` now accept `format: "spec"` and replay through
  `render_plot()`. Genome features retain original and region-clipped bounds
  and use deterministic non-overlapping lanes. Lollipop plots honour `length`
  as the full sequence domain and freeze collision-limited labels. Sashimi
  plots retain sorted coverage, complete splice junctions, read-count scaling
  and reproducible arc lanes. Dense inputs stay complete in the specification
  while bounded path layers keep SVG documents responsive.

- **Deterministic multi-track circular genomes.** `circos()` now uses
  chromosome-length-weighted arcs, interval ribbons or point links, and typed
  line, bar, point, heatmap, CNV and annotation tracks. Its PlotSpec freezes
  every chromosome, mark, link, angular coordinate and radial coordinate for
  byte-identical replay. Dense inputs retain all analytical rows while grouped
  paths bound SVG complexity. Independent base-R formulas check 83 circular
  coordinate and scaling values, including link widths and track radii.

- **Replayable multi-panel figures.** `plot_grid()` composes SVG plots and
  PlotSpecs into deterministic equal-size cells with spreadsheet-style panel
  tags, figure titles/captions, shared outer labels and an explicit shared
  legend. Child SVG is screened for active content, the complete composition
  can be stored as `biolang.plot.spec/v1`, and SVG replay is byte-identical.
  Standalone HTML retains the Canvas fallback.

- **Publication export controls.** `save_svg()` accepts a publication profile,
  controlled font stacks and paired physical `width_mm`/`height_mm` dimensions
  while preserving vector viewBox geometry. `save_png()` accepts exact `dpi`
  as an alternative to `scale` and rejects ambiguous requests containing both.
  Both exporters accept a PlotSpec directly and replay it internally, avoiding
  a separate `render_plot()` step in analysis scripts.
  The browser workbench continues to expose SVG, screen PNG, 300-DPI PNG and
  print/PDF controls.

- **Terminal plot previews instead of raw SVG dumps.** The CLI recognises a
  complete SVG plot whether it is the final value or passed to `println`, and
  renders a compact Braille/Unicode preview in an interactive terminal. Plain
  ASCII, automatic SVG files, opening in the platform viewer, suppression and
  raw markup are explicit `--plot` / `:plot` modes, selectable per run
  (`--plot`, `--plot-dir`) or per session (`:plot`). The original SVG remains
  the value, so `save_plot` and structured notebook/event clients are
  unaffected.

### Changed

- **Plot status lines now go to standard error.** Where a figure was saved, why
  a preview could not be drawn, and that display is suppressed are diagnostics,
  not data. Standard output carries only the figure itself, so
  `bl run figure.bl --print-result > figure.svg` still writes an SVG: the
  default `auto` mode draws terminal graphics only when standard output is a
  terminal, and passes a redirected stream through untouched. Ask for `unicode`
  or `ascii` explicitly to draw into a redirected stream, or `raw` to force SVG
  onto a terminal.

## [1.5.0] - 2026-08-21

### Added
- `ecdf_plot(list, opts?)` and `density_plot(list, opts?)`. Both show a
  distribution without a histogram's bin width deciding what it looks like: the
  ECDF has no smoothing parameter at all and is drawn as the step function it
  is, and the density's bandwidth is stated rather than implied by where the bin
  edges happened to fall. The default bandwidth is Silverman's rule computed the
  way R's `bw.nrd0` computes it, fallback chain included, and is checked against
  R 4.6.1 on five cases including the tied and constant columns where every
  measure of spread is zero.

- **Several series on one pair of axes.** `plot(table, {y: ["a", "b", "c"]})`
  draws one line, scatter or bar group per named column, on one shared vertical
  scale, with a legend naming them. Drawing them separately and placing them
  side by side is not the same picture: each panel gets its own scale, so the
  comparison the figure exists to make is the one thing it cannot show.

- **`chi_square_contingency(table, opts?)`**, a chi-square test of independence
  on an r x c table. `chi_square(observed, expected)` is a goodness-of-fit test
  and reports k - 1 degrees of freedom, which is wrong for a table whose
  expected counts came from its own margins — on the Berkeley 2x2 that is 3
  where the answer is 1. Yates' continuity correction is applied to 2x2 tables
  by default and to no other shape, as R does.
- **`fisher_exact` reports the conditional odds ratio too.** `odds_ratio` is
  still the sample cross-product; `conditional_odds_ratio` is the conditional
  MLE R's `fisher.test` prints, with an exact interval from inverting the same
  test rather than a Wald interval on the sample ratio.
- `regularized_gamma_q`, `chi_square_sf`, `normal_sf` and `students_t_sf` in
  `bio-core`: each upper tail computed from the branch that has the digits,
  rather than as one minus its opposite.

### Fixed
- **Upper tails no longer underflow to zero.** Every p-value was computed as
  `1 - some_cdf(x)`, a subtraction with nothing left where the answer is
  interesting: chi-square p-values drifted below 1e-15 and hit exactly 0.0 past
  it, so a chi2 of 81 reported 0.0 where the answer is 2.2572e-19, and
  `spearman` returned 0.0 on n = 272 where it should return 1.9895e-56. A
  p-value of zero is not a stronger result, it is a missing one, and it breaks
  every `-log10(p)`. Now accurate to about 1e-13 down to 9.5e-111.
- **`pnorm` is no longer a seven-figure approximation.** It was Abramowitz &
  Stegun, good to 1.5e-7 *absolute* — which in a tail is the whole answer. It
  now computes the tail from the incomplete gamma already in the module and
  agrees with R to about 4e-14, including `pnorm(-10) = 7.6199e-24`.
- **Student's t no longer becomes the normal above 100 degrees of freedom.**
  The CDF short-circuited to `normal_cdf` for df > 100. The t has heavier tails
  at every finite df, so this returned p-values too small — a factor of four at
  df = 101 and t = 5, and still 8% out at df = 2000. Every t-test, correlation
  test and regression coefficient on a sample above 100 was affected.
- **`wilcoxon` defaults to the test R would choose.** All three modes were
  implemented and each matched R, but the default was the normal approximation
  with no continuity correction — the least accurate of the three, and chosen
  for none of them. It now uses the exact distribution when both groups are
  under 50 with no ties, and the corrected approximation otherwise.
- **`permutation_test` accepts its documented fourth argument.** It was
  registered in two modules; dispatch reached `stats.rs` for the behaviour while
  the copy in `statistics.rs` decided the arity, so `Exact(3)` rejected the
  optional permutation count before the implementation that accepts it ran.
  `fisher_exact` and `chi_square` were duplicated the same way. All three copies
  are gone.
- **Inference builtins no longer ignore options they do not read.**
  `ttest(a, b, {welch: true})` returned the pooled-variance result with no
  warning, and so did a mistyped `{varianse: "welch"}` — a call that reads as
  deliberate, runs, and answers a different question. (Welch was available the
  whole time as `{variance: "welch"}`, and agrees with R exactly.) Each of the
  nine inference builtins now declares the keys it reads; anything else is
  refused by name, with the accepted list and a "did you mean" for a near miss.
  The lists differ per builtin — `anova` reads neither `alternative` nor
  `confidence` — because accepting them everywhere would put the silence back.
- **Errors printed every explicit hint twice.** `format_with_source` started
  from the `Display` rendering, which prints the suggestions itself, and then
  printed them again after the source excerpt — so a "did you mean" appeared
  once above the file location and once below it. Both renderings now share a
  header helper, and the hint appears once, below the excerpt.
- **A bar chart's y axis now includes zero.** It scaled to the range of the
  data, so counts of 100 and 104 were drawn as a bar of nothing beside a
  full-height one. A bar's length is read as the quantity; anchoring it
  anywhere but zero is the best-known way to mislead with a chart, and it was
  the default. Negative bars hang below the line rather than collapsing.
- **A bar chart labels its x axis with its categories.** The x column was read
  as numbers, so "Biology" became NaN and the axis ran 0.0 to 1.0 underneath
  bars that had nothing to do with those numbers. Labels are thinned when there
  are more than the axis can fit.
- **The box plot agrees with `quantile()`.** It took `sorted[n / 4]` and
  `sorted[3 * n / 4]` — the nearest-rank rule — while `quantile()` interpolates
  (type 7, as R does). On the ozone column that drew the top of the box at 64
  next to a printed third quartile of 63.25. The whiskers now follow Tukey's
  rule, stopping at the last value within 1.5 IQR of the box with anything
  beyond it drawn as its own mark, instead of reaching to the extremes and
  drawing every dataset as though it had no outliers.
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
