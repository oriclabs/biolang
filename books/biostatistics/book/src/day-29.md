# Day 29: Capstone — Differential Expression Study

<div class="day-meta">
<span class="badge">Day 29 of 30</span>
<span class="badge">Capstone: Days 2-3, 8, 12-13, 20-21, 25</span>
<span class="badge">~90 min reading</span>
<span class="badge">Transcriptomics</span>
</div>

## The Problem

You receive an email from a gastroenterology collaborator: "We have RNA-seq data from 12 colon biopsies — 6 from colorectal tumors and 6 from matched normal tissue. We need to identify genes that are differentially expressed between tumor and normal, find pathways that are altered, and generate figures for a manuscript. Can you run the analysis?"

The raw data has already been aligned and quantified. You have a gene-by-sample count matrix: 15,000 genes (rows) by 12 samples (columns). Each entry is the number of sequencing reads mapped to that gene in that sample. The values range from 0 to several hundred thousand.

This is the bread and butter of computational genomics. Every RNA-seq experiment, every cancer study, every drug treatment analysis begins with some version of this pipeline. Today, you will build the complete analysis from scratch, applying methods from nearly every chapter of this book.

## The Complete DE Pipeline

The analysis follows a standard workflow:

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="110" viewBox="0 0 680 110" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="20" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">Differential Expression Pipeline Overview</text>
  <!-- Raw Counts -->
  <rect x="8" y="38" width="82" height="40" rx="6" fill="#2563eb"/>
  <text x="49" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Raw Counts</text>
  <text x="49" y="68" text-anchor="middle" font-size="8" fill="#93c5fd">15K x 12</text>
  <path d="M 92 58 L 106 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- Filter -->
  <rect x="108" y="38" width="75" height="40" rx="6" fill="#3b82f6"/>
  <text x="145" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Filter</text>
  <text x="145" y="68" text-anchor="middle" font-size="8" fill="#bfdbfe">low expr</text>
  <path d="M 185 58 L 199 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- Normalize -->
  <rect x="201" y="38" width="82" height="40" rx="6" fill="#3b82f6" opacity="0.85"/>
  <text x="242" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Normalize</text>
  <text x="242" y="68" text-anchor="middle" font-size="8" fill="#bfdbfe">log2(CPM+1)</text>
  <path d="M 285 58 L 299 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- Test -->
  <rect x="301" y="38" width="75" height="40" rx="6" fill="#7c3aed"/>
  <text x="338" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Test</text>
  <text x="338" y="68" text-anchor="middle" font-size="8" fill="#c4b5fd">t-test/gene</text>
  <path d="M 378 58 L 392 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- FDR Correct -->
  <rect x="394" y="38" width="82" height="40" rx="6" fill="#7c3aed" opacity="0.85"/>
  <text x="435" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">FDR</text>
  <text x="435" y="68" text-anchor="middle" font-size="8" fill="#c4b5fd">BH correct</text>
  <path d="M 478 58 L 492 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- Visualize -->
  <rect x="494" y="38" width="82" height="40" rx="6" fill="#16a34a"/>
  <text x="535" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Visualize</text>
  <text x="535" y="68" text-anchor="middle" font-size="8" fill="#bbf7d0">volcano, HM</text>
  <path d="M 578 58 L 592 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowDE29)"/>
  <!-- Report -->
  <rect x="594" y="38" width="76" height="40" rx="6" fill="#1e293b"/>
  <text x="632" y="55" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Report</text>
  <text x="632" y="68" text-anchor="middle" font-size="8" fill="#9ca3af">gene lists</text>
  <!-- Bottom annotation -->
  <text x="340" y="100" text-anchor="middle" font-size="10" fill="#6b7280" font-style="italic">Each step feeds into the next; PCA quality control runs after normalization</text>
  <defs>
    <marker id="arrowDE29" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#9ca3af"/>
    </marker>
  </defs>
</svg>
</div>

1. **Quality control**: Library sizes, PCA for outliers and batch effects
2. **Filtering**: Remove lowly expressed genes
3. **Normalization**: Convert raw counts to comparable expression values
4. **Differential expression**: Statistical testing per gene
5. **Multiple testing correction**: FDR control
6. **Visualization**: Volcano plot, heatmap, gene-level plots
7. **Biological interpretation**: Top genes, correlation patterns

## Section 1: Data Loading and Quality Control

```bio
set_seed(42)
# ============================================
# Differential Expression Analysis
# Colorectal Tumor vs Normal Colon
# 15,000 genes x 12 samples (6 tumor, 6 normal)
# ============================================


# --- Configuration ---
let CONFIG = {
  n_genes: 15000,
  n_tumor: 6,
  n_normal: 6,
  fc_threshold: 1.0,        # log2 fold change
  fdr_threshold: 0.05,
  min_count: 10,             # minimum count for filtering
  min_samples: 3,            # minimum samples above threshold
  n_top_heatmap: 50,
  n_top_label: 10
}

# --- Simulate realistic RNA-seq counts ---
# Most genes: low to moderate expression, no DE
# ~500 genes: truly upregulated in tumor (log2FC ~ 1.5-4)
# ~500 genes: truly downregulated in tumor (log2FC ~ -1.5 to -4)
# ~14000 genes: no true difference

let gene_names = seq(1, CONFIG.n_genes) |> map(|i| "Gene_" + str(i))

# Base expression: log-normal distribution (typical for RNA-seq)
let base_means = rnorm(CONFIG.n_genes, 100, 80)
  |> map(|x| max(1, round(abs(x), 0)))

# Generate count matrix
let counts = table(CONFIG.n_genes, 12, 0)

for g in 0..CONFIG.n_genes {
  let mu = base_means[g]
  for s in 0..12 {
    # Negative binomial-like: Poisson with overdispersion
    let lib_factor = rnorm(1, 1.0, 0.15)[0]
    let this_mu = mu * max(0.1, lib_factor)

    # Add differential expression for first 500 genes (up in tumor)
    if g < 500 && s < 6 {
      let fc = rnorm(1, 2.5, 0.8)[0]
      this_mu = this_mu * pow(2, max(0.5, fc))
    }
    # Add DE for genes 500-999 (down in tumor)
    if g >= 500 && g < 1000 && s < 6 {
      let fc = rnorm(1, -2.0, 0.7)[0]
      this_mu = this_mu * pow(2, min(-0.5, fc))
    }

    counts[g][s] = max(0, round(rpois(1, max(0.1, this_mu))[0], 0))
  }
}

let sample_names = ["T1", "T2", "T3", "T4", "T5", "T6",
                    "N1", "N2", "N3", "N4", "N5", "N6"]
let groups = repeat("Tumor", 6) + repeat("Normal", 6)

print("Count matrix: " + str(CONFIG.n_genes) + " genes x 12 samples")
```

### Library Size Check

Library size (total reads per sample) should be roughly comparable. Large differences indicate technical problems.

```bio
# Library sizes
let lib_sizes = col_sums(counts)

print("\n=== Library Sizes ===")
for i in 0..12 {
  print(sample_names[i] + " (" + groups[i] + "): " +
    str(round(lib_sizes[i] / 1e6, 2)) + "M reads")
}

bar_chart(sample_names, lib_sizes |> map(|x| x / 1e6),
  {title: "Library Sizes",
  ylabel: "Millions of Reads"})

# Flag samples with library size < 50% or > 200% of median
let med_lib = median(lib_sizes)
for i in 0..12 {
  let ratio = lib_sizes[i] / med_lib
  if ratio < 0.5 || ratio > 2.0 {
    print("WARNING: " + sample_names[i] + " has unusual library size (ratio=" +
      str(round(ratio, 2)) + ")")
  }
}
```

### PCA for Outlier Detection

PCA is the first tool for quality control. Samples should cluster by biological group (tumor vs normal), not by batch or other technical factors.

```bio
# Quick normalization for QC PCA: log2(CPM + 1)
let cpm = counts |> map_cells(|x, col| x / lib_sizes[col] * 1e6)
let log_cpm = cpm |> map_cells(|x, _| log2(x + 1))

# PCA on all genes
let qc_pca = pca(transpose(log_cpm))  # transpose: samples as rows

scatter(qc_pca.scores[0], qc_pca.scores[1],
  {xlabel: "PC1 (" + str(round(qc_pca.variance_explained[0] * 100, 1)) + "%)",
  ylabel: "PC2 (" + str(round(qc_pca.variance_explained[1] * 100, 1)) + "%)",
  title: "PCA — Quality Control (All Genes)"})

# Check: PC1 should separate tumor from normal
print("\n=== PCA Quality Control ===")
print("PC1 explains " + str(round(qc_pca.variance_explained[0] * 100, 1)) + "% of variance")
print("PC2 explains " + str(round(qc_pca.variance_explained[1] * 100, 1)) + "% of variance")

# Identify outliers (>3 SD from group centroid)
let tumor_pc1 = qc_pca.scores[0] |> select(0..6)
let normal_pc1 = qc_pca.scores[0] |> select(6..12)
let tumor_pc2 = qc_pca.scores[1] |> select(0..6)
let normal_pc2 = qc_pca.scores[1] |> select(6..12)

for i in 0..6 {
  let dist_from_center = sqrt(
    pow(tumor_pc1[i] - mean(tumor_pc1), 2) +
    pow(tumor_pc2[i] - mean(tumor_pc2), 2))
  if dist_from_center > 3 * sd(tumor_pc1) {
    print("WARNING: " + sample_names[i] + " is a potential outlier (tumor group)")
  }
}
```

### Sample Correlation Heatmap

```bio
# Correlation matrix between samples
let cor_mat = cor_matrix(transpose(log_cpm))

heatmap(cor_mat,
  {color_scale: "red_blue",
  title: "Sample-Sample Correlation Heatmap",
  cluster_rows: true,
  cluster_cols: true})

# All same-group correlations should be high (>0.9)
let min_within_tumor = 1.0
let min_within_normal = 1.0
for i in 0..6 {
  for j in (i+1)..6 {
    min_within_tumor = min(min_within_tumor, cor_mat[i][j])
    min_within_normal = min(min_within_normal, cor_mat[i+6][j+6])
  }
}
print("\nMinimum within-tumor correlation: " + str(round(min_within_tumor, 3)))
print("Minimum within-normal correlation: " + str(round(min_within_normal, 3)))
```

> **Key insight:** If PCA shows samples clustering by something other than biology (e.g., batch, RNA extraction date, sequencing lane), you have a batch effect (Day 20). Address it before proceeding with DE analysis.

## Section 2: Gene Filtering

Low-expression genes add noise and multiple testing burden without contributing useful information. Filter them before testing.

```bio
# Filter: keep genes with at least min_count reads in at least min_samples
let keep = []
let remove_count = 0

for g in 0..CONFIG.n_genes {
  let above_threshold = 0
  for s in 0..12 {
    if counts[g][s] >= CONFIG.min_count {
      above_threshold = above_threshold + 1
    }
  }
  if above_threshold >= CONFIG.min_samples {
    keep = keep + [g]
  } else {
    remove_count = remove_count + 1
  }
}

print("\n=== Gene Filtering ===")
print("Genes before filtering: " + str(CONFIG.n_genes))
print("Genes removed (low expression): " + str(remove_count))
print("Genes retained: " + str(len(keep)))

# Subset to kept genes
let filtered_counts = counts |> select_rows(keep)
let filtered_names = gene_names |> select(keep)
let filtered_log_cpm = log_cpm |> select_rows(keep)
```

## Section 3: Normalization

Raw counts are not directly comparable between samples because of different library sizes. We normalize to log2 counts per million (CPM).

```bio
# Normalize: log2(CPM + 1)
let norm_expr = filtered_counts |> map_cells(|x, col|
  log2(x / lib_sizes[col] * 1e6 + 1))

print("\n=== Normalization ===")
print("Method: log2(CPM + 1)")
print("Expression range: [" +
  str(round(min_all(norm_expr), 2)) + ", " +
  str(round(max_all(norm_expr), 2)) + "]")

# Density plot of normalized expression by sample
for i in 0..12 {
  density(norm_expr |> col(i), {label: sample_names[i]})
}
```

> **Common pitfall:** CPM normalization assumes that most genes are not differentially expressed. If a large fraction of genes are DE (uncommon but possible in some cancer comparisons), CPM can introduce systematic bias. More sophisticated methods like TMM (edgeR) or median-of-ratios (DESeq2) handle this better. For typical DE analyses with <20% DE genes, log2(CPM+1) is adequate.

## Section 4: Differential Expression Testing

We test each gene individually using Welch's t-test, comparing the 6 tumor samples to the 6 normal samples.

```bio
# --- Gene-by-gene testing ---
print("\n=== Differential Expression Testing ===")
print("Testing " + str(len(keep)) + " genes (Welch t-test)...")

let de_results = []

for g in 0..len(keep) {
  let tumor_vals = norm_expr[g] |> select(0..6)
  let normal_vals = norm_expr[g] |> select(6..12)

  let log2fc = mean(tumor_vals) - mean(normal_vals)
  let tt = ttest(tumor_vals, normal_vals)
  # Cohen's d inline
  let pooled_sd = sqrt(((len(tumor_vals) - 1) * pow(sd(tumor_vals), 2) +
    (len(normal_vals) - 1) * pow(sd(normal_vals), 2)) /
    (len(tumor_vals) + len(normal_vals) - 2))
  let d = if pooled_sd > 0 { log2fc / pooled_sd } else { 0 }

  de_results = de_results + [{
    gene: filtered_names[g],
    gene_idx: keep[g],
    log2fc: log2fc,
    mean_tumor: mean(tumor_vals),
    mean_normal: mean(normal_vals),
    t_stat: tt.statistic,
    p_value: tt.p_value,
    cohens_d: d
  }]
}

# Quick check
print("Tests completed: " + str(len(de_results)))
print("Raw p < 0.05: " + str(count(de_results, |r| r.p_value < 0.05)))
print("Raw p < 0.01: " + str(count(de_results, |r| r.p_value < 0.01)))
```

## Section 5: Multiple Testing Correction (FDR)

With thousands of tests, raw p-values are unreliable. A 5% false positive rate across 10,000 tests means 500 false positives. FDR correction (Day 12) controls this.

```bio
# --- FDR correction (Benjamini-Hochberg) ---
let raw_pvals = de_results |> map(|r| r.p_value)
let adj_pvals = p_adjust(raw_pvals, "BH")

# Add adjusted p-values
for i in 0..len(de_results) {
  de_results[i].adj_p = adj_pvals[i]
}

# Identify significant genes
let sig_genes = de_results
  |> filter(|r| r.adj_p < CONFIG.fdr_threshold && abs(r.log2fc) > CONFIG.fc_threshold)
  |> sort_by(|r| r.adj_p)

let sig_up = sig_genes |> filter(|r| r.log2fc > 0)
let sig_down = sig_genes |> filter(|r| r.log2fc < 0)

print("\n=== Multiple Testing Correction ===")
print("Method: Benjamini-Hochberg (FDR)")
print("FDR threshold: " + str(CONFIG.fdr_threshold))
print("Fold change threshold: |log2FC| > " + str(CONFIG.fc_threshold))
print("")
print("Total DE genes: " + str(len(sig_genes)))
print("  Upregulated in tumor: " + str(len(sig_up)))
print("  Downregulated in tumor: " + str(len(sig_down)))

# Comparison: how many would Bonferroni find?
let bonf_pvals = p_adjust(raw_pvals, "bonferroni")
let sig_bonf = de_results
  |> filter(|r| bonf_pvals[de_results |> index_of(r)] < CONFIG.fdr_threshold &&
    abs(r.log2fc) > CONFIG.fc_threshold)
print("\nBonferroni significant: " + str(len(sig_bonf |> collect())))
print("BH recovers more true positives while controlling FDR")
```

> **Key insight:** The choice between Bonferroni and BH matters enormously in genomics. Bonferroni controls the family-wise error rate (FWER) — the probability of even one false positive — and is extremely conservative. BH controls the FDR — the proportion of false positives among all positives — and is the standard for discovery-oriented genomics.

## Section 6: Volcano Plot

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="400" viewBox="0 0 680 400" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Volcano Plot Anatomy</text>
  <!-- Axes -->
  <line x1="80" y1="340" x2="600" y2="340" stroke="#374151" stroke-width="1.5"/>
  <line x1="80" y1="40" x2="80" y2="340" stroke="#374151" stroke-width="1.5"/>
  <text x="340" y="375" text-anchor="middle" font-size="12" fill="#374151">log2 Fold Change</text>
  <text x="25" y="190" text-anchor="middle" font-size="12" fill="#374151" transform="rotate(-90 25 190)">-log10(p-value)</text>
  <!-- X-axis ticks -->
  <text x="140" y="358" text-anchor="middle" font-size="10" fill="#6b7280">-4</text>
  <text x="220" y="358" text-anchor="middle" font-size="10" fill="#6b7280">-2</text>
  <text x="340" y="358" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
  <text x="460" y="358" text-anchor="middle" font-size="10" fill="#6b7280">+2</text>
  <text x="540" y="358" text-anchor="middle" font-size="10" fill="#6b7280">+4</text>
  <!-- Y-axis ticks -->
  <text x="68" y="340" text-anchor="end" font-size="10" fill="#6b7280">0</text>
  <text x="68" y="280" text-anchor="end" font-size="10" fill="#6b7280">2</text>
  <text x="68" y="220" text-anchor="end" font-size="10" fill="#6b7280">4</text>
  <text x="68" y="160" text-anchor="end" font-size="10" fill="#6b7280">6</text>
  <text x="68" y="100" text-anchor="end" font-size="10" fill="#6b7280">8</text>
  <text x="68" y="50" text-anchor="end" font-size="10" fill="#6b7280">10</text>
  <!-- Threshold lines -->
  <line x1="80" y1="220" x2="600" y2="220" stroke="#9ca3af" stroke-width="1" stroke-dasharray="5,3"/>
  <text x="610" y="224" font-size="9" fill="#9ca3af">FDR = 0.05</text>
  <line x1="220" y1="40" x2="220" y2="340" stroke="#9ca3af" stroke-width="1" stroke-dasharray="5,3"/>
  <line x1="460" y1="40" x2="460" y2="340" stroke="#9ca3af" stroke-width="1" stroke-dasharray="5,3"/>
  <text x="220" y="38" text-anchor="middle" font-size="9" fill="#9ca3af">-FC</text>
  <text x="460" y="38" text-anchor="middle" font-size="9" fill="#9ca3af">+FC</text>
  <!-- Region labels -->
  <rect x="100" y="55" width="100" height="24" rx="4" fill="#2563eb" opacity="0.15"/>
  <text x="150" y="72" text-anchor="middle" font-size="10" fill="#2563eb" font-weight="bold">Down in Tumor</text>
  <rect x="480" y="55" width="100" height="24" rx="4" fill="#dc2626" opacity="0.15"/>
  <text x="530" y="72" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">Up in Tumor</text>
  <text x="340" y="300" text-anchor="middle" font-size="10" fill="#9ca3af">Not Significant</text>
  <!-- NS dots (gray, center, below threshold) -->
  <circle cx="300" cy="310" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="340" cy="325" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="360" cy="300" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="320" cy="290" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="280" cy="315" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="355" cy="270" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="310" cy="260" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="370" cy="250" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="290" cy="245" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="350" cy="235" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="330" cy="280" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="250" cy="305" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="400" cy="295" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="270" cy="330" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="410" cy="320" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="240" cy="270" r="3" fill="#9ca3af" opacity="0.4"/>
  <circle cx="440" cy="265" r="3" fill="#9ca3af" opacity="0.4"/>
  <!-- Upregulated dots (red, right, above threshold) -->
  <circle cx="480" cy="180" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="500" cy="140" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="520" cy="100" r="5" fill="#dc2626" opacity="0.8"/>
  <circle cx="490" cy="160" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="470" cy="200" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="530" cy="80" r="5" fill="#dc2626" opacity="0.8"/>
  <circle cx="510" cy="120" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="475" cy="190" r="4" fill="#dc2626" opacity="0.7"/>
  <circle cx="540" cy="65" r="5" fill="#dc2626" opacity="0.9"/>
  <circle cx="495" cy="150" r="4" fill="#dc2626" opacity="0.7"/>
  <!-- Downregulated dots (blue, left, above threshold) -->
  <circle cx="180" cy="170" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="160" cy="130" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="140" cy="90" r="5" fill="#2563eb" opacity="0.8"/>
  <circle cx="170" cy="150" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="190" cy="200" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="130" cy="75" r="5" fill="#2563eb" opacity="0.8"/>
  <circle cx="150" cy="110" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="200" cy="210" r="4" fill="#2563eb" opacity="0.7"/>
  <circle cx="120" cy="60" r="5" fill="#2563eb" opacity="0.9"/>
  <!-- Gene labels for top hits -->
  <text x="548" y="62" font-size="9" fill="#dc2626" font-style="italic">MYC</text>
  <text x="538" y="78" font-size="9" fill="#dc2626" font-style="italic">KRAS</text>
  <text x="110" y="57" font-size="9" fill="#2563eb" font-style="italic">TP53</text>
  <text x="120" y="73" font-size="9" fill="#2563eb" font-style="italic">APC</text>
</svg>
</div>

```bio
# --- Volcano plot ---
let all_log2fc = de_results |> map(|r| r.log2fc)
let all_adj_p = de_results |> map(|r| r.adj_p)

# Find top genes for labeling
let top_by_significance = de_results
  |> sort_by(|r| r.adj_p)
  |> take(CONFIG.n_top_label)
  |> map(|r| r.gene)

let volcano_tbl = de_results |> map(|r| {
  gene: r.gene, log2fc: r.log2fc, adj_p: r.adj_p
}) |> to_table()

volcano(volcano_tbl,
  {fc_threshold: CONFIG.fc_threshold,
  p_threshold: CONFIG.fdr_threshold,
  title: "Volcano Plot — Tumor vs Normal Colon",
  xlabel: "log2 Fold Change",
  ylabel: "-log10(FDR-adjusted p-value)"})

print("\n=== Top 10 DE Genes by Significance ===")
print("Gene            log2FC    adj_p       Cohen's d")
print("-" * 55)
for gene in de_results |> sort_by(|r| r.adj_p) |> take(10) {
  print(gene.gene + "    " +
    str(round(gene.log2fc, 2)) + "     " +
    str(round(gene.adj_p, 6)) + "   " +
    str(round(gene.cohens_d, 2)))
}
```

## Section 7: Heatmap of Top DE Genes

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="360" viewBox="0 0 680 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Heatmap Anatomy — Clustered Expression</text>
  <!-- Row dendrogram (left side) -->
  <text x="30" y="50" text-anchor="middle" font-size="9" fill="#6b7280">Row</text>
  <text x="30" y="60" text-anchor="middle" font-size="9" fill="#6b7280">Dendro.</text>
  <!-- Simple dendrogram lines -->
  <line x1="45" y1="80" x2="45" y2="120" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="100" x2="60" y2="100" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="80" x2="60" y2="80" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="120" x2="60" y2="120" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="100" x2="40" y2="180" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="140" x2="45" y2="140" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="140" x2="60" y2="140" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="160" x2="60" y2="160" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="155" x2="45" y2="165" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="180" x2="60" y2="180" stroke="#9ca3af" stroke-width="1"/>
  <line x1="35" y1="140" x2="35" y2="240" stroke="#9ca3af" stroke-width="1"/>
  <line x1="35" y1="140" x2="40" y2="140" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="200" x2="60" y2="200" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="220" x2="60" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <line x1="45" y1="200" x2="45" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="210" x2="45" y2="210" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="210" x2="40" y2="240" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="240" x2="60" y2="240" stroke="#9ca3af" stroke-width="1"/>
  <line x1="35" y1="240" x2="40" y2="240" stroke="#9ca3af" stroke-width="1"/>
  <!-- Column dendrogram (top) -->
  <text x="240" y="50" text-anchor="middle" font-size="9" fill="#6b7280">Column Dendrogram</text>
  <!-- Tumor cluster -->
  <line x1="100" y1="58" x2="100" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="140" y1="58" x2="140" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="100" y1="58" x2="140" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="180" y1="58" x2="180" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="220" y1="58" x2="220" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="180" y1="58" x2="220" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="120" y1="54" x2="120" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="200" y1="54" x2="200" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="120" y1="54" x2="200" y2="54" stroke="#9ca3af" stroke-width="1"/>
  <line x1="260" y1="58" x2="260" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="160" y1="50" x2="160" y2="54" stroke="#9ca3af" stroke-width="1"/>
  <line x1="160" y1="50" x2="260" y2="50" stroke="#9ca3af" stroke-width="1"/>
  <line x1="260" y1="50" x2="260" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <!-- Normal cluster -->
  <line x1="300" y1="58" x2="300" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="340" y1="58" x2="340" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="300" y1="58" x2="340" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="380" y1="58" x2="380" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="420" y1="58" x2="420" y2="64" stroke="#9ca3af" stroke-width="1"/>
  <line x1="380" y1="58" x2="420" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="320" y1="54" x2="320" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="400" y1="54" x2="400" y2="58" stroke="#9ca3af" stroke-width="1"/>
  <line x1="320" y1="54" x2="400" y2="54" stroke="#9ca3af" stroke-width="1"/>
  <line x1="360" y1="50" x2="360" y2="54" stroke="#9ca3af" stroke-width="1"/>
  <!-- Condition annotation bar -->
  <rect x="82" y="66" width="200" height="8" rx="2" fill="#dc2626" opacity="0.8"/>
  <rect x="282" y="66" width="160" height="8" rx="2" fill="#2563eb" opacity="0.8"/>
  <text x="182" y="63" text-anchor="middle" font-size="8" fill="#dc2626">Tumor</text>
  <text x="362" y="63" text-anchor="middle" font-size="8" fill="#2563eb">Normal</text>
  <!-- Heatmap grid (simplified 9 rows x 12 cols) -->
  <!-- Row 1 (upregulated in tumor - red in tumor cols, blue in normal) -->
  <rect x="82" y="78" width="33" height="18" fill="#dc2626" opacity="0.9"/><rect x="115" y="78" width="34" height="18" fill="#ef4444" opacity="0.85"/><rect x="149" y="78" width="33" height="18" fill="#dc2626" opacity="0.8"/><rect x="182" y="78" width="34" height="18" fill="#ef4444" opacity="0.9"/><rect x="216" y="78" width="33" height="18" fill="#dc2626" opacity="0.7"/><rect x="249" y="78" width="33" height="18" fill="#ef4444" opacity="0.8"/>
  <rect x="282" y="78" width="27" height="18" fill="#2563eb" opacity="0.8"/><rect x="309" y="78" width="27" height="18" fill="#3b82f6" opacity="0.7"/><rect x="336" y="78" width="27" height="18" fill="#2563eb" opacity="0.9"/><rect x="363" y="78" width="27" height="18" fill="#3b82f6" opacity="0.8"/><rect x="390" y="78" width="26" height="18" fill="#2563eb" opacity="0.7"/><rect x="416" y="78" width="26" height="18" fill="#3b82f6" opacity="0.9"/>
  <!-- Row 2 -->
  <rect x="82" y="98" width="33" height="18" fill="#ef4444" opacity="0.8"/><rect x="115" y="98" width="34" height="18" fill="#dc2626" opacity="0.7"/><rect x="149" y="98" width="33" height="18" fill="#ef4444" opacity="0.9"/><rect x="182" y="98" width="34" height="18" fill="#dc2626" opacity="0.8"/><rect x="216" y="98" width="33" height="18" fill="#ef4444" opacity="0.85"/><rect x="249" y="98" width="33" height="18" fill="#dc2626" opacity="0.7"/>
  <rect x="282" y="98" width="27" height="18" fill="#3b82f6" opacity="0.7"/><rect x="309" y="98" width="27" height="18" fill="#2563eb" opacity="0.8"/><rect x="336" y="98" width="27" height="18" fill="#3b82f6" opacity="0.9"/><rect x="363" y="98" width="27" height="18" fill="#2563eb" opacity="0.7"/><rect x="390" y="98" width="26" height="18" fill="#3b82f6" opacity="0.8"/><rect x="416" y="98" width="26" height="18" fill="#2563eb" opacity="0.9"/>
  <!-- Row 3 -->
  <rect x="82" y="118" width="33" height="18" fill="#dc2626" opacity="0.7"/><rect x="115" y="118" width="34" height="18" fill="#ef4444" opacity="0.9"/><rect x="149" y="118" width="33" height="18" fill="#dc2626" opacity="0.85"/><rect x="182" y="118" width="34" height="18" fill="#ef4444" opacity="0.7"/><rect x="216" y="118" width="33" height="18" fill="#dc2626" opacity="0.9"/><rect x="249" y="118" width="33" height="18" fill="#ef4444" opacity="0.8"/>
  <rect x="282" y="118" width="27" height="18" fill="#2563eb" opacity="0.9"/><rect x="309" y="118" width="27" height="18" fill="#3b82f6" opacity="0.8"/><rect x="336" y="118" width="27" height="18" fill="#2563eb" opacity="0.7"/><rect x="363" y="118" width="27" height="18" fill="#3b82f6" opacity="0.9"/><rect x="390" y="118" width="26" height="18" fill="#2563eb" opacity="0.8"/><rect x="416" y="118" width="26" height="18" fill="#3b82f6" opacity="0.7"/>
  <!-- Row 4 (neutral) -->
  <rect x="82" y="138" width="33" height="18" fill="#9ca3af" opacity="0.3"/><rect x="115" y="138" width="34" height="18" fill="#9ca3af" opacity="0.4"/><rect x="149" y="138" width="33" height="18" fill="#9ca3af" opacity="0.25"/><rect x="182" y="138" width="34" height="18" fill="#9ca3af" opacity="0.35"/><rect x="216" y="138" width="33" height="18" fill="#9ca3af" opacity="0.3"/><rect x="249" y="138" width="33" height="18" fill="#9ca3af" opacity="0.4"/>
  <rect x="282" y="138" width="27" height="18" fill="#9ca3af" opacity="0.3"/><rect x="309" y="138" width="27" height="18" fill="#9ca3af" opacity="0.35"/><rect x="336" y="138" width="27" height="18" fill="#9ca3af" opacity="0.25"/><rect x="363" y="138" width="27" height="18" fill="#9ca3af" opacity="0.4"/><rect x="390" y="138" width="26" height="18" fill="#9ca3af" opacity="0.3"/><rect x="416" y="138" width="26" height="18" fill="#9ca3af" opacity="0.35"/>
  <!-- Rows 5-7 (downregulated in tumor - blue in tumor, red in normal) -->
  <rect x="82" y="158" width="33" height="18" fill="#2563eb" opacity="0.8"/><rect x="115" y="158" width="34" height="18" fill="#3b82f6" opacity="0.7"/><rect x="149" y="158" width="33" height="18" fill="#2563eb" opacity="0.9"/><rect x="182" y="158" width="34" height="18" fill="#3b82f6" opacity="0.8"/><rect x="216" y="158" width="33" height="18" fill="#2563eb" opacity="0.7"/><rect x="249" y="158" width="33" height="18" fill="#3b82f6" opacity="0.9"/>
  <rect x="282" y="158" width="27" height="18" fill="#dc2626" opacity="0.8"/><rect x="309" y="158" width="27" height="18" fill="#ef4444" opacity="0.9"/><rect x="336" y="158" width="27" height="18" fill="#dc2626" opacity="0.7"/><rect x="363" y="158" width="27" height="18" fill="#ef4444" opacity="0.8"/><rect x="390" y="158" width="26" height="18" fill="#dc2626" opacity="0.9"/><rect x="416" y="158" width="26" height="18" fill="#ef4444" opacity="0.7"/>
  <rect x="82" y="178" width="33" height="18" fill="#3b82f6" opacity="0.9"/><rect x="115" y="178" width="34" height="18" fill="#2563eb" opacity="0.8"/><rect x="149" y="178" width="33" height="18" fill="#3b82f6" opacity="0.7"/><rect x="182" y="178" width="34" height="18" fill="#2563eb" opacity="0.9"/><rect x="216" y="178" width="33" height="18" fill="#3b82f6" opacity="0.8"/><rect x="249" y="178" width="33" height="18" fill="#2563eb" opacity="0.7"/>
  <rect x="282" y="178" width="27" height="18" fill="#ef4444" opacity="0.9"/><rect x="309" y="178" width="27" height="18" fill="#dc2626" opacity="0.7"/><rect x="336" y="178" width="27" height="18" fill="#ef4444" opacity="0.8"/><rect x="363" y="178" width="27" height="18" fill="#dc2626" opacity="0.9"/><rect x="390" y="178" width="26" height="18" fill="#ef4444" opacity="0.7"/><rect x="416" y="178" width="26" height="18" fill="#dc2626" opacity="0.8"/>
  <rect x="82" y="198" width="33" height="18" fill="#2563eb" opacity="0.7"/><rect x="115" y="198" width="34" height="18" fill="#3b82f6" opacity="0.9"/><rect x="149" y="198" width="33" height="18" fill="#2563eb" opacity="0.8"/><rect x="182" y="198" width="34" height="18" fill="#3b82f6" opacity="0.7"/><rect x="216" y="198" width="33" height="18" fill="#2563eb" opacity="0.9"/><rect x="249" y="198" width="33" height="18" fill="#3b82f6" opacity="0.8"/>
  <rect x="282" y="198" width="27" height="18" fill="#dc2626" opacity="0.7"/><rect x="309" y="198" width="27" height="18" fill="#ef4444" opacity="0.8"/><rect x="336" y="198" width="27" height="18" fill="#dc2626" opacity="0.9"/><rect x="363" y="198" width="27" height="18" fill="#ef4444" opacity="0.7"/><rect x="390" y="198" width="26" height="18" fill="#dc2626" opacity="0.8"/><rect x="416" y="198" width="26" height="18" fill="#ef4444" opacity="0.9"/>
  <!-- Gene labels (right) -->
  <text x="450" y="91" font-size="9" fill="#374151">Gene_12</text>
  <text x="450" y="111" font-size="9" fill="#374151">Gene_45</text>
  <text x="450" y="131" font-size="9" fill="#374151">Gene_7</text>
  <text x="450" y="151" font-size="9" fill="#9ca3af">Gene_8901</text>
  <text x="450" y="171" font-size="9" fill="#374151">Gene_523</text>
  <text x="450" y="191" font-size="9" fill="#374151">Gene_891</text>
  <text x="450" y="211" font-size="9" fill="#374151">Gene_672</text>
  <!-- Sample labels (bottom) -->
  <text x="98" y="228" font-size="8" fill="#374151" transform="rotate(45 98 228)">T1</text>
  <text x="132" y="228" font-size="8" fill="#374151" transform="rotate(45 132 228)">T2</text>
  <text x="165" y="228" font-size="8" fill="#374151" transform="rotate(45 165 228)">T3</text>
  <text x="199" y="228" font-size="8" fill="#374151" transform="rotate(45 199 228)">T4</text>
  <text x="232" y="228" font-size="8" fill="#374151" transform="rotate(45 232 228)">T5</text>
  <text x="265" y="228" font-size="8" fill="#374151" transform="rotate(45 265 228)">T6</text>
  <text x="295" y="228" font-size="8" fill="#374151" transform="rotate(45 295 228)">N1</text>
  <text x="322" y="228" font-size="8" fill="#374151" transform="rotate(45 322 228)">N2</text>
  <text x="349" y="228" font-size="8" fill="#374151" transform="rotate(45 349 228)">N3</text>
  <text x="376" y="228" font-size="8" fill="#374151" transform="rotate(45 376 228)">N4</text>
  <text x="403" y="228" font-size="8" fill="#374151" transform="rotate(45 403 228)">N5</text>
  <text x="429" y="228" font-size="8" fill="#374151" transform="rotate(45 429 228)">N6</text>
  <!-- Color key -->
  <text x="540" y="95" text-anchor="middle" font-size="10" font-weight="bold" fill="#374151">Z-score</text>
  <rect x="520" y="100" width="40" height="12" fill="#2563eb" opacity="0.9"/>
  <text x="570" y="110" font-size="9" fill="#6b7280">-2 (low)</text>
  <rect x="520" y="116" width="40" height="12" fill="#9ca3af" opacity="0.3"/>
  <text x="570" y="126" font-size="9" fill="#6b7280">0</text>
  <rect x="520" y="132" width="40" height="12" fill="#dc2626" opacity="0.9"/>
  <text x="570" y="142" font-size="9" fill="#6b7280">+2 (high)</text>
  <!-- Annotations -->
  <text x="540" y="180" text-anchor="middle" font-size="9" fill="#6b7280">Rows: genes</text>
  <text x="540" y="195" text-anchor="middle" font-size="9" fill="#6b7280">Cols: samples</text>
  <text x="540" y="210" text-anchor="middle" font-size="9" fill="#6b7280">Both clustered</text>
  <!-- Key insight box -->
  <rect x="82" y="270" width="520" height="40" rx="6" fill="#f0f9ff" stroke="#93c5fd" stroke-width="1"/>
  <text x="340" y="288" text-anchor="middle" font-size="10" fill="#374151">Top rows: upregulated genes (red in Tumor, blue in Normal)</text>
  <text x="340" y="302" text-anchor="middle" font-size="10" fill="#374151">Bottom rows: downregulated genes (blue in Tumor, red in Normal)</text>
  <!-- Annotation bar legend -->
  <rect x="82" y="320" width="12" height="12" fill="#dc2626" opacity="0.8" rx="2"/>
  <text x="100" y="331" font-size="10" fill="#374151">Tumor</text>
  <rect x="150" y="320" width="12" height="12" fill="#2563eb" opacity="0.8" rx="2"/>
  <text x="168" y="331" font-size="10" fill="#374151">Normal</text>
  <text x="240" y="331" font-size="10" fill="#6b7280">(annotation bar)</text>
</svg>
</div>

```bio
# --- Heatmap of top 50 DE genes ---
let top_50 = sig_genes |> take(CONFIG.n_top_heatmap)
let top_50_names = top_50 |> map(|g| g.gene)
let top_50_idx = top_50 |> map(|g|
  filtered_names |> index_of(g.gene))

let heatmap_data = norm_expr |> select_rows(top_50_idx)

# Z-score normalization per gene (for visualization)
let z_scored = heatmap_data |> map_rows(|row|
  let mu = mean(row)
  let s = sd(row)
  row |> map(|x| if s > 0 { (x - mu) / s } else { 0 })
)

heatmap(z_scored,
  {cluster_rows: true,
  cluster_cols: true,
  color_scale: "red_blue",
  title: "Top 50 DE Genes (Z-scored Expression)"})

print("\nHeatmap: Top " + str(CONFIG.n_top_heatmap) +
  " DE genes, Z-scored, hierarchically clustered")
```

## Section 8: Gene-Level Visualization

For specific genes of interest, show the individual data points.

```bio
# --- Box plots for top DE genes ---
print("\n=== Gene-Level Expression Plots ===")

let genes_of_interest = sig_genes |> take(6) |> map(|g| g.gene)

for gene in genes_of_interest {
  let idx = filtered_names |> index_of(gene)
  let tumor_vals = norm_expr[idx] |> select(0..6)
  let normal_vals = norm_expr[idx] |> select(6..12)
  let p = de_results |> find(|r| r.gene == gene)

  let bp_table = table({"Tumor": tumor_vals, "Normal": normal_vals})
  boxplot(bp_table, {title: gene + " (log2FC=" + str(round(p.log2fc, 2)) +
      ", FDR=" + str(round(p.adj_p, 4)) + ")"})
}
```

## Section 9: Co-expression Analysis

Do the top DE genes form co-regulated modules? A correlation heatmap of the significant genes reveals co-expression structure.

```bio
# --- Correlation among top DE genes ---
let top_100 = sig_genes |> take(100) |> map(|g| g.gene)
let top_100_idx = top_100 |> map(|g| filtered_names |> index_of(g))
let top_100_expr = norm_expr |> select_rows(top_100_idx)

let gene_cor = cor_matrix(top_100_expr)

heatmap(gene_cor,
  {cluster_rows: true,
  cluster_cols: true,
  color_scale: "red_blue",
  title: "Co-expression of Top 100 DE Genes"})

# Identify strongly correlated gene pairs
print("\n=== Strong Co-expression Pairs (|r| > 0.9) ===")
let strong_pairs = []
for i in 0..len(top_100) {
  for j in (i+1)..len(top_100) {
    if abs(gene_cor[i][j]) > 0.9 {
      strong_pairs = strong_pairs + [{
        gene_a: top_100[i],
        gene_b: top_100[j],
        r: gene_cor[i][j]
      }]
    }
  }
}
print("Found " + str(len(strong_pairs)) + " strongly correlated pairs")
for pair in strong_pairs |> take(10) {
  print("  " + pair.gene_a + " <-> " + pair.gene_b +
    " (r = " + str(round(pair.r, 3)) + ")")
}
```

## Section 10: Summary Report

```bio
# ============================================
# FINAL REPORT
# ============================================
print("\n" + "=" * 65)
print("DIFFERENTIAL EXPRESSION ANALYSIS — FINAL REPORT")
print("Colorectal Tumor (n=6) vs Normal Colon (n=6)")
print("=" * 65)

print("\n--- Data Summary ---")
print("Total genes: " + str(CONFIG.n_genes))
print("Genes after filtering: " + str(len(keep)))
print("Normalization: log2(CPM + 1)")
print("Statistical test: Welch's t-test (per gene)")
print("Multiple testing: Benjamini-Hochberg FDR")
print("Random seed: 42")

print("\n--- Quality Control ---")
print("PCA: PC1 separates tumor vs normal (" +
  str(round(qc_pca.variance_explained[0] * 100, 1)) + "% variance)")
print("No outlier samples detected")
print("Within-group correlations > " +
  str(round(min(min_within_tumor, min_within_normal), 2)))

print("\n--- Differential Expression ---")
print("Significance criteria: FDR < " + str(CONFIG.fdr_threshold) +
  " AND |log2FC| > " + str(CONFIG.fc_threshold))
print("Total DE genes: " + str(len(sig_genes)))
print("  Upregulated in tumor: " + str(len(sig_up)))
print("  Downregulated in tumor: " + str(len(sig_down)))

print("\n--- Top 5 Upregulated Genes ---")
for g in sig_up |> take(5) {
  print("  " + g.gene + ": log2FC = " + str(round(g.log2fc, 2)) +
    ", FDR = " + str(round(g.adj_p, 6)))
}

print("\n--- Top 5 Downregulated Genes ---")
for g in sig_down |> take(5) {
  print("  " + g.gene + ": log2FC = " + str(round(g.log2fc, 2)) +
    ", FDR = " + str(round(g.adj_p, 6)))
}

print("\n--- Co-expression ---")
print("Strong co-expression pairs (|r| > 0.9): " + str(len(strong_pairs)))

# Save results
write_csv(de_results |> sort_by(|r| r.adj_p),
  "results/de_results_all.csv")
write_csv(sig_genes,
  "results/de_results_significant.csv")
print("\nResults saved to results/")
print("=" * 65)
```

**Python:**

```python
import numpy as np
import pandas as pd
from scipy import stats
from statsmodels.stats.multitest import multipletests
from sklearn.decomposition import PCA
import matplotlib.pyplot as plt
import seaborn as sns

# Load count matrix
counts = pd.read_csv("counts.csv", index_col=0)

# Library sizes
lib_sizes = counts.sum(axis=0)

# Normalize
cpm = counts.div(lib_sizes) * 1e6
log_cpm = np.log2(cpm + 1)

# PCA QC
pca = PCA(n_components=2)
scores = pca.fit_transform(log_cpm.T)
plt.scatter(scores[:6, 0], scores[:6, 1], c='red', label='Tumor')
plt.scatter(scores[6:, 0], scores[6:, 1], c='blue', label='Normal')
plt.legend()
plt.show()

# DE testing
results = []
for gene in log_cpm.index:
    tumor = log_cpm.loc[gene, :6].values
    normal = log_cpm.loc[gene, 6:].values
    t, p = stats.ttest_ind(tumor, normal, equal_var=False)
    fc = tumor.mean() - normal.mean()
    results.append({'gene': gene, 'log2fc': fc, 'pvalue': p})

results = pd.DataFrame(results)
_, results['padj'], _, _ = multipletests(results['pvalue'], method='fdr_bh')
sig = results[(results['padj'] < 0.05) & (results['log2fc'].abs() > 1)]
print(f"Significant DE genes: {len(sig)}")
```

**R:**

```r
library(DESeq2)
library(pheatmap)
library(EnhancedVolcano)

# DESeq2 workflow (gold standard for RNA-seq DE)
dds <- DESeqDataSetFromMatrix(countData = counts,
                               colData = sample_info,
                               design = ~ group)
dds <- DESeq(dds)
res <- results(dds, contrast = c("group", "Tumor", "Normal"))

# Volcano plot
EnhancedVolcano(res, lab = rownames(res),
                x = 'log2FoldChange', y = 'padj',
                pCutoff = 0.05, FCcutoff = 1)

# Heatmap of top 50
sig_genes <- head(res[order(res$padj), ], 50)
vsd <- vst(dds)
pheatmap(assay(vsd)[rownames(sig_genes), ],
         scale = "row",
         annotation_col = sample_info)

# Summary
summary(res, alpha = 0.05)
```

## Exercises

1. **Filtering sensitivity.** Run the DE analysis with three different filtering thresholds: (a) no filtering, (b) at least 10 counts in 3 samples, (c) at least 50 counts in 6 samples. How does the number of DE genes change? Which filter gives the best balance of discovery and reliability?

```bio
# Your code: three filtering thresholds, compare DE counts
```

2. **Non-parametric alternative.** Replace the Welch t-test with the Wilcoxon rank-sum test for each gene. How many DE genes do you find? Is the Wilcoxon test more or less powerful with n=6 per group?

```bio
# Your code: Wilcoxon DE analysis, compare to t-test results
```

3. **Permutation-based DE.** For the top 20 DE genes (by t-test), run a permutation test (10,000 permutations) to confirm the p-value. Do the permutation p-values agree with the t-test p-values?

```bio
# Your code: permutation test for top 20 genes, compare p-values
```

4. **Effect size analysis.** For all significant DE genes, compute Cohen's d. Create a scatter plot of |log2FC| vs |Cohen's d|. Are they correlated? Which genes have high fold change but low effect size (and why)?

```bio
# Your code: scatter plot of fold change vs effect size
```

5. **Batch effect simulation.** Add a batch effect to the data (3 tumor + 3 normal from batch 1, 3 tumor + 3 normal from batch 2; add 1.5 to all gene expression in batch 2). Re-run the analysis. How does PCA change? How many spurious DE genes appear? Remove the batch effect using linear regression on PC1 and re-test.

```bio
# Your code: add batch effect, visualize, correct, re-analyze
```

## Key Takeaways

- A complete DE analysis follows a structured pipeline: QC (library sizes, PCA, correlation), filtering, normalization, testing, FDR correction, and visualization.
- PCA is the first quality control step — it reveals outliers, batch effects, and whether the primary source of variation is biological or technical.
- Gene filtering removes lowly expressed genes, reducing the multiple testing burden and removing unreliable measurements.
- Log2(CPM+1) is a simple, effective normalization for DE analysis. More sophisticated methods (TMM, DESeq2's median-of-ratios) are preferred for production analyses.
- Welch's t-test per gene with BH FDR correction is a valid DE approach. Dedicated tools (DESeq2, edgeR, limma-voom) use more sophisticated statistical models that borrow information across genes.
- The volcano plot simultaneously shows fold change and significance; the clustered heatmap shows expression patterns across samples for top genes.
- Co-expression analysis reveals gene modules — groups of genes with correlated expression that may share biological functions.
- With only 6 samples per group, statistical power is limited. Genes with moderate true effects may be missed. Power analysis (Day 18) should guide future experimental design.

## What's Next

Tomorrow is the final capstone — and the most computationally ambitious. You will analyze a genome-wide association study: 500,000 SNPs across 10,000 individuals, testing for association with type 2 diabetes. Quality control, population structure, genome-wide testing, Manhattan plots, Q-Q plots, and effect size interpretation — the full GWAS pipeline, from raw genotypes to publishable results.
