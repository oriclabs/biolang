# BioLang Statistical Plotting Requirements

Status: implementation requirements and progress record

## 1. Purpose and decision

BioLang should produce statistically established plots whose calculated
geometry can be validated against documented R and Python conventions.

The near-term objective is not to replace the plotting stack. It is to improve
the correctness, inspectability, performance, and validation of the plotting
code that already exists.

The longer-term architecture is a standalone Rust plotting core used through
normal BioLang functions. It must not require R, Python, Docker, a network
service, or a separate plotting executable at runtime.

R and Python are development-time validation oracles. They are not runtime
dependencies.

The intended scientific guarantee is:

> BioLang's calculated plot values, transformations, limits, annotations, and
> diagnostic quantities match a named reference convention within declared
> numerical tolerances.

Do not promise pixel-for-pixel equality. Fonts, antialiasing, operating-system
font discovery, and device-pixel ratio may change pixels without changing the
scientific result.

SVG remains supported. The question is whether it should remain the only
internal plot representation, not whether it should be removed.

## 2. Verified current baseline

This section describes the current working tree, including the unfinished CLI
plot-display changes. It must be refreshed before implementation begins.

### 2.1 Current representation and rendering

- Generic and biological plot builtins normally return SVG as a BioLang Str.
- SvgCanvas stores elements as Vec<String>.
- SvgCanvas already provides typed methods for rectangles, circles, lines,
  text, rotated text, axes, titles, and raster point layers.
- Scale currently represents a linear domain and range.
- nice_ticks currently divides the domain into equal intervals; it is not a
  conventional nice-number tick algorithm.
- Native PNG export rasterises the SVG through resvg/tiny-skia.
- Terminal Braille and ASCII fallback currently rasterise the SVG through the
  same native path.
- add_point_raster already embeds a rasterised point layer inside otherwise
  vector SVG.
- Some BioLang packages, particularly single-cell plotting code, construct raw
  SVG strings directly in BioLang.

Consequences:

- a future typed Plot cannot assume all existing plots originate in Rust;
- SVG Str compatibility is a real compatibility surface, not merely a temporary
  frontend concern;
- a BioLang-level plot-construction API would be needed before every
  package-authored SVG can become a typed scene;
- a SceneNode conversion can be incremental because much of SvgCanvas is
  already method-based.

### 2.2 Current CLI behaviour in the working tree

The current dirty working tree implements:

- automatic Unicode/Braille previews in an interactive terminal;
- portable ASCII previews;
- plot file, open, raw, and none modes;
- the same display policy for a final plot and explicitly printed SVG;
- status and fallback messages on standard error;
- redirected auto output preserved as SVG;
- :plot with no argument redrawing SVG, histogramming only List/Quality values,
  and explaining unsupported last-result types.

These are implemented current-tree capabilities, not new requirements.
They still need appropriate tests and a clean commit before being treated as a
released baseline.

The redirected-output invariant is mandatory:

    bl run figure.bl --print-result > figure.svg

must continue to write SVG unless the user explicitly requests another format.
Do not change redirected auto output to a plot specification.

### 2.3 Existing plot surface

BioLang is not starting from an empty plotting library.

The generic runtime already includes plot, heatmap, histogram, ECDF, density,
volcano, MA, and genome-track functionality plus save helpers. The biological
plot registry contains more than thirty public names, including aliases and
related implementations for:

- Manhattan and genetic Q-Q plots;
- ideogram, rainfall, CNV, and coverage views;
- violin and density plots;
- Kaplan-Meier, forest, and ROC plots;
- clustered heatmap and PCA;
- oncoprint, Venn, and UpSet;
- sequence logo, phylogenetic tree, lollipop, circos, Hi-C, and sashimi;
- volcano and alignment views;
- UMAP, feature, elbow, variable-feature, and dot plots.

Package-level BioLang code adds more single-cell plots.

Therefore, most near-term work is conformance, canonicalisation, geometry
exposure, and performance improvement. It is not a greenfield implementation
of all plot families.

### 2.4 Existing validation infrastructure

Extend the existing infrastructure:

- packages/statistics/validation/
- packages/statistics/validation/run.ps1
- packages/statistics/validation/reference.R
- packages/statistics/validation/inference_reference.R
- packages/statistics/validation/model_conformance.R
- the pinned .validation-r-library environment
- validation-results manifests and resource measurements

Do not create a parallel validation convention for plots.

Plot validation should add geometry tables and plot-specific comparison fields
to the existing manifest/resource conventions.

### 2.5 Notebook baseline

The current BLN/native notebook representation stores per-cell output as
Option<String> and can re-execute cells when opened. It does not currently have
a versioned structured-output container, MIME bundle, typed Plot store, or
general cached-image schema.

Persisting a typed Plot is therefore a notebook file-format migration. It needs:

- an explicit notebook container/schema version;
- backward-compatible loading of current BLN files;
- migration rules;
- independent tests and release gates;
- a decision about embedded transformed data versus reproducible references.

It must not be presented as a small Canvas display change.

### 2.6 GPU and size baseline

- wgpu is already an optional workspace dependency used for computation.
- The normal shipped CLI build excludes the optional GPU stack.
- The plotting question is whether to add a render pipeline, not whether to
  introduce wgpu to the workspace for the first time.
- Current local baselines are approximately 40.2 MB for target/release/bl.exe
  and 7.97 MB for desktop/public/wasm/bl_wasm_bg.wasm. Record exact byte counts
  in any future benchmark manifest rather than relying on rounded prose.

GPU rendering remains deferred until a measured plotting bottleneck survives
CPU rasterisation, Canvas, aggregation, and tiling.

## 3. Priority model

The requirements are divided into:

1. mandatory standards for every new or materially modified plot;
2. immediate work that improves the current implementation without committing
   to a new representation;
3. medium-term architecture activated by explicit product or performance
   triggers;
4. optional capabilities activated only by evidence.

This prevents a long representation rewrite from delaying demonstrable
scientific and performance fixes.

## 4. Mandatory statistical plot standard

Every new or materially modified statistical plot must document:

- canonical public name and retained aliases;
- accepted data and variable types;
- the scientific question it displays;
- the named estimator or transformation;
- missing, infinite, and invalid-value handling;
- group and category ordering;
- weight semantics;
- tie handling;
- quantile interpolation;
- transforms and inverse transforms;
- interval/band method;
- axis-domain and expansion rules;
- bin boundary and closure rules where relevant;
- outlier and clipping rules;
- deterministic ordering;
- random seed and resampling count where applicable;
- behaviour for small, constant, sparse, tied, or degenerate data;
- large-data sampling, aggregation, or rasterisation policy;
- inspectable numeric geometry;
- R/Python oracle and version;
- numeric acceptance gates;
- known differences;
- accessibility and terminal-display behaviour.

Name stable statistical methods rather than saying only R-like or Python-like.

Examples:

    normal_qq_plot(values, {
        quantile_method: "r-type-7",
        plotting_position: "r-qqnorm",
        reference_line: "quartiles",
        envelope: 0.95
    })

    histogram(values, {
        breaks: "sturges",
        right: true,
        include_lowest: true
    })

    density_plot(values, {
        kernel: "gaussian",
        bandwidth: "bw.nrd0",
        boundary_correction: "none"
    })

Aliases such as r, numpy, scipy, statsmodels, or ggplot may be offered only when
their precise behaviour is defined. Saved validation artifacts should record
the stable method name and reference version.

## 5. Immediate workstream 1: inventory and canonical names

Create one inventory row for every existing plot entry point:

    current name(s)
    canonical name
    source file/module/package
    input type
    current return type
    current renderer(s)
    statistical transformation
    existing geometry helper
    R oracle
    Python oracle
    current tests
    known divergence
    edge cases
    large-data risk
    migration priority

Resolve aliases before multiplying validation fixtures.

Rules:

- one canonical contract may cover aliases that call the same implementation;
- aliases must be tested to resolve to the same contract;
- visually related functions with different calculations require separate
  contracts;
- existing public aliases are not removed without a compatibility policy;
- documentation should prefer one canonical name.

The inventory must distinguish:

- already implemented and needing conformance;
- implemented but missing geometry access;
- implemented but demonstrably divergent;
- genuinely absent;
- package-authored raw SVG;
- display-only plots with no statistical transformation.

## 6. Immediate workstream 2: histogram conformance slice

Use histogram as the first complete conformance slice.

### 6.1 Verified current divergence

The current generic histogram:

- defaults to 20 equal-width bins;
- calculates bin membership using integer division from the lower bound;
- places an observation exactly on an internal break into the higher bin;
- clamps the maximum observation into the final bin.

R hist defaults to a Sturges-based break selection and normally uses
right-closed intervals with include.lowest behaviour. Exact R behaviour also
has boundary and floating-point details that must be captured from
documentation and black-box fixtures rather than approximated from memory.

### 6.2 Compatibility decision

Do not silently change established BioLang output before deciding compatibility.

Support explicit conventions first:

    histogram(values, {breaks: 20, closed: "left"})
    histogram(values, {breaks: "sturges", right: true, include_lowest: true})
    histogram(values, {breaks: "freedman-diaconis"})
    histogram(values, {breaks: "scott"})

The inventory must determine whether the historical 20-bin behaviour remains
the default for compatibility or changes in a documented major/versioned
transition.

### 6.3 Geometry output

Expose a single canonical histogram geometry table containing at least:

- bin index;
- lower boundary;
- upper boundary;
- left/right closure;
- count;
- density;
- cumulative count/density;
- dropped invalid-value count;
- method metadata.

The SVG, PNG, Canvas, and terminal views must consume the same geometry.

### 6.4 Fixtures

Include:

- values exactly on internal boundaries;
- minimum and maximum values;
- constant values;
- one and two observations;
- integers and floats;
- negative and positive values;
- missing/non-finite values;
- weighted data if weights are supported;
- highly skewed real data;
- a real biological measurement dataset;
- a large input for timing and memory.

Synthetic fixtures are allowed for mathematical edge cases where exact bin
membership must be constructed deliberately. Published parity and performance
claims require real data as well.

## 7. Immediate workstream 3: standard geometry access

Formalise existing ad hoc data helpers rather than immediately introducing a
new core Value variant.

A plot geometry result should use a versioned Record/Table schema:

    {
        schema: "biolang.plot.geometry/v1",
        kind: "histogram",
        data: table(...),
        scales: {...},
        methods: {...},
        warnings: [...],
        provenance: {...}
    }

Standardise or connect existing helpers such as histogram_counts, qq_data,
manhattan_data, volcano_data, PCA scores, group summaries, and other
plot-specific calculations.

Requirements:

- renderers consume geometry instead of recalculating statistics;
- geometry can be written to CSV/JSON for validation;
- geometry contains stable row identifiers and group order;
- provenance records method options, seed, and dropped data;
- renderers may add layout coordinates, but may not silently alter the
  statistical values;
- geometry schema evolution is independent from a future typed Plot schema.

This delivers most of the conformance benefit at substantially lower risk than
changing every interpreter/frontend value representation first.

## 8. Immediate workstream 4: extend validation

Use the current statistics validation runner and manifest formats.

### 8.1 External oracles

| BioLang geometry | Primary comparisons |
|---|---|
| Histogram | R hist; NumPy histogram |
| Boxplot | R boxplot.stats; Matplotlib boxplot statistics |
| ECDF | R ecdf; statsmodels/SciPy equivalent |
| Q-Q | R qqnorm/qqline; statsmodels/SciPy |
| KDE | R density; SciPy only where definitions align |
| Regression | R model results; statsmodels |
| Confidence/prediction bands | R prediction tools; statsmodels |
| GLM diagnostics | R GLM; statsmodels GLM |
| Multiple testing | R p.adjust; statsmodels |
| ROC/PR | established R and Python metric implementations |
| Survival | established R survival tools and Python comparison |
| PCA | R and scikit-learn after sign/orientation alignment |
| Clustering/heatmap | R and SciPy with identical settings |

### 8.2 Numeric comparisons

Do not validate only final images or Pearson correlation.

Compare:

- exact row, group, category, and label membership;
- bin edges, closure rules, counts, and densities;
- quantiles and plotting positions;
- Q-Q line slope/intercept and envelopes;
- KDE bandwidth and evaluation grid;
- coefficients, fitted values, residuals, and predictions;
- standard errors;
- confidence/prediction interval endpoints;
- test statistics and p-values;
- survival risk sets, censoring marks, and estimates;
- axis domains, transformations, and clipping;
- deterministic ordering and seed behaviour;
- absolute error;
- relative error;
- slope and intercept;
- RMSE;
- median and p90 error.

### 8.3 Manifest additions

Extend existing manifests with:

- geometry schema/version;
- plot kind and canonical name;
- method identifiers;
- BioLang commit/version;
- R/Python/package versions;
- operating system and architecture;
- input hash;
- complete options;
- seed;
- renderer/backend when an image is measured;
- elapsed time;
- peak resident memory;
- artifact size;
- acceptance thresholds;
- pass/fail result;
- documented differences.

Approved fixtures may be stored for offline CI. R/Python do not become runtime
or normal-user requirements.

### 8.4 Synthetic versus real data

Use synthetic data only for:

- exact edge placement;
- constant/degenerate inputs;
- controlled ties;
- known ordering;
- known model parameters;
- deterministic failure cases.

Use real data for:

- scientific parity claims;
- distribution-shape behaviour;
- feature and group ordering;
- biological plots;
- performance and memory claims;
- user-facing examples.

Every published claim must say which category produced it.

## 9. Immediate workstream 5: dense-layer performance

Do not convert every circle to raster output.

Many add_circle calls draw legends, estimates, annotations, or small datasets
that should remain vector. Rasterise only high-cardinality mark layers.

### 9.1 Policy

For each candidate plot:

1. collect the high-cardinality point layer;
2. preserve axes, text, legends, thresholds, and annotations as vector;
3. choose vector or raster using an explicit threshold;
4. record that choice in plot metadata;
5. compare visual output at normal view and zoom;
6. measure SVG bytes, PNG bytes, node count, render time, and peak memory;
7. retain a user override where appropriate.

Suggested option shape:

    {
        raster: "auto",       # auto, on, off
        raster_threshold: 20000,
        raster_scale: 2
    }

Thresholds must come from benchmarks rather than unsupported estimates.

### 9.2 Existing raster layer

Reuse and harden add_point_raster before introducing a new renderer.

Evaluate:

- transparency and overplotting;
- clipping;
- point radius at high DPI;
- deterministic point order;
- palette fidelity;
- true-colour versus indexed PNG size;
- large pixel-buffer allocation;
- browser decode cost;
- save_png consistency;
- zoom quality;
- whether data-URI embedding or separate blob delivery is better.

Separate blob delivery is a frontend protocol change, not a mechanical
replacement. It requires measurements and lifecycle/cleanup handling.

### 9.3 Claims

The repository contains a reproducible release probe and a machine-specific
`packages/statistics/validation/plot-benchmark.json` manifest. The measured
20,000-point case justifies the current UMAP threshold by output bytes and DOM
elements, not by render speed: raster construction is deliberately slower.

Any claimed reduction must be reproduced by a checked-in benchmark manifest
containing input size, plot dimensions, renderer, encoded bytes, DOM nodes,
elapsed time, peak memory, and visual-regression result.

## 10. Scale and axis correctness

Scale is currently linear. Extend it only through explicit, testable scale
types:

- linear;
- log10;
- log2;
- square root;
- symlog where justified;
- categorical/band;
- date/time if later needed.

Each scale must define:

- valid domain;
- forward and inverse transform;
- handling of zero and negative values;
- domain expansion;
- clipping;
- tick generation;
- tick formatting;
- minor ticks where applicable.

Do not label pre-transformed data as a renderer log scale. A volcano plot may
plot already calculated minus-log10 p-values on a linear display scale. Record
both the statistical transformation and display scale separately.

Replace or rename current nice_ticks:

- if it remains equal subdivision, name it equal_ticks;
- implement an actual nice-number tick algorithm for user-facing axes;
- test small, large, negative, crossing-zero, and narrow domains;
- prevent duplicate formatted tick labels.

## 11. Renderer roles in the current architecture

### 11.1 SVG

Keep SVG for:

- current runtime compatibility;
- publication vector export;
- accessible text and semantic grouping;
- small/medium static plots;
- HTML embedding;
- vector editing.

Avoid one SVG element per observation for dense layers.

### 11.2 PNG/bitmap

Use bitmap output for:

- universal static previews;
- document/email compatibility;
- notebook cached fallback;
- dense raster layers;
- high-resolution publication export;
- native display surfaces.

Validate dimensions before allocation. Support scale/DPI, background, and
reproducible font selection.

### 11.3 Terminal

Keep:

- Unicode/Braille preview;
- portable ASCII;
- explicit raw/file/open/none modes;
- redirected SVG compatibility.

Prefer semantic terminal plots for histograms, intervals, and other plots where
actual bins/estimates can be represented clearly. Retain SVG-raster terminal
preview as the universal fallback.

### 11.4 Canvas

Canvas remains a medium-term requirement because the intended notebook product
includes live browser display, interaction, and dense-data support.

Do not start Canvas before geometry schemas and measurements exist.

Canvas must eventually support:

- high-DPI rendering;
- responsive redraw without repeating statistical fits;
- deterministic redraw from geometry/specification;
- hover and selection where useful;
- appropriate pan/zoom;
- scalable hit testing;
- export to PNG;
- table/text accessibility alternative;
- offline operation;
- PNG or SVG fallback.

Canvas is a display backend, not the scientific calculation engine.

### 11.5 GPU

GPU rendering is optional and deferred. Evaluate it only when a measured dense
plot remains inadequate after geometry reuse, raster layers, Canvas,
aggregation, and tiling.

Do not add the optional GPU feature to the default CLI build for plotting.

## 12. Medium-term architecture with explicit triggers

| Capability | Trigger |
|---|---|
| Vec<String> to Vec<SceneNode> | A second renderer, normally Canvas, is approved |
| Typed Plot value | Structured notebook output or direct multi-renderer redraw is approved |
| Canvas renderer | Browser interaction/dense rendering work is scheduled |
| Versioned BLN plot storage | Typed Plot persistence is approved as a file-format migration |
| Separate raster blobs | Embedded data URI is measured as a bottleneck |
| GPU render pipeline | Canvas/raster/aggregation still fails a measured workload |
| PDF export | Concrete publication requirement cannot be met by SVG/high-DPI PNG |
| Additional themes | Core BioLang and publication themes are stable |

### 12.1 SceneNode transition

When triggered:

- replace SvgCanvas elements Vec<String> with Vec<SceneNode>;
- keep existing drawing method signatures where possible;
- convert raw elements.push sites incrementally;
- provide an escape node for compatibility only where unavoidable;
- render the same scene to SVG first;
- prove output equivalence before adding Canvas;
- do not split into many crates until module boundaries are stable.

The initial implementation should be modules or a small number of crates, not
an eight-crate reorganisation.

### 12.2 Typed Plot transition

When triggered, a typed Plot retains:

- schema/version;
- geometry reference;
- scales;
- mark/scene definitions;
- theme;
- labels and units;
- annotations;
- warnings;
- interaction metadata;
- provenance;
- accessible summary;
- optional cached fallback.

Compatibility:

- frontends recognise typed Plot and legacy SVG Str;
- save_svg/save_png accept both during migration;
- render(plot, "svg") returns explicit markup;
- the REPL retains the plot value after display;
- package-authored SVG remains supported;
- no removal version is promised until a BioLang-level construction API covers
  package-authored plots.

### 12.3 Notebook migration

When triggered:

- add an explicit BLN container version;
- define structured output or MIME-bundle representation;
- load all existing BLN files;
- define data embedding/reference limits;
- store optional cached PNG/SVG fallback;
- redraw Canvas from stored geometry/spec;
- do not require R, Python, network, or a surviving interpreter;
- test save/reopen and forward/backward migration independently.

## 13. Plot catalogue strategy

Do not use the catalogue as a greenfield construction schedule.

For every existing plot:

1. inventory;
2. select canonical name;
3. identify calculation and geometry;
4. document convention;
5. add only the necessary contract;
6. validate high-risk calculations;
7. address accessibility and large-data behaviour when touched.

Prioritise plots with:

- demonstrated R/Python divergence;
- statistical calculations hidden inside rendering code;
- high scientific impact;
- dense-data browser/memory problems;
- duplicated aliases/implementations;
- missing inspectable geometry.

Genuinely absent plots can be added after the conformance pass. Candidate gaps
include frequency polygon, P-P plot, rug/strip/swarm, general faceting, and
several inference-teaching diagrams. Confirm against the inventory before
declaring any plot absent.

Existing plots should not be blocked from release solely because a complete
five-artifact dossier has not yet been written. Apply the full standard to new
or materially modified plots; bring untouched existing plots into conformance
by risk and use.

## 14. Themes and appearance

Calculation and appearance are independent.

Start with two maintained themes:

- biolang: accessible, clear interactive/default theme;
- publication: restrained print/export theme.

Additional familiar R/Python-like themes are optional later. They must not
change bins, estimators, confidence methods, group order, or fitted values.

The default palette should:

- consider common colour-vision deficiencies;
- work in light and dark contexts;
- remain legible at notebook and print sizes;
- distinguish common 15-30-cluster biological plots;
- supplement colour with shapes/direct labels where needed.

Do not promise exact ggplot2, Matplotlib, or seaborn pixels.

## 15. Guided interpretation and accessibility

For an appropriate plot, BioLang guidance may explain:

- why the plot fits the variables/question;
- visual clues to inspect;
- misleading patterns;
- a useful alternative;
- small/large-sample cautions;
- transformations or normalisation;
- assumptions behind fits or intervals;
- visual evidence versus a formal test;
- what cannot be concluded.

Examples:

- histogram explains bin sensitivity and suggests ECDF/strip alternatives;
- boxplot notes hidden multimodality and observations;
- Q-Q explains why its reference line is not expected to lie at zero;
- KDE warns about bandwidth and boundary leakage;
- correlation warns about nonlinearity, groups, and outliers;
- regression distinguishes confidence and prediction bands;
- p-value diagrams separate evidence under the null from effect size;
- survival plots show censoring and numbers at risk.

Every new/materially changed plot should provide:

- meaningful title;
- axes and units;
- short alt-text summary;
- structured geometry/table access;
- sufficient contrast;
- non-colour encodings for critical distinctions;
- configurable dimensions/font size;
- sensible legend order;
- no essential information available only on hover;
- caption for dropped data, transformation, aggregation, or sampling.

Long teaching explanations belong in the statistics book/package. The runtime
carries concise structured diagnostics.

## 16. Small and large data

Plots must state whether they show observations, aggregates, samples, or model
summaries.

For small data:

- display individual observations when possible;
- do not imply unsupported precision with a smooth density;
- prefer strip/dot overlays to an isolated boxplot;
- warn when bins, KDE, asymptotic intervals, or curves are unstable;
- preserve ties and discrete values.

For large data:

- use vector marks only below measured thresholds;
- use deterministic sampling, binning, hexbin, contours, density tiles,
  aggregation, or rasterisation;
- disclose the reduction method;
- preserve rare groups/tails;
- calculate annotations from exact summaries even when marks are reduced;
- bound intermediate and output memory;
- keep a reproducible seed where sampling occurs.

## 17. Performance and memory measurements

Measure before selecting a new renderer.

Requirements:

- reuse geometry instead of recalculating statistics per backend;
- stream transformations where feasible;
- avoid duplicating dense datasets merely to choose displayed values;
- bound scene and pixel-buffer sizes;
- cap terminal output dimensions;
- avoid repeated fitting during redraw;
- use spatial indexing/aggregation for dense hit testing;
- preserve determinism when parallelising;
- feature-gate renderer dependencies;
- report CLI, WASM, desktop, and optional-feature size deltas.

Benchmark:

- cold start;
- first plot;
- subsequent plot;
- geometry calculation;
- SVG construction;
- PNG/raster construction;
- browser parse/render where measurable;
- export;
- peak resident memory;
- output bytes;
- DOM node count for SVG;
- visual correctness at normal and zoomed views.

Compare BioLang and R/Python only when both perform equivalent calculation and
output work.

## 18. Licensing and provenance

Preserve BioLang's MIT licensing intent.

- implement statistical methods independently from papers, standards, and open
  algorithm documentation;
- use R/Python packages as external black-box validators where appropriate;
- record oracle versions;
- do not copy unlicensed book/repository prose, code, examples, or figures;
- preserve required notices for directly adapted permissive code;
- keep THIRD_PARTY_NOTICES accurate;
- distinguish algorithm attribution from copied-source attribution;
- do not claim package equivalence without passing evidence.

## 19. Testing

### 19.1 Unit tests

- geometry calculations;
- scales, transforms, inverse transforms;
- ticks, formatting, clipping, and layout;
- missing and invalid data;
- boundary values and ties;
- deterministic ordering and seeds;
- raster threshold decisions;
- serialization when schemas are introduced.

### 19.2 Conformance tests

- existing R/Python runner;
- real and controlled edge-case fixtures;
- scale-sensitive gates;
- versioned manifests;
- offline CI from approved fixtures;
- explicit differences.

### 19.3 Visual regression

- representative golden images;
- labels, legends, clipping, dimensions, and empty plots;
- accessibility metadata;
- vector versus raster appearance;
- documented tolerance for font/platform differences;
- never use screenshot similarity as the sole correctness gate.

### 19.4 Integration

- all REPL plot modes;
- printed versus final plot values;
- redirected output;
- SVG and PNG export;
- notebook/server display;
- static HTML fallback;
- desktop/native display;
- missing backend messages;
- dense-data limits.

## 20. Release gates

Immediate work is ready when:

- current CLI plot changes pass tests and preserve redirected SVG;
- the inventory identifies canonical names and aliases;
- histogram geometry passes its declared R/Python gates;
- validation manifests contain BioLang time and peak memory;
- dense-layer claims have reproducible benchmark evidence;
- no unsupported measured claim appears in documentation;
- edge cases and known differences are documented;
- existing scripts remain compatible.

A medium-term representation change additionally requires:

- versioned schema;
- frontend inventory;
- package-authored SVG compatibility;
- notebook migration plan if applicable;
- binary-size and dependency review;
- browser/desktop/static-export tests;
- a measured benefit over the existing SVG path.

## 21. Non-goals

- requiring R or Python for normal plotting;
- calling Docker or external services for routine plots;
- cloning ggplot2, Matplotlib, seaborn, D3, Plotly, or statsmodels;
- rewriting already working plots without a measured or correctness reason;
- promising identical pixels across platforms;
- removing SVG;
- rasterising small/vector-appropriate marks indiscriminately;
- adding GPU rendering without evidence;
- hiding aggregation, sampling, dropped data, or transformation;
- treating an attractive picture as statistical validation;
- automatically deciding a scientific conclusion from a plot;
- creating many crates before the abstraction is proven.

## 22. Delivery sequence

### Stage 0: finish baseline

- test and isolate the existing CLI plot work;
- record current exact binary/WASM sizes;
- confirm redirected-output compatibility.

### Stage 1: inventory

- enumerate builtins and package plots;
- map aliases to canonical names;
- classify conformance, geometry, and performance risk.

### Stage 2: histogram vertical slice

- define explicit histogram conventions;
- expose geometry;
- validate against R and NumPy;
- render existing SVG/PNG/terminal from one geometry;
- benchmark real and edge-case inputs.

### Stage 3: geometry and validation expansion

- version the geometry schema;
- connect existing data helpers;
- extend existing validation manifests;
- prioritise high-risk plots.

### Stage 4: dense layers

- benchmark high-cardinality plots;
- apply thresholded add_point_raster;
- validate SVG/vector annotations and raster mark layers;
- decide whether embedded PNG is sufficient.

### Stage 5: triggered architecture

Only after an approved trigger:

- SceneNode;
- typed Plot;
- Canvas;
- versioned notebook persistence;
- separate raster blobs;
- GPU renderer;
- optional PDF.

## 23. Open decisions

1. Historical 20-bin histogram default versus a versioned conventional default.
2. Exact canonical names and alias policy.
3. Geometry Record/Table schema details.
4. Which real datasets become plot conformance fixtures.
5. Raster threshold per plot family.
6. Data-URI versus separate blob after measurement.
7. When Canvas interaction work is scheduled.
8. Record-recognised Plot versus a new core Value variant.
9. BioLang-level plot construction for package-authored plots.
10. BLN container version and migration rules.
11. Publication export requirements beyond SVG/high-DPI PNG.
12. Whether any measured workload justifies GPU rendering.

## 24. Immediate next action

The inventory, histogram conformance slice, dense-layer benchmark, and first
geometry expansion are now implemented in the working tree. Scatter, line,
error-bar and confidence-band plots use a renderer-neutral
`biolang.plot.spec/v1` Record; box, ECDF, normal Q-Q and violin/KDE calculations
use `biolang.plot.geometry/v1`. SVG, terminal text and standalone HTML/Canvas
fallback output originate from the same Cartesian specification. Independent
R and NumPy validation covers the numerical geometry rather than pixels.

Runtime violin and density renderers now share the validated Gaussian KDE and
bandwidth implementation. Guided distribution, grouped-box, normal-Q-Q,
relationship and residual-Q-Q renderers now consume shared statistical
geometry, and fitted confidence/prediction coordinates have independent R and
statsmodels gates. Real `airquality` values exercise box, ECDF, Q-Q, KDE and
linear-fit geometry. Categorical counts now retain first-observed order in a
shared inspectable geometry, while missingness exposes full-data counts and a
separately bounded deterministic display grid. SVG and standalone HTML have
structural accessibility coverage for titles, descriptions, controls and the
Canvas fallback. The alias audit keeps genomic versus normal Q-Q, wide versus
long-form violin, and single versus grouped/ASCII density contracts distinct.

UMAP and FeaturePlot are now the first biological family on the shared
`biolang.plot.spec/v1` contract. `format: "spec"` exposes source row,
coordinates, group, point label, continuous feature value and resolved
publication draw rank. Replay also freezes numeric/quantile cutoffs, equal/free
aspect and the vector/raster choice. Direct and replayed UMAP and FeaturePlot
SVGs have byte-equivalence tests; 20,000-point replay retains the bounded
embedded-PNG mark layer, while standalone HTML supplies the Canvas fallback.
Non-finite coordinate pairs remain visible in provenance/warnings but are not
sent to SVG or terminal geometry.

PCA, volcano and MA now use the same inspectable contract. PCA specifications
store the computed scores and explained-variance percentages rather than the
matrix that would require a second decomposition. Volcano and MA specifications
keep raw values beside their displayed transforms, resolved thresholds, gene
labels and the classification assigned to every row. Exact replay, malformed
specification, non-finite geometry and 20,000-gene raster tests cover the three
paths without changing their default SVG figures.

Violin, dot plot and heatmap now use the inspectable contract. Both wide- and
long-form violin specifications expose the frozen KDE grid, bandwidth, sample
count and median without merging their input contracts. Single-cell dot plots
retain mean expression, detected-cell fraction and clipped per-gene z-score for
every gene-cluster pair. Generic heatmaps expose source/display row order and
the resolved colour domain; clustered heatmaps additionally retain column
order, dendrogram topology and merge heights, so replay never reclusters.
Direct and replayed SVGs have byte-equivalence tests, and malformed or
non-finite matrix data has explicit coverage.

Kaplan-Meier, ROC and forest plots now use the inspectable contract.
Kaplan-Meier freezes each distinct-time risk set, simultaneous event/censor
counts, product-limit probability and Greenwood standard error, including
first-seen group order and median survival. ROC groups tied raw scores before
updating TP/FP/TN/FN counts, making the empirical curve and trapezoidal AUC
independent of input order; precomputed curves must be finite, bounded and
monotone. Forest plots retain raw intervals, optional positive weights, the
reference line, linear/log scale and resolved display domain. Direct and replay
SVGs are byte-identical. Survival steps use one SVG path per group and dense ROC
points use one polyline, so thousands of analytical rows do not create
thousands of DOM elements.

Manhattan, genetic Q-Q and rainfall plots now use the inspectable contract.
Manhattan freezes first-observed chromosome order, cumulative offsets, raw and
transformed p-values, resolved significance, highlighting and raster/thinning
choices. Genetic Q-Q freezes sorted p-values, `(rank - 0.5) / n` positions,
λGC and an opt-in exact beta order-statistic confidence envelope. Rainfall
freezes stable within-chromosome ordering, raw distances, log display values
and duplicate-position floors. Exact replay, malformed-specification and dense
raster tests cover all three paths.

Ideogram, CNV and coverage tracks now use the inspectable contract. Ideograms
retain every cytoband, standard stain class and first-observed chromosome order,
and draw all chromosomes on one shared length scale. CNV profiles freeze actual
genomic segment starts and ends, cumulative offsets, ratios, gain/loss thresholds
and classifications instead of reconstructing widths around offset midpoints.
Coverage tracks distinguish point samples from half-open intervals, require an
explicit chromosome for multi-chromosome input, and clip overlapping intervals
to a requested region instead of filtering their midpoints. Dense inputs remain
complete in the specification while fixed path layers bound SVG element counts.
All three replay exactly and their numeric geometry passes independent base-R
gates.

Regional annotation and splicing now use the inspectable contract.
`genome_track` freezes original and region-clipped feature bounds, strand,
stable source order, label decisions and greedy non-overlapping lanes.
`lollipop` freezes the sequence domain (including the previously ignored
`length` option), stem heights and collision-limited label lanes. `sashimi`
freezes sorted coverage, complete in-region junctions, greedy arc lanes,
square-root count scaling and quantised stroke widths. Direct and replayed SVGs
are byte-identical; dense marks are grouped into bounded path layers without
dropping analytical rows.

Multi-track circular genomes now use the inspectable contract too. `circos`
freezes chromosome-length-weighted arcs, typed radial tracks, point and interval
endpoints, chord/ribbon geometry, label decisions and count-scaled link widths.
The renderer groups dense links and marks into bounded paths while keeping every
source row in the specification. Direct/replay SVG is byte-identical, malformed
or tampered angular/radial coordinates are rejected, and 83 scale-sensitive
values pass independently calculated base-R gates.

`plot_grid` is the first explicit figure-composition contract. It freezes equal
panel cells, spreadsheet-style tags, shared outer labels, captions and explicit
legends while retaining each safe child SVG. It does not claim semantic axis
alignment inside arbitrary child plots; that remains a scene-level capability.
Publication export now supports physical SVG dimensions, controlled font stacks
and exact-DPI PNG output. A new core `Value::Plot`, BLN file-format migration,
embedded font files, direct Canvas scientific engine, native vector PDF, crate
split or GPU renderer remains deferred until compatibility work demonstrates a
concrete need.

### Publication-theme milestone

The first presentation layer is now implemented as an opt-in path. Generic
Cartesian plots, UMAP and FeaturePlot share publication typography, adaptive
margins, grid/axis tokens, title/subtitle/caption placement and external legend
space. UMAP expands its coordinate domains to preserve equal x/y units without
distorting the embedding. FeaturePlot uses a perceptually ordered continuous
ramp, exposes missing-value colour and numeric or quantile cutoffs, and draws
high feature values last. The legacy and Seurat palette paths remain compatible;
`theme: "publication"` is explicit while its visual regressions mature.

Violin and dot plots now use the same opt-in publication presentation layer.
Violin plots preserve the Gaussian KDE grid, bandwidth and median geometry
while adding horizontal guides and adaptive category labels. Dot plots preserve
their detection-rate and per-gene z-score calculations, add a gridded matrix,
and use a perceptually balanced diverging scale whose neutral midpoint is zero.
Structural gallery tests render both at 800 px notebook, 321 px (85 mm) and
680 px (180 mm) logical widths and reject missing labels, guides, metadata or
non-finite SVG coordinates. PNG visual inspection remains a release check; it
does not replace geometry assertions.

Heatmaps and annotated marker panels now use the opt-in publication layer. The
generic heatmap still preserves input order, or sorts rows by mean only when
`cluster: true` is explicit. `clustered_heatmap` retains its existing
deterministic nearest-neighbour traversal from the first row and column by
default; tests freeze both row and column order. `order: "hierarchical"` opts
into actual agglomerative clustering and row/column dendrograms, with explicit
Euclidean or Manhattan distance and complete, average, single or Ward D2
linkage. The renderer adds adaptive labels, named colour guides,
missing-value handling, subtitles/captions and automatic
zero-centred diverging colour for signed data. Structural checks cover notebook,
85 mm and 180 mm widths. Single-cell marker helpers pass numeric matrices with
explicit gene and group labels, so annotation text never enters clustering
distance calculations.

Hierarchical mode is deliberately opt-in: the historical default is not
silently reinterpreted. Its leaf order and scale-sensitive merge heights are
validated against base R `hclust(dist(x))` fixtures for complete, average,
single and Ward D2 linkage. The SVG records distance, linkage, dendrogram mode,
row/column leaf order and every merge height in machine-readable metadata.
Distance storage reuses merged cluster slots, bounding the clustering grid at
`n x n` values instead of allocating a `2n x 2n` grid.

The publication presentation option is now enforced across the complete
website biological-plot gallery rather than only the newer specification
renderers. `hic_map`, `oncoprint`, wide `violin`, grouped `density`, `pca_plot`,
`sequence_logo`, `phylo_tree`, `venn`, and `upset` previously accepted the
option but silently constructed a legacy canvas. They now use the requested
theme and preserve custom title, subtitle, and caption text. Renderer tests
freeze this contract, and the gallery generator records the theme read from
each produced SVG so stale or ignored options fail validation.
