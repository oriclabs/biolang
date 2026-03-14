# Day 30: Capstone — Genome-Wide Association Study

<div class="day-meta">
<span class="badge">Day 30 of 30</span>
<span class="badge">Capstone: Days 4, 7, 11-12, 16, 18-19, 21, 25</span>
<span class="badge">~90 min reading</span>
<span class="badge">Genomics</span>
</div>

## The Problem

A large consortium has genotyped 10,000 individuals — 5,000 with type 2 diabetes (T2D) and 5,000 controls — at 500,000 single nucleotide polymorphisms (SNPs) spread across the genome. The goal is to identify genetic variants associated with T2D risk.

This is a genome-wide association study (GWAS) — the workhorse of modern human genetics. Since the first GWAS in 2005, thousands of studies have identified genetic associations for hundreds of diseases and traits. The method is conceptually simple: for each SNP, ask "is this variant more common in cases than controls?" But the execution requires careful statistical reasoning, because testing 500,000 hypotheses simultaneously creates an extreme multiple testing problem, and subtle confounders (especially population structure) can generate thousands of false positives.

By the end of today, you will have built a complete GWAS pipeline: quality control, population structure analysis, genome-wide association testing, multiple testing correction, visualization (Manhattan plot, Q-Q plot), and effect size interpretation. This capstone integrates methods from nearly every chapter of the book.

## The GWAS Pipeline

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="110" viewBox="0 0 680 110" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="20" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">GWAS Pipeline Overview</text>
  <!-- Genotyping -->
  <rect x="8" y="38" width="82" height="40" rx="6" fill="#2563eb"/>
  <text x="49" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">Genotyping</text>
  <text x="49" y="68" text-anchor="middle" font-size="8" fill="#93c5fd">500K SNPs</text>
  <path d="M 92 58 L 106 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- QC -->
  <rect x="108" y="38" width="75" height="40" rx="6" fill="#3b82f6"/>
  <text x="145" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">QC</text>
  <text x="145" y="68" text-anchor="middle" font-size="8" fill="#bfdbfe">HWE, MAF</text>
  <path d="M 185 58 L 199 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- PCA -->
  <rect x="201" y="38" width="75" height="40" rx="6" fill="#3b82f6" opacity="0.85"/>
  <text x="238" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">PCA</text>
  <text x="238" y="68" text-anchor="middle" font-size="8" fill="#bfdbfe">pop structure</text>
  <path d="M 278 58 L 292 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- Association -->
  <rect x="294" y="38" width="86" height="40" rx="6" fill="#7c3aed"/>
  <text x="337" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">Association</text>
  <text x="337" y="68" text-anchor="middle" font-size="8" fill="#c4b5fd">logistic reg.</text>
  <path d="M 382 58 L 396 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- Multiple Testing -->
  <rect x="398" y="38" width="86" height="40" rx="6" fill="#7c3aed" opacity="0.85"/>
  <text x="441" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">Correction</text>
  <text x="441" y="68" text-anchor="middle" font-size="8" fill="#c4b5fd">p &lt; 5e-8</text>
  <path d="M 486 58 L 500 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- Visualization -->
  <rect x="502" y="38" width="82" height="40" rx="6" fill="#16a34a"/>
  <text x="543" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">Visualize</text>
  <text x="543" y="68" text-anchor="middle" font-size="8" fill="#bbf7d0">Manhattan, QQ</text>
  <path d="M 586 58 L 600 58" stroke="#9ca3af" stroke-width="1.5" marker-end="url(#arrowGW30)"/>
  <!-- Interpret -->
  <rect x="602" y="38" width="68" height="40" rx="6" fill="#1e293b"/>
  <text x="636" y="55" text-anchor="middle" font-size="9" fill="white" font-weight="bold">Interpret</text>
  <text x="636" y="68" text-anchor="middle" font-size="8" fill="#9ca3af">OR, loci</text>
  <text x="340" y="100" text-anchor="middle" font-size="10" fill="#6b7280" font-style="italic">10,000 individuals x 500,000 SNPs — testing for Type 2 Diabetes associations</text>
  <defs>
    <marker id="arrowGW30" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#9ca3af"/>
    </marker>
  </defs>
</svg>
</div>

1. **SNP Quality Control**: Hardy-Weinberg equilibrium, call rates, minor allele frequency
2. **Population Structure**: PCA on genotype data, identify and correct for ancestry
3. **Association Testing**: Chi-square or logistic regression per SNP
4. **Multiple Testing Correction**: Bonferroni at genome-wide threshold
5. **Visualization**: Manhattan plot, Q-Q plot
6. **Effect Size Interpretation**: Odds ratios for top hits
7. **Power Considerations**: What could we have missed?

## Section 1: Data Simulation

```bio
set_seed(42)
# ============================================
# Genome-Wide Association Study
# Type 2 Diabetes: 5,000 Cases + 5,000 Controls
# 500,000 SNPs across 22 autosomes
# ============================================


# --- Configuration ---
let CONFIG = {
  n_cases: 5000,
  n_controls: 5000,
  n_total: 10000,
  n_snps: 500000,
  genome_wide_p: 5e-8,      # genome-wide significance
  suggestive_p: 1e-5,       # suggestive significance
  hwe_threshold: 1e-6,      # HWE filter in controls
  maf_threshold: 0.01,      # minimum minor allele frequency
  call_rate_threshold: 0.95  # minimum genotyping rate
}

# Phenotype: 0 = control, 1 = case
let phenotype = repeat(1, CONFIG.n_cases) + repeat(0, CONFIG.n_controls)

# Distribute SNPs across chromosomes (proportional to chromosome length)
let chr_lengths = [249, 243, 198, 191, 182, 171, 159, 145, 138, 134,
                   135, 133, 114, 107, 102, 90, 83, 80, 59, 64, 47, 51]
let total_length = sum(chr_lengths)
let snps_per_chr = chr_lengths |> map(|l|
  round(l / total_length * CONFIG.n_snps, 0))

# Generate SNP positions
let snp_chr = []
let snp_pos = []
let snp_names = []

for c in 0..22 {
  let n = snps_per_chr[c]
  let positions = range(0, n) |> map(|i| random_int(1, chr_lengths[c] * 1000000))
    |> sort_by(|x| x)
  for p in positions {
    snp_chr = snp_chr + [c + 1]
    snp_pos = snp_pos + [p]
    snp_names = snp_names + ["rs" + str(len(snp_names) + 1)]
  }
}

print("Simulated " + str(len(snp_names)) + " SNPs across 22 chromosomes")
print("Samples: " + str(CONFIG.n_cases) + " cases + " +
  str(CONFIG.n_controls) + " controls")

# Generate MAFs and genotypes
# Most SNPs: no association (null)
# ~50 SNPs: true associations with small effect sizes (OR 1.1-1.4)

let n_true_assoc = 50
let true_snp_indices = range(0, n_true_assoc) |> map(|i| random_int(0, len(snp_names) - 1))
let true_effect_sizes = rnorm(n_true_assoc, 0.18, 0.08)
  |> map(|x| max(0.05, min(0.40, x)))  # log(OR) range

# Generate p-values for each SNP
let p_values = []
let odds_ratios = []
let mafs = []

for i in 0..len(snp_names) {
  let maf = abs(rnorm(1, 0.2, 0.12)[0])
  maf = max(0.01, min(0.49, maf))
  mafs = mafs + [maf]

  if true_snp_indices |> contains(i) {
    # True association: generate p-value from the effect
    let idx = true_snp_indices |> index_of(i)
    let log_or = true_effect_sizes[idx]
    let or_val = exp(log_or)

    # Approximate chi-square statistic for this effect and sample size
    let freq_case = maf * or_val / (1 + maf * (or_val - 1))
    let freq_ctrl = maf
    let n = CONFIG.n_total
    let chi2 = n * pow(freq_case - freq_ctrl, 2) / (maf * (1 - maf))
    let p = max(1e-300, exp(-chi2 / 2))  # approximate

    p_values = p_values + [p]
    odds_ratios = odds_ratios + [or_val]
  } else {
    # Null SNP: p-value from uniform distribution
    let p = abs(rnorm(1, 0.5, 0.3)[0])
    p = max(0.0001, min(0.9999, p))
    p_values = p_values + [p]
    odds_ratios = odds_ratios + [1.0 + rnorm(1, 0, 0.02)[0]]
  }
}
```

## Section 2: SNP Quality Control

Before testing associations, filter out low-quality SNPs.

### Hardy-Weinberg Equilibrium

SNPs that violate Hardy-Weinberg equilibrium (HWE) in controls may indicate genotyping errors. We test each SNP with a chi-square goodness-of-fit test.

```bio
set_seed(42)
# --- Quality Control ---
print("\n=== SNP Quality Control ===")

# Simulate HWE p-values for each SNP
# Most SNPs: in HWE (high p-value)
# A few: genotyping errors (low p-value)
let n_geno_errors = 500  # SNPs with genotyping problems
let geno_error_idx = range(0, n_geno_errors) |> map(|i| random_int(0, len(snp_names) - 1))

let hwe_pvalues = []
for i in 0..len(snp_names) {
  if geno_error_idx |> contains(i) {
    # Genotyping error: HWE violation
    hwe_pvalues = hwe_pvalues + [abs(rnorm(1, 0, 1e-7)[0])]
  } else {
    # Normal SNP: in HWE
    hwe_pvalues = hwe_pvalues + [abs(rnorm(1, 0.5, 0.3)[0])]
  }
}

# Apply HWE filter (in controls only)
let pass_hwe = hwe_pvalues |> map(|p| p >= CONFIG.hwe_threshold)
let fail_hwe = count(pass_hwe, |x| !x)

print("HWE filter (p < " + str(CONFIG.hwe_threshold) + " in controls):")
print("  Failed: " + str(fail_hwe) + " SNPs removed")

# MAF filter
let pass_maf = mafs |> map(|m| m >= CONFIG.maf_threshold)
let fail_maf = count(pass_maf, |x| !x)
print("MAF filter (< " + str(CONFIG.maf_threshold) + "):")
print("  Failed: " + str(fail_maf) + " SNPs removed")

# Combined QC
let pass_qc = []
let qc_indices = []
for i in 0..len(snp_names) {
  if pass_hwe[i] && pass_maf[i] {
    pass_qc = pass_qc + [true]
    qc_indices = qc_indices + [i]
  } else {
    pass_qc = pass_qc + [false]
  }
}

let n_pass = len(qc_indices)
print("\nSNPs passing QC: " + str(n_pass) + " / " + str(len(snp_names)) +
  " (" + str(round(n_pass / len(snp_names) * 100, 1)) + "%)")

# Filter arrays to QC-passing SNPs
let qc_pvalues = p_values |> select(qc_indices)
let qc_ors = odds_ratios |> select(qc_indices)
let qc_chr = snp_chr |> select(qc_indices)
let qc_pos = snp_pos |> select(qc_indices)
let qc_names = snp_names |> select(qc_indices)
let qc_mafs = mafs |> select(qc_indices)
```

> **Key insight:** HWE filtering is performed in controls only, not cases. Disease-associated variants may legitimately deviate from HWE in cases (this is actually expected for associated variants under certain genetic models). Testing HWE in cases would remove true associations.

## Section 3: Population Structure — PCA

Population structure is the most common confounder in GWAS. If cases and controls have different ancestral backgrounds, allele frequency differences due to ancestry will be mistaken for disease associations.

```bio
set_seed(42)
# --- Population Structure Analysis ---
print("\n=== Population Structure (PCA) ===")

# Simulate PCA scores reflecting population structure
# Assume 3 major ancestry clusters with some admixture
let ancestry = range(0, CONFIG.n_total) |> map(|i| {
  let r = rnorm(1, 0, 1)[0]
  if r < 0.25 { "European" } else if r < 0.92 { "East Asian" } else { "African" }
})

# PC1 and PC2 separate ancestries
let pc1 = []
let pc2 = []
for i in 0..CONFIG.n_total {
  if ancestry[i] == "European" {
    pc1 = pc1 + [rnorm(1, 0, 5)[0]]
    pc2 = pc2 + [rnorm(1, 0, 4)[0]]
  } else if ancestry[i] == "East Asian" {
    pc1 = pc1 + [rnorm(1, 30, 5)[0]]
    pc2 = pc2 + [rnorm(1, -10, 4)[0]]
  } else {
    pc1 = pc1 + [rnorm(1, -25, 6)[0]]
    pc2 = pc2 + [rnorm(1, 20, 5)[0]]
  }
}

# PCA plot colored by ancestry
scatter(pc1, pc2,
  {xlabel: "PC1",
  ylabel: "PC2",
  title: "Population Structure — Genotype PCA"})

# PCA plot colored by case/control
scatter(pc1, pc2,
  {xlabel: "PC1",
  ylabel: "PC2",
  title: "PCA — Case/Control Overlay"})

# Check: are cases and controls balanced across ancestry?
print("\nAncestry distribution:")
print("             Cases     Controls")
for anc in ["European", "East Asian", "African"] {
  let n_case = 0
  let n_ctrl = 0
  for i in 0..CONFIG.n_total {
    if ancestry[i] == anc {
      if phenotype[i] == 1 { n_case = n_case + 1 }
      else { n_ctrl = n_ctrl + 1 }
    }
  }
  print(anc + "   " + str(n_case) + "       " + str(n_ctrl))
}
```

> **Common pitfall:** If one ancestry group has a higher prevalence of T2D (which is biologically true — T2D rates differ across populations), and allele frequencies also differ by ancestry, then every ancestry-differentiated SNP will appear associated with T2D. This is confounding, not causation. Including top PCs as covariates in the regression model removes this confounding.

## Section 4: Association Testing

For each SNP, test the association with disease status. The simplest approach is a chi-square test on the 2x3 genotype table. A more powerful approach is logistic regression with PC covariates.

### Chi-Square Test (Basic)

```bio
# --- Association Testing ---
print("\n=== Genome-Wide Association Testing ===")
print("Testing " + str(n_pass) + " SNPs...")

# The p-values were pre-computed in simulation
# In a real GWAS, you would compute them here:
#
# for each SNP:
#   let geno_table = cross_tabulate(genotypes, phenotype)
#   let chi2 = chi_square_test(geno_table)
#   or: let lr = logistic_regression(phenotype ~ snp + PC1 + PC2 + ...)
```

### Logistic Regression (Adjusted)

```bio
# Adjusted analysis includes top PCs as covariates
# This removes population structure confounding

# For this simulation, we adjust p-values to reflect
# the improvement from PC adjustment
let adjusted_pvalues = qc_pvalues  # In practice, from logistic_regression

print("Association method: logistic regression")
print("Covariates: PC1, PC2, PC3, PC4 (top 4 principal components)")
print("Model: disease ~ SNP + PC1 + PC2 + PC3 + PC4")
```

## Section 5: Multiple Testing Correction

With 500,000 tests, even a tiny false positive rate generates thousands of false hits. The genome-wide significance threshold of p < 5 x 10^-8 is the standard Bonferroni correction for approximately 1 million independent tests (accounting for linkage disequilibrium).

```bio
# --- Multiple Testing ---
print("\n=== Multiple Testing Correction ===")

# Genome-wide significant hits
let gw_sig = []
for i in 0..n_pass {
  if adjusted_pvalues[i] < CONFIG.genome_wide_p {
    gw_sig = gw_sig + [{
      snp: qc_names[i],
      chr: qc_chr[i],
      pos: qc_pos[i],
      p: adjusted_pvalues[i],
      or: qc_ors[i],
      maf: qc_mafs[i]
    }]
  }
}

# Suggestive hits
let suggestive = []
for i in 0..n_pass {
  if adjusted_pvalues[i] < CONFIG.suggestive_p && adjusted_pvalues[i] >= CONFIG.genome_wide_p {
    suggestive = suggestive + [{
      snp: qc_names[i],
      chr: qc_chr[i],
      pos: qc_pos[i],
      p: adjusted_pvalues[i],
      or: qc_ors[i]
    }]
  }
}

print("Bonferroni threshold (0.05 / " + str(n_pass) + "): " +
  str(round(0.05 / n_pass, 10)))
print("Standard genome-wide threshold: 5 x 10^-8")
print("Suggestive threshold: 1 x 10^-5")
print("")
print("Genome-wide significant hits: " + str(len(gw_sig)))
print("Suggestive hits: " + str(len(suggestive)))

# Top hits table
let all_hits = gw_sig |> sort_by(|h| h.p)

print("\n=== Top Genome-Wide Significant SNPs ===")
print("SNP              Chr    Position       p-value          OR       MAF")
print("-" * 75)
for hit in all_hits |> take(20) {
  print(hit.snp + "    " + str(hit.chr) + "    " +
    str(hit.pos) + "    " +
    str(hit.p) + "    " +
    str(round(hit.or, 3)) + "    " +
    str(round(hit.maf, 3)))
}
```

> **Key insight:** The genome-wide significance threshold of 5 x 10^-8 is extremely stringent — it corresponds to a Bonferroni correction for roughly 1 million independent tests. This means we need very large sample sizes (thousands to hundreds of thousands) to detect the small effects typical of common variant associations (OR 1.05-1.30). This is why modern GWAS consortia combine data from dozens of cohorts.

## Section 6: Manhattan Plot

```bio
# --- Manhattan Plot ---
let gwas_tbl = range(0, n_pass) |> map(|i| {
  chr: qc_chr[i], pos: qc_pos[i], p: adjusted_pvalues[i]
}) |> to_table()

manhattan(gwas_tbl,
  {significance_line: CONFIG.genome_wide_p,
  suggestive_line: CONFIG.suggestive_p,
  title: "GWAS — Type 2 Diabetes",
  xlabel: "Chromosome",
  ylabel: "-log10(p-value)"})
```

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Manhattan Plot Anatomy — GWAS Results</text>
  <!-- Axes -->
  <line x1="60" y1="260" x2="650" y2="260" stroke="#374151" stroke-width="1.5"/>
  <line x1="60" y1="40" x2="60" y2="260" stroke="#374151" stroke-width="1.5"/>
  <text x="355" y="295" text-anchor="middle" font-size="12" fill="#374151">Chromosome</text>
  <text x="20" y="150" text-anchor="middle" font-size="11" fill="#374151" transform="rotate(-90 20 150)">-log10(p-value)</text>
  <!-- Y-axis ticks -->
  <text x="50" y="260" text-anchor="end" font-size="9" fill="#6b7280">0</text>
  <text x="50" y="216" text-anchor="end" font-size="9" fill="#6b7280">2</text>
  <text x="50" y="172" text-anchor="end" font-size="9" fill="#6b7280">4</text>
  <text x="50" y="128" text-anchor="end" font-size="9" fill="#6b7280">6</text>
  <text x="50" y="84" text-anchor="end" font-size="9" fill="#6b7280">8</text>
  <text x="50" y="50" text-anchor="end" font-size="9" fill="#6b7280">10</text>
  <!-- Significance lines -->
  <line x1="60" y1="98" x2="650" y2="98" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="654" y="95" font-size="8" fill="#dc2626">5x10⁻⁸</text>
  <line x1="60" y1="150" x2="650" y2="150" stroke="#9ca3af" stroke-width="1" stroke-dasharray="4,3"/>
  <text x="654" y="147" font-size="8" fill="#9ca3af">1x10⁻⁵</text>
  <!-- Chromosome labels -->
  <text x="85" y="278" text-anchor="middle" font-size="9" fill="#6b7280">1</text>
  <text x="115" y="278" text-anchor="middle" font-size="9" fill="#6b7280">2</text>
  <text x="142" y="278" text-anchor="middle" font-size="9" fill="#6b7280">3</text>
  <text x="168" y="278" text-anchor="middle" font-size="9" fill="#6b7280">4</text>
  <text x="194" y="278" text-anchor="middle" font-size="9" fill="#6b7280">5</text>
  <text x="218" y="278" text-anchor="middle" font-size="9" fill="#6b7280">6</text>
  <text x="242" y="278" text-anchor="middle" font-size="9" fill="#6b7280">7</text>
  <text x="264" y="278" text-anchor="middle" font-size="9" fill="#6b7280">8</text>
  <text x="286" y="278" text-anchor="middle" font-size="9" fill="#6b7280">9</text>
  <text x="308" y="278" text-anchor="middle" font-size="9" fill="#6b7280">10</text>
  <text x="330" y="278" text-anchor="middle" font-size="9" fill="#6b7280">11</text>
  <text x="355" y="278" text-anchor="middle" font-size="9" fill="#6b7280">12</text>
  <text x="380" y="278" text-anchor="middle" font-size="9" fill="#6b7280">13</text>
  <text x="402" y="278" text-anchor="middle" font-size="9" fill="#6b7280">14</text>
  <text x="424" y="278" text-anchor="middle" font-size="9" fill="#6b7280">15</text>
  <text x="446" y="278" text-anchor="middle" font-size="9" fill="#6b7280">16</text>
  <text x="466" y="278" text-anchor="middle" font-size="9" fill="#6b7280">17</text>
  <text x="486" y="278" text-anchor="middle" font-size="9" fill="#6b7280">18</text>
  <text x="506" y="278" text-anchor="middle" font-size="9" fill="#6b7280">19</text>
  <text x="526" y="278" text-anchor="middle" font-size="9" fill="#6b7280">20</text>
  <text x="546" y="278" text-anchor="middle" font-size="9" fill="#6b7280">21</text>
  <text x="565" y="278" text-anchor="middle" font-size="9" fill="#6b7280">22</text>
  <!-- Chr 1 dots (blue) - noise floor -->
  <circle cx="72" cy="240" r="2" fill="#2563eb" opacity="0.5"/><circle cx="78" cy="235" r="2" fill="#2563eb" opacity="0.5"/><circle cx="84" cy="245" r="2" fill="#2563eb" opacity="0.5"/><circle cx="90" cy="238" r="2" fill="#2563eb" opacity="0.5"/><circle cx="96" cy="250" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 2 dots (teal) -->
  <circle cx="108" cy="242" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="114" cy="248" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="120" cy="236" r="2" fill="#3b82f6" opacity="0.5"/>
  <!-- Chr 3 (blue) -->
  <circle cx="134" cy="244" r="2" fill="#2563eb" opacity="0.5"/><circle cx="140" cy="240" r="2" fill="#2563eb" opacity="0.5"/><circle cx="148" cy="252" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 4 (teal) -->
  <circle cx="160" cy="238" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="168" cy="246" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="175" cy="240" r="2" fill="#3b82f6" opacity="0.5"/>
  <!-- Chr 5 (blue) -->
  <circle cx="186" cy="244" r="2" fill="#2563eb" opacity="0.5"/><circle cx="192" cy="250" r="2" fill="#2563eb" opacity="0.5"/><circle cx="200" cy="235" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 6 (teal) - HAS A PEAK -->
  <circle cx="210" cy="240" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="216" cy="248" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="220" cy="180" r="3" fill="#3b82f6" opacity="0.7"/>
  <circle cx="222" cy="130" r="3" fill="#3b82f6" opacity="0.8"/>
  <circle cx="224" cy="70" r="4" fill="#dc2626" opacity="0.9"/>
  <circle cx="226" cy="115" r="3" fill="#3b82f6" opacity="0.8"/>
  <circle cx="228" cy="165" r="3" fill="#3b82f6" opacity="0.7"/>
  <!-- Chr 7 (blue) -->
  <circle cx="236" cy="242" r="2" fill="#2563eb" opacity="0.5"/><circle cx="244" cy="248" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 8 (teal) -->
  <circle cx="258" cy="240" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="266" cy="250" r="2" fill="#3b82f6" opacity="0.5"/>
  <!-- Chr 9 (blue) - HAS A PEAK -->
  <circle cx="280" cy="244" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="284" cy="190" r="3" fill="#2563eb" opacity="0.7"/>
  <circle cx="286" cy="85" r="4" fill="#dc2626" opacity="0.9"/>
  <circle cx="288" cy="140" r="3" fill="#2563eb" opacity="0.7"/>
  <circle cx="292" cy="238" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 10-15 noise -->
  <circle cx="302" cy="245" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="310" cy="240" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="324" cy="248" r="2" fill="#2563eb" opacity="0.5"/><circle cx="332" cy="242" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="348" cy="244" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="358" cy="250" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="374" cy="240" r="2" fill="#2563eb" opacity="0.5"/><circle cx="384" cy="246" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="396" cy="248" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="408" cy="242" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="418" cy="240" r="2" fill="#2563eb" opacity="0.5"/><circle cx="428" cy="250" r="2" fill="#2563eb" opacity="0.5"/>
  <!-- Chr 16 (teal) - HAS A PEAK -->
  <circle cx="440" cy="246" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="446" cy="170" r="3" fill="#3b82f6" opacity="0.7"/>
  <circle cx="448" cy="60" r="4" fill="#dc2626" opacity="0.9"/>
  <circle cx="450" cy="105" r="3" fill="#3b82f6" opacity="0.8"/>
  <circle cx="454" cy="242" r="2" fill="#3b82f6" opacity="0.5"/>
  <!-- Chr 17-22 noise -->
  <circle cx="462" cy="248" r="2" fill="#2563eb" opacity="0.5"/><circle cx="470" cy="240" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="480" cy="244" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="490" cy="250" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="500" cy="242" r="2" fill="#2563eb" opacity="0.5"/><circle cx="510" cy="248" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="520" cy="240" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="530" cy="246" r="2" fill="#3b82f6" opacity="0.5"/>
  <circle cx="540" cy="248" r="2" fill="#2563eb" opacity="0.5"/><circle cx="550" cy="242" r="2" fill="#2563eb" opacity="0.5"/>
  <circle cx="558" cy="244" r="2" fill="#3b82f6" opacity="0.5"/><circle cx="568" cy="250" r="2" fill="#3b82f6" opacity="0.5"/>
  <!-- Peak annotations -->
  <line x1="224" y1="55" x2="224" y2="67" stroke="#dc2626" stroke-width="0.8"/>
  <text x="224" y="50" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">6p21</text>
  <line x1="286" y1="70" x2="286" y2="82" stroke="#dc2626" stroke-width="0.8"/>
  <text x="286" y="65" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">9p21</text>
  <line x1="448" y1="45" x2="448" y2="57" stroke="#dc2626" stroke-width="0.8"/>
  <text x="448" y="40" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">16q12</text>
  <!-- Legend -->
  <rect x="580" y="40" width="90" height="50" rx="4" fill="white" stroke="#e5e7eb" stroke-width="1"/>
  <circle cx="594" cy="53" r="3" fill="#dc2626"/>
  <text x="602" y="57" font-size="9" fill="#374151">GW sig.</text>
  <circle cx="594" cy="70" r="2.5" fill="#2563eb" opacity="0.5"/>
  <text x="602" y="74" font-size="9" fill="#374151">Not sig.</text>
</svg>
</div>

### Reading the Manhattan Plot

The Manhattan plot gets its name from its resemblance to the New York City skyline. Key features:

| Feature | Interpretation |
|---|---|
| Tall peaks above significance line | Genome-wide significant loci — strong associations |
| Peaks above suggestive line | Suggestive associations — may reach significance with more samples |
| Uniform noise floor | Background of null associations — should be flat |
| Elevated baseline | Possible inflation from population structure or technical artifacts |
| Peak width | Multiple associated SNPs in linkage disequilibrium — one true signal region |

## Section 7: Q-Q Plot and Genomic Inflation

The Q-Q plot compares observed p-values to expected p-values under the null. Under the null hypothesis (no associations), p-values should follow a uniform distribution. On the Q-Q plot, this appears as points along the diagonal.

```bio
# --- Q-Q Plot ---
qq_plot(adjusted_pvalues,
  {title: "Q-Q Plot — Observed vs Expected p-values",
  ci: true})

# Genomic inflation factor (lambda)
# Lambda = median(chi-square statistics) / 0.456
# Lambda close to 1.0: no inflation
# Lambda > 1.1: population structure or other systematic bias
let neg_log_p = adjusted_pvalues |> map(|p| -log10(max(p, 1e-300)))
let chi2_equiv = adjusted_pvalues |> map(|p| -2 * log(max(p, 1e-300)))
let lambda = median(chi2_equiv) / 0.456

print("\n=== Genomic Inflation ===")
print("Lambda (genomic inflation factor): " + str(round(lambda, 3)))
if lambda < 1.05 {
  print("Interpretation: No meaningful inflation. Analysis is well-calibrated.")
} else if lambda < 1.10 {
  print("Interpretation: Mild inflation. Acceptable for large GWAS.")
} else {
  print("WARNING: Lambda > 1.10 suggests confounding.")
  print("Consider additional PC covariates or genomic control correction.")
}
```

<div style="text-align: center; margin: 2em 0;">
<svg width="600" height="340" viewBox="0 0 600 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="300" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Q-Q Plot for GWAS — Observed vs Expected</text>
  <!-- Axes -->
  <line x1="80" y1="280" x2="520" y2="280" stroke="#374151" stroke-width="1.5"/>
  <line x1="80" y1="40" x2="80" y2="280" stroke="#374151" stroke-width="1.5"/>
  <text x="300" y="315" text-anchor="middle" font-size="12" fill="#374151">Expected -log10(p)</text>
  <text x="25" y="160" text-anchor="middle" font-size="12" fill="#374151" transform="rotate(-90 25 160)">Observed -log10(p)</text>
  <!-- Axis ticks -->
  <text x="80" y="298" text-anchor="middle" font-size="9" fill="#6b7280">0</text>
  <text x="190" y="298" text-anchor="middle" font-size="9" fill="#6b7280">1</text>
  <text x="300" y="298" text-anchor="middle" font-size="9" fill="#6b7280">2</text>
  <text x="410" y="298" text-anchor="middle" font-size="9" fill="#6b7280">3</text>
  <text x="520" y="298" text-anchor="middle" font-size="9" fill="#6b7280">4</text>
  <text x="68" y="280" text-anchor="end" font-size="9" fill="#6b7280">0</text>
  <text x="68" y="220" text-anchor="end" font-size="9" fill="#6b7280">2</text>
  <text x="68" y="160" text-anchor="end" font-size="9" fill="#6b7280">4</text>
  <text x="68" y="100" text-anchor="end" font-size="9" fill="#6b7280">6</text>
  <text x="68" y="48" text-anchor="end" font-size="9" fill="#6b7280">8</text>
  <!-- Reference diagonal (y = x) -->
  <line x1="80" y1="280" x2="520" y2="40" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="530" y="52" font-size="9" fill="#dc2626">y = x</text>
  <!-- CI band around diagonal -->
  <path d="M 80 280 L 300 210 L 520 130" fill="none" stroke="#93c5fd" stroke-width="1" stroke-dasharray="3,2"/>
  <path d="M 80 280 L 300 170 L 520 50" fill="none" stroke="#93c5fd" stroke-width="1" stroke-dasharray="3,2"/>
  <!-- Q-Q points - following diagonal at first, then deviating -->
  <circle cx="95" cy="275" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="105" cy="270" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="115" cy="266" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="125" cy="260" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="135" cy="255" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="150" cy="248" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="165" cy="241" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="180" cy="234" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="195" cy="226" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="210" cy="218" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="225" cy="210" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="240" cy="202" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="260" cy="192" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="280" cy="182" r="2.5" fill="#2563eb" opacity="0.6"/>
  <circle cx="300" cy="170" r="2.5" fill="#2563eb" opacity="0.6"/>
  <!-- Deviation starts here (true associations) -->
  <circle cx="320" cy="152" r="3" fill="#2563eb" opacity="0.7"/>
  <circle cx="340" cy="135" r="3" fill="#2563eb" opacity="0.7"/>
  <circle cx="360" cy="115" r="3" fill="#7c3aed" opacity="0.8"/>
  <circle cx="380" cy="92" r="3.5" fill="#7c3aed" opacity="0.8"/>
  <circle cx="400" cy="72" r="3.5" fill="#dc2626" opacity="0.9"/>
  <circle cx="420" cy="58" r="4" fill="#dc2626" opacity="0.9"/>
  <circle cx="445" cy="48" r="4" fill="#dc2626" opacity="0.9"/>
  <!-- Annotation: null region -->
  <path d="M 160 268 Q 120 290 150 300" fill="none" stroke="#16a34a" stroke-width="1"/>
  <text x="85" y="310" font-size="9" fill="#16a34a">Null SNPs follow</text>
  <text x="85" y="322" font-size="9" fill="#16a34a">the diagonal</text>
  <!-- Annotation: deviation region -->
  <path d="M 390 110 Q 430 130 460 120" fill="none" stroke="#dc2626" stroke-width="1"/>
  <text x="462" y="108" font-size="9" fill="#dc2626">True associations</text>
  <text x="462" y="120" font-size="9" fill="#dc2626">deviate upward</text>
  <!-- Lambda annotation -->
  <rect x="130" y="55" width="150" height="30" rx="4" fill="#f0f9ff" stroke="#93c5fd" stroke-width="1"/>
  <text x="205" y="73" text-anchor="middle" font-size="10" fill="#374151" font-weight="bold">Lambda = 1.02 (good)</text>
</svg>
</div>

### Interpreting the Q-Q Plot

| Pattern | Interpretation |
|---|---|
| Points follow diagonal, peel off at the tail | Expected: most SNPs are null, a few are truly associated |
| Points systematically above diagonal everywhere | Inflation: population structure, technical artifacts |
| Points below diagonal | Deflation: overly conservative test or data quality issue |
| Sharp deviation only at extreme tail | True strong associations — expected in well-powered GWAS |

> **Common pitfall:** A Q-Q plot that looks good (lambda ~ 1.0) does not guarantee the results are correct. It only means there is no systematic inflation. Individual false positives can still occur. Always validate top hits in independent cohorts.

## Section 8: Effect Sizes — Odds Ratios

```bio
# --- Effect Size Analysis ---
print("\n=== Effect Sizes for Top Hits ===")

for hit in all_hits |> take(10) {
  let log_or = log(hit.or)
  let se_log_or = abs(log_or) / sqrt(-2 * log(hit.p))  # approximate
  let ci_lower = exp(log_or - 1.96 * se_log_or)
  let ci_upper = exp(log_or + 1.96 * se_log_or)

  print(hit.snp + " (chr" + str(hit.chr) + "): OR = " +
    str(round(hit.or, 3)) + " [" +
    str(round(ci_lower, 3)) + ", " +
    str(round(ci_upper, 3)) + "]" +
    " p = " + str(hit.p))
}

# Distribution of effect sizes among significant hits
let sig_ors = gw_sig |> map(|h| h.or)
if len(sig_ors) > 0 {
  print("\nEffect size distribution (genome-wide significant):")
  print("  Median OR: " + str(round(median(sig_ors), 3)))
  print("  Range: " + str(round(min(sig_ors), 3)) + " - " +
    str(round(max(sig_ors), 3)))

  histogram(sig_ors, {bins: 20,
    title: "Distribution of Odds Ratios — Significant Hits",
    xlabel: "Odds Ratio",
    ylabel: "Count"})
}
```

### Interpreting Odds Ratios in GWAS

| OR | Risk increase per allele | Typical for |
|---|---|---|
| 1.05 - 1.10 | 5-10% | Most common variant associations |
| 1.10 - 1.20 | 10-20% | Moderate-effect common variants |
| 1.20 - 1.50 | 20-50% | Larger-effect variants (less common) |
| 1.50 - 3.00 | 50-200% | Strong effects (rare in GWAS, common in candidate gene studies) |
| > 3.00 | >200% | Very rare in GWAS; usually rare variants with large effects |

> **Clinical relevance:** Individual GWAS hits typically have small effects (OR 1.05-1.20). No single SNP is useful for predicting disease. However, combining hundreds of hits into a polygenic risk score (PRS) can identify individuals at meaningfully elevated risk. For T2D, top-decile PRS individuals have ~3-5x higher risk than bottom-decile individuals.

## Section 9: Power Analysis — What Did We Miss?

```bio
# --- Power Analysis ---
print("\n=== Statistical Power ===")

# At our sample size (5000 cases + 5000 controls), what ORs can we detect?
let mafs_to_check = [0.01, 0.05, 0.10, 0.20, 0.30, 0.50]
let ors_to_check = [1.05, 1.10, 1.15, 1.20, 1.30, 1.50]

print("\nPower to detect association at p < 5e-8:")
print("(N = 5,000 cases + 5,000 controls)")
print("")
print("MAF    OR=1.05  OR=1.10  OR=1.15  OR=1.20  OR=1.30  OR=1.50")
print("-" * 65)

for maf in mafs_to_check {
  let powers = []
  for or_val in ors_to_check {
    # Approximate power calculation for GWAS
    let log_or = log(or_val)
    let n = CONFIG.n_total
    let var_logistic = 1 / (n * maf * (1 - maf) * 0.25)
    let z_alpha = 5.33  # z for p = 5e-8 (two-sided)
    let ncp = log_or / sqrt(var_logistic)
    let power = 1.0 - pnorm(z_alpha - ncp) + pnorm(-z_alpha - ncp)
    power = max(0, min(1, power))
    powers = powers + [power]
  }
  print(str(maf) + "   " +
    powers |> map(|p| str(round(p * 100, 0)) + "%") |> join("    "))
}

print("\nInterpretation:")
print("- At MAF=0.20 and OR=1.20, we have good power (~80%+)")
print("- At MAF=0.05 and OR=1.10, power is very low")
print("- Many true associations with small effects are missed")
print("- Larger sample sizes or meta-analysis would recover more hits")
```

## Section 10: Locus Summary and Reporting

```bio
# --- Locus-Level Summary ---
print("\n=== Locus Summary ===")

# Group nearby significant SNPs into loci (within 500kb)
let loci = []
let used = []

for hit in all_hits {
  if used |> contains(hit.snp) { continue }

  let locus_snps = [hit]
  used = used + [hit.snp]

  for other in all_hits {
    if used |> contains(other.snp) { continue }
    if other.chr == hit.chr && abs(other.pos - hit.pos) < 500000 {
      locus_snps = locus_snps + [other]
      used = used + [other.snp]
    }
  }

  let lead = locus_snps |> sort_by(|s| s.p) |> first()
  loci = loci + [{
    lead_snp: lead.snp,
    chr: lead.chr,
    pos: lead.pos,
    p: lead.p,
    or: lead.or,
    n_snps: len(locus_snps)
  }]
}

print("\nIndependent loci: " + str(len(loci)))
print("\nLocus  Lead SNP         Chr  Position       p-value       OR     #SNPs")
print("-" * 75)
for i in 0..len(loci) {
  let l = loci[i]
  print(str(i + 1) + "     " + l.lead_snp + "   " + str(l.chr) + "    " +
    str(l.pos) + "    " + str(l.p) + "    " +
    str(round(l.or, 3)) + "    " + str(l.n_snps))
}
```

## Section 11: Complete Report

```bio
# ============================================
# FINAL GWAS REPORT
# ============================================
print("\n" + "=" * 70)
print("GENOME-WIDE ASSOCIATION STUDY — FINAL REPORT")
print("Phenotype: Type 2 Diabetes")
print("=" * 70)

print("\n--- Study Design ---")
print("Cases: " + str(CONFIG.n_cases))
print("Controls: " + str(CONFIG.n_controls))
print("SNPs genotyped: " + str(CONFIG.n_snps))
print("SNPs after QC: " + str(n_pass))
print("Association model: logistic regression with PC1-4 covariates")
print("Random seed: 42")

print("\n--- Quality Control ---")
print("HWE filter (p < " + str(CONFIG.hwe_threshold) + " in controls): " +
  str(fail_hwe) + " removed")
print("MAF filter (< " + str(CONFIG.maf_threshold) + "): " +
  str(fail_maf) + " removed")
print("QC pass rate: " + str(round(n_pass / CONFIG.n_snps * 100, 1)) + "%")

print("\n--- Population Structure ---")
print("Ancestry groups: European (" +
  str(count(ancestry, |a| a == "European")) + "), East Asian (" +
  str(count(ancestry, |a| a == "East Asian")) + "), African (" +
  str(count(ancestry, |a| a == "African")) + ")")
print("PCA: PC1-PC2 separate ancestry clusters clearly")
print("Top 4 PCs included as covariates")

print("\n--- Genomic Inflation ---")
print("Lambda: " + str(round(lambda, 3)))
if lambda < 1.05 {
  print("Status: Well-calibrated (no inflation)")
} else {
  print("Status: Mild inflation detected — review QC")
}

print("\n--- Association Results ---")
print("Genome-wide significant (p < 5e-8): " + str(len(gw_sig)) + " SNPs")
print("Independent loci: " + str(len(loci)))
print("Suggestive (p < 1e-5): " + str(len(suggestive)) + " SNPs")

print("\n--- Effect Sizes ---")
if len(sig_ors) > 0 {
  print("Median OR among hits: " + str(round(median(sig_ors), 3)))
  print("OR range: " + str(round(min(sig_ors), 3)) + " - " +
    str(round(max(sig_ors), 3)))
}

print("\n--- Key Findings ---")
print("1. " + str(len(loci)) + " independent loci associated with T2D at genome-wide significance")
print("2. Effect sizes are modest (OR 1.1-1.4), consistent with polygenic architecture")
print("3. No evidence of systematic inflation (lambda = " + str(round(lambda, 3)) + ")")
print("4. Power analysis indicates additional loci would be detectable with larger samples")

print("\n--- Figures Generated ---")
print("1. PCA plot (population structure)")
print("2. Manhattan plot (genome-wide results)")
print("3. Q-Q plot (inflation assessment)")
print("4. OR distribution (effect sizes)")

print("\n" + "=" * 70)
```

**Python:**

```python
import numpy as np
import pandas as pd
from scipy import stats
import matplotlib.pyplot as plt

# Load PLINK output or compute associations
# assoc = pd.read_csv("gwas_results.assoc", sep="\t")

# Chi-square per SNP
for snp in genotype_matrix.columns:
    table = pd.crosstab(phenotype, genotype_matrix[snp])
    chi2, p, dof, expected = stats.chi2_contingency(table)

# Logistic regression with covariates
from sklearn.linear_model import LogisticRegression
for snp in snps:
    X = np.column_stack([genotype[:, snp], pc1, pc2, pc3, pc4])
    model = LogisticRegression().fit(X, phenotype)

# Manhattan plot
from qmplot import manhattanplot
manhattanplot(data=results, chrom="CHR", pos="BP", pv="P",
              suggestiveline=-np.log10(1e-5),
              genomewideline=-np.log10(5e-8))

# Q-Q plot
from qmplot import qqplot
qqplot(data=results["P"])

# Genomic inflation
chi2_stats = stats.chi2.isf(results['P'], df=1)
lambda_gc = np.median(chi2_stats) / 0.456
```

**R:**

```r
# PLINK association (most common tool for GWAS)
# system("plink --bfile mydata --assoc --adjust --out results")

# Or in R with snpStats
library(snpStats)
result <- single.snp.tests(phenotype ~ 1, snp.data = genotypes)

# Manhattan plot
library(qqman)
manhattan(results, chr="CHR", bp="BP", p="P", snp="SNP",
          suggestiveline=-log10(1e-5),
          genomewideline=-log10(5e-8))

# Q-Q plot
qq(results$P)

# Logistic regression with covariates
for (snp in colnames(geno)) {
  model <- glm(pheno ~ geno[, snp] + PC1 + PC2 + PC3 + PC4,
               family = binomial)
  p <- summary(model)$coefficients[2, 4]
}

# Genomic inflation
chisq <- qchisq(1 - results$P, df = 1)
lambda <- median(chisq) / qchisq(0.5, df = 1)
```

## Exercises

1. **Stricter QC.** Re-run the analysis with a stricter HWE filter (p < 1e-4 instead of 1e-6) and higher MAF threshold (0.05 instead of 0.01). How many SNPs are removed? Does the number of significant hits change?

```bio
# Your code: stricter QC filters, compare hit counts
```

2. **Bonferroni vs FDR.** Apply BH FDR correction instead of the genome-wide significance threshold. How many more hits does FDR q < 0.05 find compared to p < 5e-8? What is the tradeoff?

```bio
# Your code: BH correction, compare to Bonferroni
let fdr_adjusted = p_adjust(adjusted_pvalues, "BH")
```

3. **Ancestry-stratified analysis.** Run the association analysis separately in each ancestry group. Do the same loci reach significance? What happens to power when you split the sample?

```bio
# Your code: subset by ancestry, run GWAS in each, compare
```

4. **Genomic control.** If lambda were 1.15 (inflated), apply genomic control correction: divide all chi-square statistics by lambda and recompute p-values. How does this change the Manhattan plot?

```bio
# Your code: inflate p-values, apply GC correction, re-plot
```

5. **Power for future study.** Your consortium plans a Phase 2 GWAS with 50,000 cases and 50,000 controls. At MAF = 0.10, what is the minimum detectable OR at 80% power? How many more loci would you expect to find?

```bio
# Your code: power calculation for larger sample
```

## Key Takeaways

- A GWAS tests hundreds of thousands of SNPs for association with a phenotype, requiring rigorous QC (HWE, MAF, call rate), population structure control (PCA), and extreme multiple testing correction (p < 5 x 10^-8).
- Hardy-Weinberg equilibrium testing in controls identifies genotyping errors; MAF filtering removes uninformative rare variants; both are standard QC steps.
- Population structure is the primary confounder in GWAS. PCA on genotype data reveals ancestry, and including top PCs as covariates in logistic regression removes confounding.
- The genome-wide significance threshold (5 x 10^-8) is a Bonferroni correction for approximately 1 million independent tests, accounting for linkage disequilibrium.
- The Manhattan plot displays -log10(p-values) by genomic position; the Q-Q plot assesses whether the test statistics are well-calibrated (lambda near 1.0).
- Most GWAS hits have modest effect sizes (OR 1.05-1.30), reflecting the polygenic architecture of complex traits. Individual variants are not clinically useful predictors, but combined into polygenic risk scores, they have growing clinical applications.
- Power in GWAS depends on sample size, allele frequency, and effect size. Large international consortia (>100,000 individuals) are needed to detect the small effects typical of common diseases.
- This capstone integrates concepts from probability (Day 4), hypothesis testing (Day 7), chi-square tests (Day 11), multiple testing (Day 12), logistic regression (Day 16), power analysis (Day 18), effect sizes (Day 19), PCA (Day 21), and visualization (Day 25).

## Congratulations

You have completed "Practical Biostatistics in 30 Days." Over the past four weeks, you have journeyed from basic descriptive statistics to a genome-wide association study analyzing half a million genetic variants. Along the way, you have learned:

- **Foundations**: Distributions, probability, sampling, and why n matters
- **Core methods**: Confidence intervals, hypothesis testing, t-tests, non-parametric alternatives, ANOVA, chi-square tests
- **Multiple testing**: The FDR crisis and how to control it
- **Modeling**: Correlation, linear regression, multiple regression, logistic regression, survival analysis
- **Design**: Experimental design, statistical power, effect sizes, batch effects
- **Advanced methods**: PCA, clustering, resampling, Bayesian inference, meta-analysis
- **Practice**: Reproducible analysis, clinical trials, differential expression, GWAS

Every method in this book is a tool. Like any tool, it can be used well or poorly. The difference is understanding — knowing not just how to run a test, but when to run it, what its assumptions are, and how to interpret its results in the context of your biological question.

Statistics is not the final step of an experiment. It is the lens through which every experiment should be designed, conducted, and interpreted. The best statisticians are not those who know the most tests — they are those who ask the right questions.

Go forth and analyze. Be rigorous. Be honest. Be curious. And always, always check your assumptions.
