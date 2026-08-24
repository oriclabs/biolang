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
| `histogram`, `histogram_data` | `histogram` | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes, `biolang.plot.geometry/v1` | Explicit edges, closure, endpoint handling, counts and density have an independent base-R gate. Automatic rule parity remains separate. |
| `ecdf_plot`, `ecdf_data` | `ecdf_plot` | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes | Ties collapse into one jump and match base R/NumPy reference geometry. |
| `boxplot_data` and `plot(..., {type: "box"})` | box plot | `crates/bl-runtime/src/plot.rs` | SVG `Str`; geometry `Record` | Yes | Type-7 summaries and optional base-R Tukey hinges are separately declared and validated. |
| `normal_qq_data` | normal Q-Q geometry | `crates/bl-runtime/src/plot.rs` | geometry `Record` | Yes | R plotting positions plus quartile reference line validated against R and Python. |
| `violin_data`, `violin`, `violin_plot` | violin/KDE geometry | `plot.rs`, `bio_plots.rs` | geometry `Record`; SVG/ASCII | Yes | All runtime violin/density paths now share the validated Gaussian KDE and `bw.nrd0` bandwidth rule. |
| `density_plot`, `density` | distinct compatibility surfaces sharing KDE | `plot.rs`, `bio_plots.rs` | SVG `Str`; `density` also has terminal output | Via `violin_data` KDE | Not aliases: `density_plot` is a single-list 256-point line; `density` supports multiple table-column groups, filled curves and ASCII. Both now use Gaussian `bw.nrd0` geometry. |
| `linear_fit_data`; `stats_relationship_plot(..., {interval: ...})` | `linear_fit_data` geometry | `plot.rs`, `stats_explore.rs` | geometry `Record`; guided SVG/ASCII | Yes | OLS fitted means, confidence intervals and prediction intervals independently match R and statsmodels on real data. |
| `heatmap`, `clustered_heatmap`, `hic_map` | distinct until audited | `plot.rs`, `bio_plots.rs` | SVG `Str` | No | Similar marks but different statistical transformations and domain meaning. |
| `volcano`, `volcano_plot` | `volcano_plot` | `plot.rs`, `bio_plots.rs` | SVG `Str` | No | Duplicate public concepts with potentially different thresholds/options. |
| `ma_plot` | `ma_plot` | `plot.rs` | SVG `Str` | No | Point-heavy path; candidate for selective raster marks. |
| `genome_track`, `coverage_track`, `coverage` | distinct until audited | `plot.rs`, `bio_plots.rs`, `viz.rs` | SVG or text | No | Names overlap but inputs and biological meanings differ. |
| `save_svg`, `save_plot` | `save_svg` | `plot.rs` | path/result value | N/A | True aliases. `save_plot` should remain compatibility-only. |
| `save_png` | `save_png` | `plot.rs` | path/result value | N/A | Uses SVG-to-PNG conversion; no direct scene/geometry input. |

## Bio plot API

All entries below are registered in `crates/bl-runtime/src/bio_plots.rs` and
currently return SVG strings unless their implementation explicitly documents a
different result.

| Family | Current names | Canonicalisation / risk |
|---|---|---|
| Association and diagnostic | `manhattan`, `qq_plot`, `roc_curve`, `pca_plot` | Keep distinct. `qq_plot` needs a declared reference distribution and line convention in conformance tests. |
| Genomic coordinate | `ideogram`, `rainfall`, `cnv_plot`, `lollipop`, `circos`, `circos_plot`, `sashimi`, `coverage_track` | `circos`/`circos_plot` are duplicate concepts to audit. Coordinate clipping and ordering need tests. |
| Distribution and survival | `violin`, `violin_plot`, `density`, `kaplan_meier`, `forest_plot` | `violin` and `violin_plot` are not aliases: wide table columns versus long-form group/value input. They share KDE geometry. Survival estimates should be validated independently of pixels. |
| Matrix and set | `clustered_heatmap`, `oncoprint`, `venn`, `upset`, `upset_plot`, `hic_map` | `upset`/`upset_plot` overlap. Clustering/reordering must be separated from drawing. |
| Sequence and tree | `sequence_logo`, `phylo_tree`, `alignment_view` | `alignment_view` also exists in `viz.rs`; dispatch ownership must be made unambiguous. |
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
| `statistics` | `distribution_plot`, `normal_qq_plot`, `group_plot`, `relationship_plot`, `categorical_plot`, `missingness_plot`, `linear_diagnostic_plot`, `normal_diagram`, `visualize`, plus ASCII summaries | records, SVG and ASCII assembled by guided-statistics builtins | Distribution histograms, normal Q-Q, grouped boxes, relationship fits/bands and residual Q-Q now consume shared geometry. Categorical and missingness plots remain local geometry. |
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
