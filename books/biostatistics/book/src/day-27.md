# Day 27: Reproducible Statistical Analysis

<div class="day-meta">
<span class="badge">Day 27 of 30</span>
<span class="badge">Prerequisites: All previous days</span>
<span class="badge">~55 min reading</span>
<span class="badge">Best Practices</span>
</div>

## The Problem

It is 11 PM on a Thursday. Your collaborator emails: "The sequencing core re-processed 3 samples with updated base-calling. Can you re-run the entire analysis with the updated data? The manuscript revision is due Monday."

You open your analysis folder. There are 14 scripts with names like `analysis_v2_final_FINAL.bl`, `test_new.bl`, and `run_this_one.bl`. You cannot remember which scripts to run in which order. One script hardcodes a file path that no longer exists. Another uses a random seed that you never recorded, so bootstrap confidence intervals will not match the figures in the manuscript. A third script produces slightly different p-values depending on whether you run it before or after another script, because they share a global variable.

This is not a hypothetical scenario. It is the daily reality of computational biology. And it is entirely preventable. Reproducible analysis is not about perfection — it is about structure, documentation, and discipline. Today, you will learn the practices that make "re-run everything" a one-command operation instead of a week of panic.

## Why Reproducibility Matters

On Day 1, we discussed the reproducibility crisis: 89% of landmark cancer biology studies could not be replicated. Computational analyses are theoretically easier to reproduce than wet-lab experiments — you have all the inputs and instructions. Yet in practice, computational reproducibility is shockingly rare.

A 2019 study attempted to reproduce analyses from 204 published bioinformatics papers. Only 14% could be reproduced from the provided code and data. The failures were rarely due to errors in logic — they were due to missing files, undocumented dependencies, hardcoded paths, unrecorded random seeds, and ambiguous analysis steps.

Journals increasingly require:
- **Code availability**: deposit analysis scripts in a public repository
- **Data availability**: raw data in GEO, SRA, or similar
- **Computational environment**: specify software versions
- **Reproducibility statement**: confirm that provided code reproduces all figures

> **Key insight:** Reproducibility is not just about other people reproducing your work. It is about future-you reproducing your work. The person most likely to need to re-run your analysis is yourself, six months later, when a reviewer asks for a revision.

## Random Seeds: The Foundation of Reproducibility

Any analysis involving randomness — bootstrap, permutation tests, cross-validation, simulation, stochastic algorithms — will produce different results each run unless you fix the random seed.

### Setting Seeds in BioLang

```bio
set_seed(42)
# ALWAYS set a seed at the start of any analysis with randomness
# Note: set_seed() is planned but not yet implemented in BioLang.
# For now, random results may vary between runs.

# Once available, these will produce identical results every time
# Bootstrap the median
let n_boot = 10000
let boot_medians = range(0, n_boot) |> map(|i| {
  let resampled = range(0, len(data)) |> map(|j| data[random_int(0, len(data) - 1)])
  median(resampled)
})
```

### Seed Best Practices

<div style="text-align: center; margin: 2em 0;">
<svg width="640" height="200" viewBox="0 0 640 200" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="320" y="22" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">Seed Management — Determinism vs Randomness</text>
  <!-- Left: Same seed → Same results -->
  <rect x="20" y="40" width="280" height="145" rx="8" fill="#f0fdf4" stroke="#16a34a" stroke-width="1"/>
  <text x="160" y="60" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Deterministic (Same Seed)</text>
  <!-- Run 1 -->
  <rect x="40" y="72" width="100" height="24" rx="4" fill="#2563eb"/>
  <text x="90" y="88" text-anchor="middle" font-size="10" fill="white">set_seed(42)</text>
  <path d="M 142 84 L 160 84" stroke="#374151" stroke-width="1.2" marker-end="url(#arrowDark27)"/>
  <rect x="162" y="72" width="120" height="24" rx="4" fill="#16a34a"/>
  <text x="222" y="88" text-anchor="middle" font-size="10" fill="white">CI: [2.31, 4.87]</text>
  <!-- Run 2 -->
  <rect x="40" y="104" width="100" height="24" rx="4" fill="#2563eb"/>
  <text x="90" y="120" text-anchor="middle" font-size="10" fill="white">set_seed(42)</text>
  <path d="M 142 116 L 160 116" stroke="#374151" stroke-width="1.2" marker-end="url(#arrowDark27)"/>
  <rect x="162" y="104" width="120" height="24" rx="4" fill="#16a34a"/>
  <text x="222" y="120" text-anchor="middle" font-size="10" fill="white">CI: [2.31, 4.87]</text>
  <!-- Checkmark -->
  <text x="160" y="152" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Identical results</text>
  <!-- Right: Different seed → Different results -->
  <rect x="340" y="40" width="280" height="145" rx="8" fill="#fef2f2" stroke="#dc2626" stroke-width="1"/>
  <text x="480" y="60" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Random (No Seed)</text>
  <!-- Run 1 -->
  <rect x="360" y="72" width="100" height="24" rx="4" fill="#6b7280"/>
  <text x="410" y="88" text-anchor="middle" font-size="10" fill="white">no seed set</text>
  <path d="M 462 84 L 480 84" stroke="#374151" stroke-width="1.2" marker-end="url(#arrowDark27)"/>
  <rect x="482" y="72" width="120" height="24" rx="4" fill="#dc2626"/>
  <text x="542" y="88" text-anchor="middle" font-size="10" fill="white">CI: [2.18, 4.95]</text>
  <!-- Run 2 -->
  <rect x="360" y="104" width="100" height="24" rx="4" fill="#6b7280"/>
  <text x="410" y="120" text-anchor="middle" font-size="10" fill="white">no seed set</text>
  <path d="M 462 116 L 480 116" stroke="#374151" stroke-width="1.2" marker-end="url(#arrowDark27)"/>
  <rect x="482" y="104" width="120" height="24" rx="4" fill="#ef4444"/>
  <text x="542" y="120" text-anchor="middle" font-size="10" fill="white">CI: [2.44, 4.72]</text>
  <!-- X mark -->
  <text x="480" y="152" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Different each run</text>
  <defs>
    <marker id="arrowDark27" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#374151"/>
    </marker>
  </defs>
</svg>
</div>

| Practice | Why |
|---|---|
| Set seed at the top of every script | Ensures full reproducibility |
| Use a fixed, documented number | Avoid `set_seed(current_time())` |
| Record the seed in your results | Others can verify |
| Use different seeds for sensitivity | Check that conclusions do not depend on one seed |

```bio
set_seed(42)
# Sensitivity check: run with multiple seeds
let seeds = [42, 123, 456, 789, 2024]

for s in seeds {
  # set_seed(s) — not yet implemented; planned for a future release
  let boot_vals = range(0, 10000) |> map(|i| {
    let resampled = range(0, len(data)) |> map(|j| data[random_int(0, len(data) - 1)])
    median(resampled)
  })
  let sorted_b = sort(boot_vals)
  let ci_lo = sorted_b[250]
  let ci_hi = sorted_b[9749]
  print("Seed " + str(s) + ": CI = [" +
    str(round(ci_lo, 3)) + ", " +
    str(round(ci_hi, 3)) + "]")
}
```

> **Common pitfall:** Setting the seed inside a loop resets the random state on every iteration, which can create subtle correlations. Set it once at the top of the script, or set it deliberately when you need a specific, documented behavior.

## Script Structure: The Analysis Pipeline

Every analysis script should follow a predictable structure:

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="120" viewBox="0 0 680 120" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">Analysis Pipeline — Standard Structure</text>
  <!-- Step 1: Config -->
  <rect x="12" y="42" width="78" height="38" rx="6" fill="#2563eb" opacity="0.9"/>
  <text x="51" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Config</text>
  <text x="51" y="70" text-anchor="middle" font-size="8" fill="#93c5fd">params, paths</text>
  <!-- Arrow -->
  <path d="M 92 61 L 104 61" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGray27)"/>
  <!-- Step 2: Load -->
  <rect x="106" y="42" width="78" height="38" rx="6" fill="#2563eb" opacity="0.85"/>
  <text x="145" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Load Data</text>
  <text x="145" y="70" text-anchor="middle" font-size="8" fill="#93c5fd">read, validate</text>
  <path d="M 186 61 L 198 61" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGray27)"/>
  <!-- Step 3: QC -->
  <rect x="200" y="42" width="78" height="38" rx="6" fill="#3b82f6" opacity="0.9"/>
  <text x="239" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">QC</text>
  <text x="239" y="70" text-anchor="middle" font-size="8" fill="#bfdbfe">clean, filter</text>
  <path d="M 280 61 L 292 61" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGray27)"/>
  <!-- Step 4: Analysis -->
  <rect x="294" y="42" width="78" height="38" rx="6" fill="#7c3aed" opacity="0.9"/>
  <text x="333" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Analysis</text>
  <text x="333" y="70" text-anchor="middle" font-size="8" fill="#c4b5fd">tests, models</text>
  <path d="M 374 61 L 386 61" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGray27)"/>
  <!-- Step 5: Visualize -->
  <rect x="388" y="42" width="78" height="38" rx="6" fill="#7c3aed" opacity="0.8"/>
  <text x="427" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Visualize</text>
  <text x="427" y="70" text-anchor="middle" font-size="8" fill="#c4b5fd">plots, figures</text>
  <path d="M 468 61 L 480 61" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGray27)"/>
  <!-- Step 6: Report -->
  <rect x="482" y="42" width="78" height="38" rx="6" fill="#16a34a" opacity="0.9"/>
  <text x="521" y="57" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Report</text>
  <text x="521" y="70" text-anchor="middle" font-size="8" fill="#bbf7d0">save, export</text>
  <!-- Reproducibility arrow underneath -->
  <path d="M 560 80 Q 570 105 340 105 Q 110 105 50 80" fill="none" stroke="#16a34a" stroke-width="1.5" stroke-dasharray="5,3" marker-start="url(#arrowGreen27)"/>
  <text x="340" y="115" text-anchor="middle" font-size="10" fill="#16a34a" font-style="italic">set_seed(42) ensures identical re-run</text>
  <defs>
    <marker id="arrowGray27" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#9ca3af"/>
    </marker>
    <marker id="arrowGreen27" markerWidth="6" markerHeight="6" refX="1" refY="3" orient="auto">
      <path d="M6,0 L0,3 L6,6 Z" fill="#16a34a"/>
    </marker>
  </defs>
</svg>
</div>

```
1. Configuration     — parameters, paths, thresholds
2. Data loading      — read files, validate inputs
3. Preprocessing     — cleaning, normalization, filtering
4. Analysis          — statistical tests, models
5. Results           — tables, summaries
6. Visualization     — publication figures
7. Output            — save results, figures, reports
```

### Example: Well-Structured Analysis

```bio
# ============================================
# Differential Expression Analysis
# Author: Your Name
# Date: 2025-03-15
# Input: counts_matrix.csv, sample_info.csv
# Output: de_results.csv, volcano.svg, heatmap.svg
# ============================================

# --- 1. Configuration ---
# set_seed(42) — not yet implemented; planned for a future release

let CONFIG = {
  input_counts: "data/counts_matrix.csv",
  input_samples: "data/sample_info.csv",
  output_dir: "results/",
  fc_threshold: 1.0,           # log2 fold change
  fdr_threshold: 0.05,
  n_top_genes: 50,             # for heatmap
  n_bootstrap: 10000
}

# --- 2. Data Loading ---
let counts = read_csv(CONFIG.input_counts)
let samples = read_csv(CONFIG.input_samples)

print("Loaded " + str(nrow(counts)) + " genes x " + str(ncol(counts)) + " samples")
print("Groups: " + str(unique(samples.group)))

# --- 3. Preprocessing ---
# Filter low-expression genes (at least 10 counts in 3+ samples)
let keep = counts |> filter_rows(|row| count(row, |x| x >= 10) >= 3)
print("Genes after filtering: " + str(nrow(keep)))

# Normalize: log2(CPM + 1)
let lib_sizes = col_sums(keep)
let cpm = keep |> map_cells(|x, col| x / lib_sizes[col] * 1e6)
let log_cpm = cpm |> map_cells(|x, _| log2(x + 1))

# --- 4. Analysis ---
let tumor_idx = samples |> which(|s| s.group == "tumor")
let normal_idx = samples |> which(|s| s.group == "normal")

let de_results = []
for gene in row_names(log_cpm) {
  let tumor_vals = log_cpm[gene] |> select(tumor_idx)
  let normal_vals = log_cpm[gene] |> select(normal_idx)
  let tt = ttest(tumor_vals, normal_vals)
  let fc = mean(tumor_vals) - mean(normal_vals)
  de_results = de_results + [{
    gene: gene,
    log2fc: fc,
    p_value: tt.p_value,
    mean_tumor: mean(tumor_vals),
    mean_normal: mean(normal_vals)
  }]
}

# Multiple testing correction
let p_vals = de_results |> map(|r| r.p_value)
let adj_p = p_adjust(p_vals, "BH")
for i in 0..len(de_results) {
  de_results[i].adj_p = adj_p[i]
}

# --- 5. Results ---
let sig_genes = de_results
  |> filter(|r| r.adj_p < CONFIG.fdr_threshold && abs(r.log2fc) > CONFIG.fc_threshold)
  |> sort_by(|r| r.adj_p)

print("\nSignificant DE genes: " + str(len(sig_genes)))
print("  Upregulated: " + str(count(sig_genes, |g| g.log2fc > 0)))
print("  Downregulated: " + str(count(sig_genes, |g| g.log2fc < 0)))

# --- 6. Visualization ---
let de_tbl = de_results |> to_table()
volcano(de_tbl,
  {fc_threshold: CONFIG.fc_threshold,
  p_threshold: CONFIG.fdr_threshold,
  title: "Tumor vs Normal — Differential Expression"})

let top_genes = sig_genes |> take(CONFIG.n_top_genes) |> map(|g| g.gene)
heatmap(log_cpm |> select_rows(top_genes),
  {cluster_rows: true, cluster_cols: true,
  title: "Top " + str(CONFIG.n_top_genes) + " DE Genes"})

# --- 7. Output ---
write_csv(de_results, CONFIG.output_dir + "de_results.csv")
write_csv(sig_genes, CONFIG.output_dir + "sig_genes.csv")
print("\nResults saved to " + CONFIG.output_dir)
```

## Modular Functions

As analyses grow complex, extract repeated logic into functions. This avoids copy-paste errors and makes the analysis self-documenting.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="200" viewBox="0 0 650 200" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="22" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">Modular Script Architecture</text>
  <!-- config.bl -->
  <rect x="30" y="50" width="100" height="50" rx="6" fill="#2563eb"/>
  <text x="80" y="72" text-anchor="middle" font-size="11" fill="white" font-weight="bold">config.bl</text>
  <text x="80" y="88" text-anchor="middle" font-size="9" fill="#93c5fd">seeds, paths,</text>
  <text x="80" y="98" text-anchor="middle" font-size="9" fill="#93c5fd">thresholds</text>
  <!-- Arrow from config to load -->
  <path d="M 132 75 L 152 75" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowMod27)"/>
  <!-- load.bl -->
  <rect x="154" y="50" width="100" height="50" rx="6" fill="#3b82f6"/>
  <text x="204" y="72" text-anchor="middle" font-size="11" fill="white" font-weight="bold">load.bl</text>
  <text x="204" y="88" text-anchor="middle" font-size="9" fill="#bfdbfe">read CSVs,</text>
  <text x="204" y="98" text-anchor="middle" font-size="9" fill="#bfdbfe">validate inputs</text>
  <!-- Arrow -->
  <path d="M 256 75 L 276 75" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowMod27)"/>
  <!-- analyze.bl -->
  <rect x="278" y="50" width="100" height="50" rx="6" fill="#7c3aed"/>
  <text x="328" y="72" text-anchor="middle" font-size="11" fill="white" font-weight="bold">analyze.bl</text>
  <text x="328" y="88" text-anchor="middle" font-size="9" fill="#c4b5fd">t-tests, models,</text>
  <text x="328" y="98" text-anchor="middle" font-size="9" fill="#c4b5fd">FDR correction</text>
  <!-- Arrow -->
  <path d="M 380 75 L 400 75" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowMod27)"/>
  <!-- plot.bl -->
  <rect x="402" y="50" width="100" height="50" rx="6" fill="#7c3aed" opacity="0.8"/>
  <text x="452" y="72" text-anchor="middle" font-size="11" fill="white" font-weight="bold">plot.bl</text>
  <text x="452" y="88" text-anchor="middle" font-size="9" fill="#c4b5fd">volcano, heatmap,</text>
  <text x="452" y="98" text-anchor="middle" font-size="9" fill="#c4b5fd">forest plots</text>
  <!-- Arrow -->
  <path d="M 504 75 L 524 75" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowMod27)"/>
  <!-- report.bl -->
  <rect x="526" y="50" width="100" height="50" rx="6" fill="#16a34a"/>
  <text x="576" y="72" text-anchor="middle" font-size="11" fill="white" font-weight="bold">report.bl</text>
  <text x="576" y="88" text-anchor="middle" font-size="9" fill="#bbf7d0">save CSV,</text>
  <text x="576" y="98" text-anchor="middle" font-size="9" fill="#bbf7d0">write summary</text>
  <!-- Main script at bottom -->
  <rect x="200" y="130" width="250" height="40" rx="6" fill="#1e293b"/>
  <text x="325" y="155" text-anchor="middle" font-size="12" fill="white" font-weight="bold">main.bl — imports all modules</text>
  <!-- Lines from main to each module -->
  <line x1="250" y1="130" x2="80" y2="102" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="280" y1="130" x2="204" y2="102" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="325" y1="130" x2="328" y2="102" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="370" y1="130" x2="452" y2="102" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="400" y1="130" x2="576" y2="102" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <!-- Benefits -->
  <text x="325" y="192" text-anchor="middle" font-size="10" fill="#6b7280" font-style="italic">Each module is testable, reusable, and independently version-controlled</text>
  <defs>
    <marker id="arrowMod27" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#9ca3af"/>
    </marker>
  </defs>
</svg>
</div>

```bio
# --- Helper functions ---

fn normalize_cpm(counts) {
  let lib_sizes = col_sums(counts)
  counts |> map_cells(|x, col| log2(x / lib_sizes[col] * 1e6 + 1))
}

fn run_de(expr, group_a_idx, group_b_idx) {
  let results = []
  for gene in row_names(expr) {
    let a_vals = expr[gene] |> select(group_a_idx)
    let b_vals = expr[gene] |> select(group_b_idx)
    let tt = ttest(a_vals, b_vals)
    results = results + [{
      gene: gene,
      log2fc: mean(a_vals) - mean(b_vals),
      p_value: tt.p_value
    }]
  }
  let adj = p_adjust(results |> map(|r| r.p_value), "BH")
  for i in 0..len(results) { results[i].adj_p = adj[i] }
  results
}

fn filter_significant(results, fc_cut, fdr_cut) {
  results |> filter(|r| r.adj_p < fdr_cut && abs(r.log2fc) > fc_cut)
}

# --- Main analysis (now concise and readable) ---
# set_seed(42) — not yet implemented; planned for a future release
let expr = read_csv("data/counts.csv") |> normalize_cpm()
let de = run_de(expr, tumor_idx, normal_idx)
let sig = filter_significant(de, 1.0, 0.05)
print("Significant genes: " + str(len(sig)))
```

## Parameter Files

Hardcoding thresholds in scripts is brittle. Extract parameters into a configuration file that lives alongside the analysis.

### config.yaml

```yaml
# Analysis parameters — Differential Expression
# Change these and re-run to explore sensitivity

random_seed: 42
input:
  counts: "data/counts_matrix.csv"
  samples: "data/sample_info.csv"
analysis:
  fc_threshold: 1.0
  fdr_threshold: 0.05
  min_counts: 10
  min_samples: 3
  n_bootstrap: 10000
output:
  dir: "results/"
  n_top_genes: 50
```

### Loading Parameters in BioLang

```bio
# Load configuration
let config = read_yaml("config.yaml")
# set_seed(config.random_seed) — not yet implemented; planned for a future release

let counts = read_csv(config.input.counts)
let sig = de_results |> filter(|r|
  r.adj_p < config.analysis.fdr_threshold &&
  abs(r.log2fc) > config.analysis.fc_threshold
)
```

### Benefits of Parameter Files

| Benefit | Explanation |
|---|---|
| Sensitivity analysis | Change one number, re-run everything |
| Documentation | All assumptions in one place |
| Collaboration | Collaborator adjusts parameters without editing code |
| Reproducibility | Record exact parameters used |
| Version control | git diff shows exactly what changed |

## Literate Analysis

Literate analysis interleaves code, results, and narrative explanation in a single document. The document is both the analysis and its documentation.

```bio
# === Section 1: Quality Control ===
# We first check for outliers using PCA on the full expression matrix.
# Any sample > 3 SD from the centroid will be flagged.

let pca_result = pca(log_cpm)
scatter(pca_result.scores[0], pca_result.scores[1],
  {title: "PCA — Quality Control"})

# Result: Sample S14 is a clear outlier on PC1 (3.8 SD from centroid).
# Decision: Remove S14 from downstream analysis.
# Justification: S14 had the lowest library size (2.1M reads vs
# median 15.3M) and highest duplication rate (82%).
```

> **Key insight:** Every decision in an analysis (removing a sample, choosing a threshold, selecting a normalization method) should be documented with a justification. Six months later, neither you nor a reviewer will remember why you chose FDR < 0.05 instead of 0.01.

## Version Tracking

Track your analysis with version control (git). This provides:

1. **History**: See exactly what changed between runs
2. **Rollback**: Undo mistakes by reverting to a previous version
3. **Collaboration**: Multiple people can work on the same analysis
4. **Provenance**: Link each figure in the manuscript to the exact code that generated it

### Minimum Version Control Workflow

```
project/
  config.yaml          # Parameters
  analysis.bl          # Main analysis script
  helpers.bl           # Reusable functions
  data/                # Input data (track metadata, not raw data)
    README.md          # Data provenance and download instructions
  results/             # Output (may or may not track)
    de_results.csv
    figures/
  .gitignore           # Exclude large data files
```

### Key Rules

| Rule | Why |
|---|---|
| Commit before and after major changes | Creates a clean timeline |
| Never commit large data files | Use .gitignore; document download instructions |
| Tag releases | `git tag v1.0-submission` marks the manuscript version |
| Write meaningful commit messages | "Fix FDR threshold" > "update" |
| Track your config file | Most important file to version |

## Putting It All Together: Restructuring an Analysis

Let us take a messy analysis from earlier chapters and restructure it into a reproducible pipeline.

### Before (messy):

```bio
# quick analysis
let d = read_csv("data/expression.csv")
let a = d |> filter(|r| r.group == "A") |> map(|r| r.value)
let b = d |> filter(|r| r.group == "B") |> map(|r| r.value)
print(ttest(a, b))
# p = 0.003 — hardcoded from a previous run
histogram(a, {bins: 30})
# TODO: fix this later
```

### After (reproducible):

```bio
set_seed(42)
# ============================================
# Two-Group Comparison: Treatment A vs B
# Author: Lab Name
# Date: 2025-03-15
# Purpose: Test whether treatment A differs from B in enzyme activity
# ============================================

# --- Configuration ---
# set_seed(42) — not yet implemented; planned for a future release

let CONFIG = {
  input: "data/enzyme_activity.csv",
  output_dir: "results/two_group/",
  alpha: 0.05,
  n_bootstrap: 10000,
  group_col: "treatment",
  value_col: "activity",
  group_a: "A",
  group_b: "B"
}

# --- Data Loading ---
let data = read_csv(CONFIG.input)
print("Total observations: " + str(nrow(data)))
print("Group A: n=" + str(count(data, |r| r[CONFIG.group_col] == CONFIG.group_a)))
print("Group B: n=" + str(count(data, |r| r[CONFIG.group_col] == CONFIG.group_b)))

let a = data |> filter(|r| r[CONFIG.group_col] == CONFIG.group_a) |> map(|r| r[CONFIG.value_col])
let b = data |> filter(|r| r[CONFIG.group_col] == CONFIG.group_b) |> map(|r| r[CONFIG.value_col])

# --- Descriptive Statistics ---
print("\nGroup A: " + str(summary(a)))
print("Group B: " + str(summary(b)))

# --- Normality Check (visual) ---
qq_plot(a, {title: "Q-Q Plot — Group A"})
qq_plot(b, {title: "Q-Q Plot — Group B"})

# --- Primary Analysis ---
let tt = ttest(a, b)
print("\nWelch t-test:")
print("  t = " + str(round(tt.statistic, 3)))
print("  p = " + str(round(tt.p_value, 4)))
print("  95% CI for difference: [" +
  str(round(tt.ci_lower, 3)) + ", " + str(round(tt.ci_upper, 3)) + "]")

# Cohen's d (inline)
let pooled_sd = sqrt(((len(a) - 1) * pow(sd(a), 2) + (len(b) - 1) * pow(sd(b), 2)) /
  (len(a) + len(b) - 2))
let d = (mean(a) - mean(b)) / pooled_sd
print("  Cohen's d = " + str(round(d, 3)))

# --- Bootstrap Confirmation ---
let n_boot = CONFIG.n_bootstrap
let combined = concat(a, b)
let boot_diffs = range(0, n_boot) |> map(|i| {
  let ra = range(0, len(a)) |> map(|j| a[random_int(0, len(a) - 1)])
  let rb = range(0, len(b)) |> map(|j| b[random_int(0, len(b) - 1)])
  mean(ra) - mean(rb)
})
let sorted_boot = sort(boot_diffs)
let boot_ci_lo = sorted_boot[round(n_boot * 0.025, 0)]
let boot_ci_hi = sorted_boot[round(n_boot * 0.975, 0)]
print("\nBootstrap 95% CI for mean difference: [" +
  str(round(boot_ci_lo, 3)) + ", " +
  str(round(boot_ci_hi, 3)) + "]")

# --- Visualization ---
violin([a, b],
  {labels: [CONFIG.group_a, CONFIG.group_b],
  title: "Enzyme Activity by Treatment",
  ylabel: "Activity (U/L)"})

# --- Output ---
let results = {
  test: "Welch t-test",
  statistic: tt.statistic,
  p_value: tt.p_value,
  ci_lower: tt.ci_lower,
  ci_upper: tt.ci_upper,
  cohens_d: d,
  boot_ci_lower: boot_ci_lo,
  boot_ci_upper: boot_ci_hi,
  seed: 42,
  n_bootstrap: CONFIG.n_bootstrap
}
write_json(results, CONFIG.output_dir + "test_results.json")
print("\nResults saved to " + CONFIG.output_dir)

# --- Conclusion ---
if tt.p_value < CONFIG.alpha {
  print("\nConclusion: Significant difference (p = " +
    str(round(tt.p_value, 4)) + ", d = " + str(round(d, 2)) + ")")
} else {
  print("\nConclusion: No significant difference (p = " +
    str(round(tt.p_value, 4)) + ")")
}
```

**Python:**

```python
# Python reproducibility essentials
import numpy as np
import random

# Set ALL random seeds
np.random.seed(42)
random.seed(42)

# Use pathlib for cross-platform paths
from pathlib import Path
DATA_DIR = Path("data")
RESULTS_DIR = Path("results")
RESULTS_DIR.mkdir(exist_ok=True)

# Save configuration
import json
config = {"seed": 42, "alpha": 0.05, "n_bootstrap": 10000}
with open(RESULTS_DIR / "config.json", "w") as f:
    json.dump(config, f, indent=2)
```

**R:**

```r
# R reproducibility essentials
set.seed(42)

# Configuration
config <- list(
  seed = 42,
  alpha = 0.05,
  n_bootstrap = 10000,
  input = "data/enzyme_activity.csv",
  output_dir = "results/"
)

# Reproducible environment
sessionInfo()  # Record package versions
renv::snapshot() # Lock package versions with renv
```

## The Reproducibility Checklist

Before submitting a manuscript, verify:

| Check | Status |
|---|---|
| Random seed set and documented? | |
| All file paths relative (not absolute)? | |
| All parameters in config file (not hardcoded)? | |
| Scripts run in order without manual intervention? | |
| Input data available or download instructions provided? | |
| Software versions recorded? | |
| Output matches manuscript figures and tables? | |
| Collaborator can run the analysis on their machine? | |

## Exercises

1. **Seed sensitivity.** Take the bootstrap analysis from Day 23 and run it with seeds 1 through 100. Plot the distribution of bootstrap CI widths. How much do they vary? Does the choice of seed ever change your conclusion?

```bio
# Your code: 100 seeds, collect CI widths, summarize
```

2. **Restructure Day 8.** Take the t-test analysis from Day 8 and restructure it following the template above: configuration block, data loading, descriptive statistics, primary analysis, effect size, bootstrap confirmation, visualization, and output.

```bio
# Your code: complete restructured analysis
```

3. **Parameter sensitivity.** Using the DE analysis structure above, run the analysis with FDR thresholds of 0.01, 0.05, and 0.10, and FC thresholds of 0.5, 1.0, and 1.5 (9 combinations total). Report the number of significant genes for each. Which thresholds are your results most sensitive to?

```bio
# Your code: 9 parameter combinations, summary table
```

4. **Modular functions.** Write a reusable `compare_groups(group_a, group_b, config)` function that performs: descriptive stats, normality test, parametric test, non-parametric test, effect size, bootstrap CI, and visualization. Test it on two different datasets.

```bio
fn compare_groups(a, b, config) {
  # Your code: complete group comparison function
}
```

5. **End-to-end check.** Write a script that runs an analysis twice (with the same seed) and asserts that every numerical result matches. If any result differs, the script should report which value changed.

```bio
# Your code: run twice, compare all outputs, assert equality
```

## Key Takeaways

- Reproducibility is not optional — journals increasingly require it, and future-you will thank present-you for the investment.
- Always set a random seed at the start of any analysis involving randomness. Document the seed and verify that conclusions are robust to different seeds.
- Structure scripts consistently: configuration, data loading, preprocessing, analysis, results, visualization, output.
- Extract parameters into configuration files rather than hardcoding them in scripts. This enables sensitivity analysis and transparent documentation.
- Use modular functions to avoid copy-paste errors and make analyses self-documenting.
- Version control (git) provides history, rollback, collaboration, and provenance — it is the minimum requirement for professional computational work.
- The ultimate test of reproducibility: can a colleague, given your code, data, and documentation, reproduce every figure and table in your manuscript?

## What's Next

You have now mastered the individual techniques and the practices that make them trustworthy. It is time to put everything together. The final three days are capstone projects — complete, end-to-end statistical analyses of realistic datasets. Tomorrow: a clinical trial with survival endpoints, subgroup analyses, and publication figures. Day 29: a differential expression study with 15,000 genes. Day 30: a genome-wide association study with 500,000 SNPs. Each capstone integrates methods from across the entire book.
