# BioLang plot inventory

This is the initial inventory for plot conformance work. It records what users
can call today, where each plot is implemented, what it returns, and where two
names or implementations overlap. It is deliberately descriptive: an entry in
this file is not a claim of statistical parity with R or Python.

## Output forms

| Output | Current representation | Main consumers | Risk |
|---|---|---|---|
| Vector figure | SVG stored in `Str` | notebook, browser, `save_svg` | Geometry and presentation are inseparable; a terminal can print the XML by accident. |
| Raster figure | PNG encoded from SVG | `save_png`, terminal preview | Export is reliable, but it rasterises after an SVG has already been built. |
| Terminal figure | Unicode/Braille or raw text | `bl` REPL, `render_plot` | Versioned Cartesian specs render through the same SVG before terminal conversion; legacy plots remain SVG-first. |
| Standalone browser figure | HTML with SVG plus Canvas/PNG fallback | browser, exported artifact | `render_plot(..., {format: "html"})` preserves SVG and prepares Canvas only as a display fallback. |
| Guided statistics | records containing SVG/ASCII/text | `statistics` package | Several functions calculate their own display geometry. |
| Package figure | hand-built SVG `Str` | `singlecell` | Package code bypasses runtime scales, validation, accessibility, and raster thresholds. |

## Runtime plot API

| Current name(s) | Canonical name | Source | Return | Shared geometry | Status / main risk |
|---|---|---|---|---|---|
| `plot`, `plot_spec`, `render_plot` | `plot` | `crates/bl-runtime/src/plot.rs` | SVG/text/HTML `Str`; spec `Record` | Yes for scatter, line, error bar and confidence band: `biolang.plot.spec/v1` | Bar remains legacy; box drawing now consumes shared box geometry. |
| `plot_grid` | `plot_grid` | `crates/bl-runtime/src/plot.rs` | SVG/text/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Equal-size panel cells, tags, outer labels and explicit shared legends are frozen. Child SVG rejects active content; semantic axis alignment remains a future scene-level capability. |
| `histogram`, `histogram_data` | `histogram` | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes, `biolang.plot.geometry/v1` | Explicit edges, closure, endpoint handling, counts and density have an independent base-R gate. Automatic rule parity remains separate. |
| `ecdf_plot`, `ecdf_data` | `ecdf_plot` | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes | Ties collapse into one jump and match base R/NumPy reference geometry. |
| `boxplot_data` and `plot(..., {type: "box"})` | box plot | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes | Type-7 summaries and optional base-R Tukey hinges are separately declared and validated. |
| `normal_qq_data` | normal Q-Q geometry | `crates/bl-runtime/src/plot.rs` | geometry `Record` | Yes | R plotting positions plus quartile reference line validated against R and Python. |
| `violin_data`, `violin`, `violin_plot` | violin/KDE geometry | `plot.rs`, `bio_plots.rs` | geometry `Record`; SVG/ASCII | Yes | All runtime violin/density paths now share the validated Gaussian KDE and `bw.nrd0` bandwidth rule. |
| `density_plot`, `density` | distinct compatibility surfaces sharing KDE | `plot.rs`, `bio_plots.rs` | SVG `Str`; `density` also has terminal output | Via `violin_data` KDE | Not aliases: `density_plot` is a single-list 256-point line; `density` supports multiple table-column groups, filled curves and ASCII. Both now use Gaussian `bw.nrd0` geometry. |
| `linear_fit_data`; `stats_relationship_plot(..., {interval: ...})` | `linear_fit_data` geometry | `plot.rs`, `stats_explore.rs` | geometry `Record`; guided SVG/ASCII | Yes | OLS fitted means, confidence intervals and prediction intervals independently match R and statsmodels on real data. |
| `categorical_data`; `stats_categorical_plot` | categorical frequency geometry | `plot.rs`, `stats_explore.rs` | geometry `Record`; guided SVG/ASCII | Yes | Counts and proportions retain first-observed category order; missing values are reported separately. |
| `missingness_data`; `stats_missingness_plot` | missingness geometry | `plot.rs`, `stats_explore.rs` | geometry `Record`; guided SVG/ASCII | Yes | Full-data counts use every cell; deterministic row/column strides bound only the rendered grid. Nil and non-finite numbers are missing. |
| `kaplan_meier` | Kaplan-Meier survival | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Tied event/censor risk sets, product-limit survival and Greenwood standard errors match independent `survival::survfit` output. |
| `roc_curve` | receiver operating characteristic | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Equal scores are advanced as one threshold; confusion counts and trapezoidal AUC are frozen and independently validated. |
| `forest_plot` | confidence-interval forest plot | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Linear/log domains, reference line, row provenance and marker-area weights are frozen; interval fields match the R fixture. |
| `manhattan` | genome-wide association Manhattan plot | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | First-observed chromosome order, cumulative offsets, raw/transformed p-values and significance are frozen and independently checked against base R. |
| `qq_plot` | genetic p-value Q-Q plot | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Distinct from normal Q-Q: freezes `(rank - 0.5) / n`, λGC and an opt-in exact beta order-statistic envelope, all checked against base R. |
| `rainfall` | inter-variant distance rainfall plot | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Stable within-chromosome sorting and raw/plotted/log distances retain duplicate positions explicitly and match the R fixture. |
| `ideogram` | chromosome cytoband ideogram | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Cytobands, stains and first-observed chromosome order are frozen; every chromosome uses the same length scale and geometry matches the R fixture. |
| `cnv_plot` | copy-number segment profile | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Actual segment bounds, cumulative offsets, log2 ratios, thresholds and gain/loss states are frozen and independently checked. |
| `coverage_track` | regional point/interval coverage | `crates/bl-runtime/src/bio_plots.rs` | SVG/terminal/HTML `Str`; spec `Record` | Yes, `biolang.plot.spec/v1` | Point and interval geometry remain distinct; half-open overlaps are clipped to the requested single-chromosome region and independently checked. |
| `heatmap`, `clustered_heatmap`, `hic_map` | distinct until audited | `plot.rs`, `bio_plots.rs` | SVG `Str` | No | Similar marks but different statistical transformations and domain meaning. All three now honour the shared publication presentation tokens; this does not make their analytical geometry interchangeable. |
| `volcano`, `volcano_plot` | `volcano_plot` | `plot.rs`, `bio_plots.rs` | SVG `Str` | No | Duplicate public concepts with potentially different thresholds/options. |
| `ma_plot` | `ma_plot` | `plot.rs` | SVG `Str` | No | Point-heavy path; candidate for selective raster marks. |
| `genome_track`, `coverage` | distinct concepts | `plot.rs`, `viz.rs` | SVG/terminal/HTML `Str`; genome spec `Record` | `genome_track`: yes | `coverage` is a terminal depth summary; `genome_track` freezes clipped named intervals, strand and deterministic non-overlapping lanes. |
| `save_svg`, `save_plot` | `save_svg` | `plot.rs` | path/result value | N/A | SVG strings and PlotSpecs are accepted. Optional publication metadata, controlled font stacks and paired physical millimetre dimensions retain the vector viewBox. `save_plot` should remain compatibility-only. |
| `save_png` | `save_png` | `plot.rs` | path/result value | N/A | SVG strings and PlotSpecs are accepted; conversion uses explicit scale or exact DPI. |

## Bio plot API

All entries below are registered in `crates/bl-runtime/src/bio_plots.rs` and
currently return SVG strings unless their implementation explicitly documents a
different result.

| Family | Current names | Canonicalisation / risk |
|---|---|---|
| Association and diagnostic | `manhattan`, `qq_plot`, `roc_curve`, `pca_plot` | Keep distinct. Manhattan, genetic Q-Q and ROC now have frozen, independently validated specifications. Genetic `qq_plot` is not the normal-data Q-Q diagnostic. |
| Genomic coordinate | `ideogram`, `rainfall`, `cnv_plot`, `lollipop`, `circos`, `circos_plot`, `sashimi`, `coverage_track` | Ideogram, rainfall, CNV, coverage, lollipop, sashimi and the multi-track `circos` API now freeze and independently validate coordinate geometry. `circos_plot` remains a separate legacy/default-human-chromosome surface pending compatibility migration. |
| Distribution and survival | `violin`, `violin_plot`, `density`, `kaplan_meier`, `forest_plot` | `violin` and `violin_plot` are not aliases: wide table columns versus long-form group/value input. They share KDE geometry. Kaplan-Meier and forest geometry now have frozen specifications and independent R gates. |
| Matrix and set | `clustered_heatmap`, `oncoprint`, `venn`, `upset`, `upset_plot`, `hic_map` | `upset`/`upset_plot` overlap. Clustering/reordering must be separated from drawing. The primary gallery paths now honour publication theme/title/subtitle/caption options consistently. |
| Sequence and tree | `sequence_logo`, `phylo_tree`, `alignment_view` | `sequence_logo` and `phylo_tree` now honour the publication presentation layer. `alignment_view` also exists in `viz.rs`; dispatch ownership must be made unambiguous. |
| Single-cell | `umap_plot`, `feature_plot`, `elbow_plot`, `variable_feature_plot`, `dot_plot` | High-value parity targets. Large embeddings need selective raster marks without changing axes/text. |

## Terminal visual API

These are registered in `crates/bl-runtime/src/viz.rs`.

| Current name | Return style | Relationship / risk |
|---|---|---|
| `sparkline` | text | Terminal-only compact summary. |
| `bar_chart` | text | Not a renderer for runtime SVG bar plots. |
| `boxplot` | text | Must eventually consume the same quartile/whisker geometry as graphical box plots. |
| `heatmap_ascii` | text | Terminal counterpart, currently independent. |
| `coverage` | text | Not an alias of `coverage_track`. |
| `dotplot` | text | Not an alias of single-cell `dot_plot`. |
| `alignment_view` | text | Same public name is also registered by bio plots; dispatch order needs an explicit test. |
| `quality_plot` | text | Domain-specific terminal summary. |

## Package-authored plots

| Package | Public functions | Current representation | Priority |
|---|---|---|---|
| `statistics` | `distribution_plot`, `normal_qq_plot`, `group_plot`, `relationship_plot`, `categorical_plot`, `missingness_plot`, `linear_diagnostic_plot`, `normal_diagram`, `visualize`, plus ASCII summaries | records, SVG and ASCII assembled by guided-statistics builtins | Distribution histograms, normal Q-Q, grouped boxes, relationship fits/bands, residual Q-Q, categorical frequencies and missingness grids now consume shared geometry. |
| `singlecell` | `plot_umap`, `plot_pca`, `dim_plot`, `plot_feature`, `feature_plot`, `plot_violin`, `vln_plot`, `plot_markers`, `plot_elbow`, `plot_proportions`, `expr_dotplot`, `dot_plot`, `do_heatmap` | BioLang code; some paths call runtime plots and some build raw SVG | Replace raw SVG only after equivalent geometry and snapshots exist. |
| `singlecell` advanced | QC, embedding, split-feature, heatmap, differential-expression, pseudobulk, composition, enrichment and stability plots in `advanced_plots.bl` | hand-built SVG | High maintenance risk; migrate by plot family, not with a mechanical rewrite. |

## First conformance sequence

1. `histogram_data()` now exposes a versioned, renderer-independent record.
2. `histogram()` consumes exactly that geometry.
3. Explicit breaks, closure and endpoint handling are validated against base R.
4. Automatic break rules are clearly named; do not call an
   equal-width approximation “R-compatible”.
5. Box plots, ECDF, normal Q-Q and violin/KDE now repeat the geometry-first
   pattern and pass independent R/Python numeric gates.
6. Scatter, line, error-bar and confidence-band plots now use
   `biolang.plot.spec/v1` across SVG, terminal and standalone browser output.
7. The first alias audit records that genomic `qq_plot` and normal
   `normal_qq_plot` are different diagnostics, that `violin` and `violin_plot`
   have wide-versus-long input contracts, and that `density` adds grouped/ASCII
   behavior absent from `density_plot`; none should be silently aliased.
8. Real `airquality` ozone values now gate box, ECDF, Q-Q and KDE geometry, and
   ozone-by-month gates OLS confidence/prediction coordinates against R and
   statsmodels.
9. Categorical frequencies and missingness grids expose stable geometry, and
   SVG/standalone HTML output has structural accessibility tests for titles,
   descriptions, controls and the Canvas fallback.
10. Kaplan-Meier, ROC and forest plots now expose validated frozen
    specifications, replay through every renderer, and keep dense curves in
    bounded SVG paths rather than one element per observation.
11. Manhattan, genetic Q-Q and rainfall plots now freeze genomic ordering,
    transforms and diagnostic metadata; independent base-R gates include exact
    beta order-statistic bounds and duplicate-position rainfall distances.
12. Ideogram, CNV and coverage-track specifications now freeze cytobands,
    chromosome lengths, segment bounds, threshold states and half-open region
    clipping; dense SVGs use bounded path layers and base-R gates cover the
    numeric geometry.
13. Genome-track, lollipop and sashimi specifications now freeze regional
    clipping, greedy interval lanes, sequence domains, coverage order and
    count-scaled splice arcs; direct/replay output is byte-identical and dense
    marks use bounded path layers.
14. Multi-track circos specifications now freeze length-weighted chromosome
    angles, typed track radii, ribbons and link weights; 83 independent base-R
    comparisons gate the numeric geometry, while dense SVG layers remain
    bounded without dropping specification rows.
