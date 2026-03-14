# Day 20: Batch Effects and Confounders

## The Problem

Dr. David Liu is leading a multi-center study comparing gene expression in 200 breast tumors versus 100 normal breast tissues. Three hospitals contributed samples: Memorial (80 tumors, 40 normals), Hopkins (70 tumors, 30 normals), and Mayo (50 tumors, 30 normals). He runs PCA on the full expression matrix, expecting to see tumor and normal separate.

Instead, the PCA plot shows **three tight clusters — one per hospital**. The first principal component explains 35% of variance and perfectly separates the centers. Tumor vs. normal? It's buried in PC4 at 4% of variance.

The dominant signal in his data is **which hospital processed the sample**, not the biology he's studying. These are **batch effects**, and they're one of the most insidious problems in genomics.

## What Are Batch Effects?

Batch effects are **systematic technical differences** between groups of samples that were processed differently. They have nothing to do with biology but can completely dominate a dataset.

| Source | Mechanism | Example |
|--------|-----------|---------|
| Processing date | Reagent lots, temperature, humidity | Monday vs. Friday RNA extractions |
| Technician | Handling differences | Technician A vs. B |
| Center/site | Different protocols, equipment | Hospital A vs. B |
| Sequencing lane | Flow cell chemistry, loading density | Lane 1 vs. Lane 2 |
| Plate position | Edge effects, evaporation | Well A1 vs. H12 |
| Reagent lot | Batch-to-batch kit variation | Kit lot 2024A vs. 2024B |
| Storage time | RNA degradation over time | Samples banked in 2020 vs. 2024 |

> **Key insight:** Batch effects are not random noise — they are systematic biases that affect thousands of genes simultaneously. They shift entire samples in the same direction, which is why PCA detects them so readily.

### How Large Are Batch Effects?

In a landmark 2010 study, Leek et al. analyzed publicly available microarray data and found:

- Batch effects were present in **virtually all** high-throughput datasets
- They often explained **more variance** than the biological signal of interest
- They affected **thousands of genes** per batch, not just a handful

In RNA-seq, batch effects are typically smaller than in microarrays but still substantial — often explaining 10-30% of total variance.

## Identifying Batch Effects

### 1. PCA Visualization

The most powerful diagnostic. Color samples by batch variable and biological variable. If batch dominates PC1/PC2, you have a problem.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">PCA Reveals Batch Effects: Samples Cluster by Facility, Not Biology</text>
  <g transform="translate(50, 40)">
    <!-- Axes -->
    <line x1="60" y1="290" x2="560" y2="290" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="290" x2="60" y2="30" stroke="#6b7280" stroke-width="1.5"/>
    <text x="310" y="320" text-anchor="middle" font-size="12" fill="#6b7280">PC1 (35% variance) — Dominated by batch!</text>
    <text x="15" y="160" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 15, 160)">PC2 (18% variance)</text>
    <!-- Memorial cluster (blue) — left -->
    <circle cx="130" cy="130" r="30" fill="#2563eb" opacity="0.07" stroke="#2563eb" stroke-width="1" stroke-dasharray="3,2"/>
    <circle cx="120" cy="120" r="5" fill="#2563eb" opacity="0.7"/><circle cx="140" cy="115" r="5" fill="#2563eb" opacity="0.7"/>
    <circle cx="135" cy="135" r="5" fill="#2563eb" opacity="0.7"/><circle cx="115" cy="140" r="5" fill="#2563eb" opacity="0.7"/>
    <circle cx="145" cy="128" r="5" fill="#2563eb" opacity="0.7"/><circle cx="125" cy="110" r="5" fill="#2563eb" opacity="0.7"/>
    <!-- Shapes for condition within Memorial: circles=tumor, triangles=normal -->
    <polygon points="118,150 122,142 114,142" fill="#2563eb" opacity="0.7"/>
    <polygon points="138,145 142,137 134,137" fill="#2563eb" opacity="0.7"/>
    <polygon points="128,155 132,147 124,147" fill="#2563eb" opacity="0.7"/>
    <!-- Hopkins cluster (red) — middle -->
    <circle cx="310" cy="180" r="30" fill="#dc2626" opacity="0.07" stroke="#dc2626" stroke-width="1" stroke-dasharray="3,2"/>
    <circle cx="300" cy="170" r="5" fill="#dc2626" opacity="0.7"/><circle cx="320" cy="175" r="5" fill="#dc2626" opacity="0.7"/>
    <circle cx="315" cy="190" r="5" fill="#dc2626" opacity="0.7"/><circle cx="295" cy="185" r="5" fill="#dc2626" opacity="0.7"/>
    <circle cx="325" cy="165" r="5" fill="#dc2626" opacity="0.7"/><circle cx="305" cy="195" r="5" fill="#dc2626" opacity="0.7"/>
    <polygon points="298,200 302,192 294,192" fill="#dc2626" opacity="0.7"/>
    <polygon points="318,200 322,192 314,192" fill="#dc2626" opacity="0.7"/>
    <!-- Mayo cluster (green) — right -->
    <circle cx="470" cy="100" r="30" fill="#16a34a" opacity="0.07" stroke="#16a34a" stroke-width="1" stroke-dasharray="3,2"/>
    <circle cx="460" cy="90" r="5" fill="#16a34a" opacity="0.7"/><circle cx="480" cy="95" r="5" fill="#16a34a" opacity="0.7"/>
    <circle cx="475" cy="110" r="5" fill="#16a34a" opacity="0.7"/><circle cx="455" cy="105" r="5" fill="#16a34a" opacity="0.7"/>
    <circle cx="485" cy="85" r="5" fill="#16a34a" opacity="0.7"/><circle cx="465" cy="115" r="5" fill="#16a34a" opacity="0.7"/>
    <polygon points="478,118 482,110 474,110" fill="#16a34a" opacity="0.7"/>
    <polygon points="458,120 462,112 454,112" fill="#16a34a" opacity="0.7"/>
    <!-- Cluster labels -->
    <text x="130" y="88" text-anchor="middle" font-size="11" fill="#2563eb" font-weight="bold">Memorial</text>
    <text x="310" y="145" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Hopkins</text>
    <text x="470" y="63" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Mayo</text>
    <!-- Legend -->
    <rect x="350" y="240" width="210" height="48" rx="4" fill="white" stroke="#e5e7eb" stroke-width="1"/>
    <text x="360" y="256" font-size="10" fill="#6b7280" font-weight="bold">Shape = Biology:</text>
    <circle cx="370" cy="270" r="4" fill="#6b7280"/>
    <text x="380" y="274" font-size="10" fill="#1e293b">= Tumor</text>
    <polygon points="440,274 444,266 436,266" fill="#6b7280"/>
    <text x="452" y="274" font-size="10" fill="#1e293b">= Normal</text>
    <!-- Warning annotation -->
    <rect x="140" y="240" width="180" height="38" rx="4" fill="#fef2f2" stroke="#dc2626" stroke-width="0.8"/>
    <text x="230" y="256" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">Tumor and Normal are</text>
    <text x="230" y="270" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">mixed within each cluster!</text>
  </g>
</svg>
</div>

### 2. Correlation Heatmap

Compute sample-to-sample correlation. If samples cluster by batch rather than biology, batch effects are present.

### 3. Box Plots by Batch

Plot the distribution of expression values (or a summary statistic) stratified by batch. Systematic shifts indicate batch effects.

### 4. ANOVA for Batch

For each gene, test whether expression differs significantly by batch. If hundreds or thousands of genes show batch effects, the problem is pervasive.

## Confounders: A Deeper Problem

A **confounder** is a variable associated with BOTH the predictor and the outcome, creating a spurious association (or masking a real one).

| Scenario | Confounder | Danger |
|----------|-----------|--------|
| Gene expression differs by sex | Tumor subtype differs by sex | Sex drives both |
| Drug response varies by ethnicity | Genetic variant frequency varies by ethnicity | Population stratification |
| Survival differs by TP53 status | Stage differs by TP53 status | Stage confounds TP53 effect |
| Expression differs by treatment | Samples processed on different days | Processing day = treatment |

### Simpson's Paradox

The most dramatic form of confounding. A trend that appears in aggregate **reverses** when groups are separated:

| Hospital | Drug A Survival | Drug B Survival | Better Drug |
|----------|----------------|-----------------|-------------|
| Hospital 1 (mild cases) | 95% | 90% | Drug A |
| Hospital 2 (severe cases) | 40% | 30% | Drug A |
| **Combined** | **55%** | **70%** | **Drug B??** |

Drug A is better at BOTH hospitals, but Drug B appears better overall because Hospital 2 (severe cases, low survival) preferentially used Drug A.

> **Clinical relevance:** Simpson's paradox has real consequences. In the 1970s, Berkeley was accused of sex discrimination in graduate admissions. Overall, women had lower acceptance rates. But department by department, women were accepted at equal or higher rates — they had applied more often to competitive departments with low overall acceptance rates.

## The Confounded Design Trap

The most dangerous scenario: **when batch perfectly correlates with biology**.

| Sample | Center | Condition |
|--------|--------|-----------|
| 1-50 | Center A | All Tumor |
| 51-100 | Center B | All Normal |

Here, center and condition are **completely confounded**. Every difference between tumor and normal could equally be a difference between Center A and Center B. No statistical method can separate them. The experiment is fatally flawed.

> **Common pitfall:** This happens more often than you'd think. A collaborator sends tumor samples from one hospital and normal samples from another. Or samples are processed tumor on Monday, normal on Tuesday. The solution is **balanced design** — ensure batch variables are distributed across biological groups.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Confounded vs. Balanced Study Design</text>
  <!-- Left: Confounded (BAD) -->
  <g transform="translate(20, 50)">
    <text x="150" y="16" text-anchor="middle" font-size="13" font-weight="bold" fill="#dc2626">Confounded Design</text>
    <rect x="20" y="30" width="260" height="220" rx="8" fill="white" stroke="#dc2626" stroke-width="1.5"/>
    <!-- Batch A column -->
    <rect x="40" y="55" width="100" height="170" rx="4" fill="#2563eb" opacity="0.08" stroke="#2563eb" stroke-width="1"/>
    <text x="90" y="50" text-anchor="middle" font-size="11" fill="#2563eb" font-weight="bold">Batch A</text>
    <!-- All tumors -->
    <rect x="55" y="70" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="83" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="92" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="105" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="114" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="127" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="136" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="149" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="158" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="171" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <text x="90" y="200" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">100% Tumor</text>
    <!-- Batch B column -->
    <rect x="160" y="55" width="100" height="170" rx="4" fill="#16a34a" opacity="0.08" stroke="#16a34a" stroke-width="1"/>
    <text x="210" y="50" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Batch B</text>
    <!-- All normals -->
    <rect x="175" y="70" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="83" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="92" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="105" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="114" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="127" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="136" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="149" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="158" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="171" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <text x="210" y="200" text-anchor="middle" font-size="10" fill="#2563eb" font-weight="bold">100% Normal</text>
    <!-- Big X -->
    <line x1="30" y1="35" x2="270" y2="245" stroke="#dc2626" stroke-width="4" opacity="0.3"/>
    <line x1="270" y1="35" x2="30" y2="245" stroke="#dc2626" stroke-width="4" opacity="0.3"/>
    <!-- Verdict -->
    <text x="150" y="268" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">FATAL: Cannot separate</text>
    <text x="150" y="282" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">batch from biology!</text>
  </g>
  <!-- Right: Balanced (GOOD) -->
  <g transform="translate(360, 50)">
    <text x="150" y="16" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">Balanced Design</text>
    <rect x="20" y="30" width="260" height="220" rx="8" fill="white" stroke="#16a34a" stroke-width="1.5"/>
    <!-- Batch A column -->
    <rect x="40" y="55" width="100" height="170" rx="4" fill="#2563eb" opacity="0.08" stroke="#2563eb" stroke-width="1"/>
    <text x="90" y="50" text-anchor="middle" font-size="11" fill="#2563eb" font-weight="bold">Batch A</text>
    <!-- Mixed -->
    <rect x="55" y="70" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="83" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="92" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="90" y="105" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="55" y="114" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="127" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="55" y="136" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="90" y="149" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="55" y="158" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="90" y="171" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <text x="90" y="200" text-anchor="middle" font-size="10" fill="#6b7280" font-weight="bold">50/50 mix</text>
    <!-- Batch B column -->
    <rect x="160" y="55" width="100" height="170" rx="4" fill="#16a34a" opacity="0.08" stroke="#16a34a" stroke-width="1"/>
    <text x="210" y="50" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Batch B</text>
    <rect x="175" y="70" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="83" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="92" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="210" y="105" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="175" y="114" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="127" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <rect x="175" y="136" width="70" height="18" rx="3" fill="#dc2626" opacity="0.2"/>
    <text x="210" y="149" text-anchor="middle" font-size="9" fill="#dc2626">Tumor</text>
    <rect x="175" y="158" width="70" height="18" rx="3" fill="#3b82f6" opacity="0.2"/>
    <text x="210" y="171" text-anchor="middle" font-size="9" fill="#2563eb">Normal</text>
    <text x="210" y="200" text-anchor="middle" font-size="10" fill="#6b7280" font-weight="bold">50/50 mix</text>
    <!-- Checkmark -->
    <path d="M 130,240 L 145,258 L 175,225" stroke="#16a34a" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <text x="150" y="282" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Batch is correctable!</text>
  </g>
</svg>
</div>

## Strategies for Handling Batch Effects

### 1. Prevention (Best Option)

| Strategy | How |
|----------|-----|
| Balanced design | Distribute conditions across batches equally |
| Randomization | Random assignment to processing order/position |
| Blocking | Process one sample from each condition per batch |
| Standard protocols | Minimize technical variation with SOPs |
| Single batch | Process everything together (often impractical) |

### 2. Detection

| Method | What It Reveals |
|--------|----------------|
| PCA colored by batch | Visual clustering by technical variable |
| Correlation heatmap | Sample similarity by batch |
| Box plots by batch | Distribution shifts |
| ANOVA per gene | Number of genes affected |

### 3. Correction

| Method | Approach | Caveat |
|--------|----------|--------|
| Include as covariate | Add batch to regression/DE model | Requires balanced design |
| ComBat (parametric) | Empirical Bayes batch correction | Assumes batch is known |
| ComBat-seq | ComBat for raw RNA-seq counts | Preserves count nature |
| SVA (surrogate variables) | Discovers unknown batch effects | May remove biology |
| RUVseq | Uses control genes | Needs negative controls |

> **Common pitfall:** Do NOT use batch correction methods when batch is perfectly confounded with biology. They will remove both the batch effect AND the biological signal. If all tumors were processed in batch 1 and all normals in batch 2, no correction method can save you — the experiment must be redesigned.

## Batch Effects in BioLang

### Simulating Batch-Contaminated Data

```bio
set_seed(42)
# Simulate a multi-center gene expression study
let n_samples = 300
let n_genes = 50

# Assign samples to centers and conditions
let center = []
let condition = []
for i in 0..n_samples {
    if i < 100 {
        center = center + ["Memorial"]
        condition = condition + [if i < 60 { "Tumor" } else { "Normal" }]
    } else if i < 200 {
        center = center + ["Hopkins"]
        condition = condition + [if i < 170 { "Tumor" } else { "Normal" }]
    } else {
        center = center + ["Mayo"]
        condition = condition + [if i < 250 { "Tumor" } else { "Normal" }]
    }
}

# Generate expression matrix with batch effects
let expr_matrix = []
let batch_effects = {
    "Memorial": rnorm(n_genes, 2.0, 0.5),
    "Hopkins": rnorm(n_genes, -1.5, 0.5),
    "Mayo": rnorm(n_genes, 0.5, 0.5)
}

# True biological signal (tumor vs normal)
let bio_signal = rnorm(n_genes, 0, 0.3)
# Only 10 genes are truly DE
for g in 0..10 {
    bio_signal[g] = rnorm(1, 1.5, 0.3)[0]
}

for i in 0..n_samples {
    let sample_expr = []
    for g in 0..n_genes {
        let base = 10 + rnorm(1, 0, 1)[0]
        let batch = batch_effects[center[i]][g]
        let bio = if condition[i] == "Tumor" { bio_signal[g] } else { 0 }
        sample_expr = sample_expr + [base + batch + bio]
    }
    expr_matrix = expr_matrix + [sample_expr]
}

print("Expression matrix: {n_samples} samples x {n_genes} genes")
print("Centers: Memorial=100, Hopkins=100, Mayo=100")
```

### Detecting Batch Effects with PCA

```bio
# PCA on the expression matrix
let pca_result = pca(expr_matrix, 5)

print("=== PCA Variance Explained ===")
for i in 0..5 {
    print("PC{i+1}: {(pca_result.variance_explained[i] * 100) |> round(1)}%")
}

# Plot PC1 vs PC2, colored by center
let pca_data = table({
    "PC1": pca_result.scores |> map(|s| s[0]),
    "PC2": pca_result.scores |> map(|s| s[1]),
    "center": center,
    "condition": condition
})

pca_plot(pca_data)

# If center dominates PC1 and condition is only visible on PC3+,
# batch effects are overwhelming the biological signal
```

### Quantifying Batch Effect with ANOVA

```bio
# For each gene, test how much variance is explained by batch vs biology
let batch_significant = 0
let bio_significant = 0

print("=== ANOVA: Batch vs Biology ===")

for g in 0..10 {
    let gene_expr = expr_matrix |> map(|row| row[g])

    # Group by center
    let memorial_expr = []
    let hopkins_expr = []
    let mayo_expr = []
    for i in 0..n_samples {
        if center[i] == "Memorial" { memorial_expr = memorial_expr + [gene_expr[i]] }
        else if center[i] == "Hopkins" { hopkins_expr = hopkins_expr + [gene_expr[i]] }
        else { mayo_expr = mayo_expr + [gene_expr[i]] }
    }
    let batch_test = anova([memorial_expr, hopkins_expr, mayo_expr])

    # Group by condition
    let tumor_expr = []
    let normal_expr = []
    for i in 0..n_samples {
        if condition[i] == "Tumor" { tumor_expr = tumor_expr + [gene_expr[i]] }
        else { normal_expr = normal_expr + [gene_expr[i]] }
    }
    let bio_test = ttest(tumor_expr, normal_expr)

    if batch_test.p_value < 0.05 { batch_significant = batch_significant + 1 }
    if bio_test.p_value < 0.05 { bio_significant = bio_significant + 1 }

    print("Gene {g+1}: batch F={batch_test.f_statistic |> round(2)} p={batch_test.p_value |> round(4)}  bio p={bio_test.p_value |> round(4)}")
}

print("\nGenes with significant batch effect: {batch_significant} / 10")
print("Genes with significant biological effect: {bio_significant} / 10")
```

### Correlation Heatmap for Batch Detection

```bio
# Sample-to-sample correlation: compute a few representative pairs
# Full matrix would be 300x300; spot-check a few
let s1 = expr_matrix[0]   # Memorial, Tumor
let s2 = expr_matrix[50]  # Memorial, Normal
let s3 = expr_matrix[100] # Hopkins, Tumor

print("=== Sample Correlations ===")
print("Memorial Tumor vs Memorial Normal: {cor(s1, s2) |> round(3)}")
print("Memorial Tumor vs Hopkins Tumor:   {cor(s1, s3) |> round(3)}")
print("If same-center pairs are more correlated than same-condition pairs,")
print("batch effects dominate")
```

### Box Plots by Batch

```bio
# Distribution of a batch-affected gene across centers
let gene_1_expr = expr_matrix |> map(|row| row[0])

let memorial_g1 = []
let hopkins_g1 = []
let mayo_g1 = []
for i in 0..n_samples {
    if center[i] == "Memorial" { memorial_g1 = memorial_g1 + [gene_1_expr[i]] }
    else if center[i] == "Hopkins" { hopkins_g1 = hopkins_g1 + [gene_1_expr[i]] }
    else { mayo_g1 = mayo_g1 + [gene_1_expr[i]] }
}

let bp_table = table({"Memorial": memorial_g1, "Hopkins": hopkins_g1, "Mayo": mayo_g1})
boxplot(bp_table, {title: "Gene 1 Expression by Center"})

# Systematic shifts between centers = batch effect
```

### Correcting Batch Effects: Include as Covariate

```bio
# The simplest and most transparent correction:
# include batch as a covariate in your statistical model

# For differential expression: multiple regression
for g in 0..5 {
    let gene_expr = expr_matrix |> map(|row| row[g])

    # Encode condition: Tumor=1, Normal=0
    let cond_numeric = condition |> map(|c| if c == "Tumor" { 1.0 } else { 0.0 })

    # Encode center: dummy variables
    let is_hopkins = center |> map(|c| if c == "Hopkins" { 1.0 } else { 0.0 })
    let is_mayo = center |> map(|c| if c == "Mayo" { 1.0 } else { 0.0 })

    # Model WITHOUT batch correction
    let model_naive = lm(gene_expr, cond_numeric)

    # Model WITH batch correction (include center as covariate)
    let adj_data = table({
        "expr": gene_expr, "cond": cond_numeric,
        "hopkins": is_hopkins, "mayo": is_mayo
    })
    let model_adjusted = lm("expr ~ cond + hopkins + mayo", adj_data)

    print("Gene {g+1}:")
    print("  Naive:    slope = {model_naive.slope |> round(3)}, p = {model_naive.p_value |> round(4)}")
    print("  Adjusted: cond coef = {model_adjusted.coefficients[0] |> round(3)}")
}
```

### Before/After Comparison

<div style="text-align: center; margin: 2em 0;">
<svg width="700" height="340" viewBox="0 0 700 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="350" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Before and After Batch Correction (PCA)</text>
  <!-- Arrow between panels -->
  <polygon points="342,170 358,160 358,165 385,165 385,175 358,175 358,180" fill="#16a34a" opacity="0.6"/>
  <text x="363" y="195" font-size="10" fill="#16a34a" font-weight="bold">Correct</text>
  <!-- LEFT: Before correction -->
  <g transform="translate(10, 40)">
    <text x="160" y="14" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Before Correction</text>
    <text x="160" y="28" text-anchor="middle" font-size="9" fill="#6b7280">Samples cluster by batch</text>
    <rect x="20" y="35" width="280" height="245" rx="6" fill="white" stroke="#e5e7eb" stroke-width="1"/>
    <!-- Axes -->
    <line x1="40" y1="260" x2="280" y2="260" stroke="#9ca3af" stroke-width="0.5"/>
    <line x1="40" y1="260" x2="40" y2="50" stroke="#9ca3af" stroke-width="0.5"/>
    <text x="160" y="278" text-anchor="middle" font-size="9" fill="#6b7280">PC1 (35% batch)</text>
    <text x="28" y="155" text-anchor="middle" font-size="9" fill="#6b7280" transform="rotate(-90, 28, 155)">PC2</text>
    <!-- Cluster: Memorial (blue circles + triangles) -->
    <circle cx="80" cy="100" r="4" fill="#2563eb"/><circle cx="90" cy="110" r="4" fill="#2563eb"/>
    <circle cx="75" cy="120" r="4" fill="#2563eb"/><circle cx="95" cy="95" r="4" fill="#2563eb"/>
    <polygon points="85,88 89,80 81,80" fill="#2563eb"/><polygon points="100,115 104,107 96,107" fill="#2563eb"/>
    <polygon points="70,108 74,100 66,100" fill="#2563eb"/>
    <!-- Cluster: Hopkins (red) -->
    <circle cx="170" cy="180" r="4" fill="#dc2626"/><circle cx="180" cy="170" r="4" fill="#dc2626"/>
    <circle cx="165" cy="195" r="4" fill="#dc2626"/><circle cx="185" cy="185" r="4" fill="#dc2626"/>
    <polygon points="175,200 179,192 171,192" fill="#dc2626"/><polygon points="190,175 194,167 186,167" fill="#dc2626"/>
    <polygon points="160,185 164,177 156,177" fill="#dc2626"/>
    <!-- Cluster: Mayo (green) -->
    <circle cx="245" cy="80" r="4" fill="#16a34a"/><circle cx="255" cy="90" r="4" fill="#16a34a"/>
    <circle cx="240" cy="95" r="4" fill="#16a34a"/><circle cx="260" cy="75" r="4" fill="#16a34a"/>
    <polygon points="250,100 254,92 246,92" fill="#16a34a"/><polygon points="235,82 239,74 231,74" fill="#16a34a"/>
    <!-- Labels -->
    <text x="85" y="75" text-anchor="middle" font-size="8" fill="#2563eb">Memorial</text>
    <text x="175" y="155" text-anchor="middle" font-size="8" fill="#dc2626">Hopkins</text>
    <text x="250" y="62" text-anchor="middle" font-size="8" fill="#16a34a">Mayo</text>
    <!-- Legend inside -->
    <circle cx="55" cy="240" r="3" fill="#6b7280"/>
    <text x="62" y="243" font-size="8" fill="#6b7280">Tumor</text>
    <polygon points="105,243 108,237 102,237" fill="#6b7280"/>
    <text x="115" y="243" font-size="8" fill="#6b7280">Normal</text>
  </g>
  <!-- RIGHT: After correction -->
  <g transform="translate(380, 40)">
    <text x="160" y="14" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">After Correction</text>
    <text x="160" y="28" text-anchor="middle" font-size="9" fill="#6b7280">Samples cluster by biology</text>
    <rect x="20" y="35" width="280" height="245" rx="6" fill="white" stroke="#e5e7eb" stroke-width="1"/>
    <!-- Axes -->
    <line x1="40" y1="260" x2="280" y2="260" stroke="#9ca3af" stroke-width="0.5"/>
    <line x1="40" y1="260" x2="40" y2="50" stroke="#9ca3af" stroke-width="0.5"/>
    <text x="160" y="278" text-anchor="middle" font-size="9" fill="#6b7280">PC1 (15% biology)</text>
    <text x="28" y="155" text-anchor="middle" font-size="9" fill="#6b7280" transform="rotate(-90, 28, 155)">PC2</text>
    <!-- Tumor cluster (left, circles from all centers mixed) -->
    <circle cx="90" cy="130" r="4" fill="#2563eb"/><circle cx="100" cy="145" r="4" fill="#dc2626"/>
    <circle cx="80" cy="150" r="4" fill="#16a34a"/><circle cx="110" cy="135" r="4" fill="#2563eb"/>
    <circle cx="95" cy="160" r="4" fill="#dc2626"/><circle cx="85" cy="120" r="4" fill="#16a34a"/>
    <circle cx="105" cy="155" r="4" fill="#2563eb"/><circle cx="75" cy="140" r="4" fill="#dc2626"/>
    <circle cx="115" cy="125" r="4" fill="#16a34a"/>
    <!-- Normal cluster (right, triangles from all centers mixed) -->
    <polygon points="210,110 214,102 206,102" fill="#2563eb"/><polygon points="220,125 224,117 216,117" fill="#dc2626"/>
    <polygon points="200,120 204,112 196,112" fill="#16a34a"/><polygon points="230,105 234,97 226,97" fill="#2563eb"/>
    <polygon points="215,135 219,127 211,127" fill="#dc2626"/><polygon points="205,100 209,92 201,92" fill="#16a34a"/>
    <polygon points="225,115 229,107 221,107" fill="#2563eb"/><polygon points="195,130 199,122 191,122" fill="#16a34a"/>
    <!-- Cluster labels -->
    <rect x="65" y="170" width="65" height="18" rx="3" fill="#fef2f2" stroke="#dc2626" stroke-width="0.5"/>
    <text x="97" y="182" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">Tumor</text>
    <rect x="185" y="75" width="65" height="18" rx="3" fill="#eff6ff" stroke="#2563eb" stroke-width="0.5"/>
    <text x="217" y="87" text-anchor="middle" font-size="9" fill="#2563eb" font-weight="bold">Normal</text>
    <!-- Legend -->
    <circle cx="65" cy="240" r="3" fill="#2563eb"/>
    <text x="72" y="243" font-size="8" fill="#2563eb">Mem</text>
    <circle cx="105" cy="240" r="3" fill="#dc2626"/>
    <text x="112" y="243" font-size="8" fill="#dc2626">Hop</text>
    <circle cx="145" cy="240" r="3" fill="#16a34a"/>
    <text x="152" y="243" font-size="8" fill="#16a34a">Mayo</text>
    <text x="200" y="243" font-size="8" fill="#6b7280">(centers now intermixed)</text>
  </g>
</svg>
</div>

```bio
# Visualize the effect of batch correction with PCA

# Step 1: PCA on raw data (batch-contaminated)
let pca_before = pca(expr_matrix, 3)
print("=== Before Correction ===")
print("PC1 variance: {(pca_before.variance_explained[0] * 100) |> round(1)}% (likely batch)")

# Step 2: Regress out batch effect from each gene
let is_hopkins = center |> map(|c| if c == "Hopkins" { 1.0 } else { 0.0 })
let is_mayo = center |> map(|c| if c == "Mayo" { 1.0 } else { 0.0 })

let corrected_matrix = []
for i in 0..n_samples {
    let corrected_sample = []
    for g in 0..n_genes {
        let gene_expr = expr_matrix |> map(|row| row[g])
        let batch_data = table({"expr": gene_expr, "hopkins": is_hopkins, "mayo": is_mayo})
        let model = lm("expr ~ hopkins + mayo", batch_data)

        let residual = gene_expr[i] - model.coefficients[0] * is_hopkins[i]
            - model.coefficients[1] * is_mayo[i]
        corrected_sample = corrected_sample + [residual]
    }
    corrected_matrix = corrected_matrix + [corrected_sample]
}

# Step 3: PCA on corrected data
let pca_after = pca(corrected_matrix, 3)
print("\n=== After Correction ===")
print("PC1 variance: {(pca_after.variance_explained[0] * 100) |> round(1)}% (should be biology now)")

# Compare side by side
let pca_corrected = table({
    "PC1": pca_after.scores |> map(|s| s[0]),
    "PC2": pca_after.scores |> map(|s| s[1]),
    "condition": condition
})
pca_plot(pca_corrected)
```

### Detecting the Confounded Design Trap

```bio
# Check whether batch and condition are confounded
print("=== Balance Check ===")
print("Center       Tumor    Normal   % Tumor")

let centers = ["Memorial", "Hopkins", "Mayo"]
for c in centers {
    let n_tumor = 0
    let n_normal = 0
    for i in 0..n_samples {
        if center[i] == c {
            if condition[i] == "Tumor" { n_tumor = n_tumor + 1 }
            else { n_normal = n_normal + 1 }
        }
    }
    let pct = (n_tumor / (n_tumor + n_normal) * 100) |> round(1)
    print("{c}       {n_tumor}       {n_normal}      {pct}%")
}

# If one center is 100% tumor and another is 100% normal,
# the design is fatally confounded — no statistical fix exists
print("\nDesign assessment:")
let balanced = true
for c in centers {
    let has_tumor = false
    let has_normal = false
    for i in 0..n_samples {
        if center[i] == c {
            if condition[i] == "Tumor" { has_tumor = true }
            else { has_normal = true }
        }
    }
    if !has_tumor || !has_normal {
        print("FATAL: {c} has only one condition!")
        balanced = false
    }
}
if balanced {
    print("Design is balanced — batch correction is possible")
}
```

**Python:**

```python
import numpy as np
from sklearn.decomposition import PCA
import matplotlib.pyplot as plt

# PCA for batch detection
pca = PCA(n_components=5)
scores = pca.fit_transform(expr_matrix)
plt.scatter(scores[:, 0], scores[:, 1], c=batch_labels, cmap='Set1')
plt.title('PCA Colored by Batch')

# ComBat batch correction
from combat.pycombat import pycombat
corrected = pycombat(expr_df, batch_series)

# SVA for unknown batches
# Use the sva package via rpy2 or pydeseq2

# Include batch as covariate in DE analysis
import statsmodels.api as sm
X = sm.add_constant(pd.get_dummies(df[['condition', 'center']], drop_first=True))
model = sm.OLS(gene_expr, X).fit()
```

**R:**

```r
# PCA for batch detection
pca <- prcomp(t(expr_matrix), scale. = TRUE)
plot(pca$x[,1], pca$x[,2], col = batch, pch = 19)

# ComBat batch correction
library(sva)
corrected <- ComBat(dat = expr_matrix, batch = center)

# ComBat-seq for RNA-seq counts
corrected <- ComBat_seq(counts = count_matrix, batch = center)

# SVA for unknown batches
mod <- model.matrix(~ condition)
mod0 <- model.matrix(~ 1, data = pdata)
svobj <- sva(expr_matrix, mod, mod0)

# Include in DE model (limma)
design <- model.matrix(~ condition + center)
fit <- lmFit(expr_matrix, design)
fit <- eBayes(fit)
topTable(fit, coef = "conditionTumor")
```

## Exercises

### Exercise 1: Detect Batch Effects

Given a simulated expression matrix with hidden batch effects, use PCA and ANOVA to identify which technical variable is causing the problem.

```bio
let n = 120
let n_genes = 30

# Simulate with batch effect on processing_date
# Biology: treatment vs control
# Batch: 3 processing dates

# 1. Run pca() and color by treatment, then by processing date
# 2. Which variable dominates PC1?
# 3. How many genes show significant batch effects (anova)?
# 4. How many show significant treatment effects?
```

### Exercise 2: Balanced vs. Confounded Design

Create two study designs: one where batch and condition are balanced, another where they are completely confounded. Show that the confounded design cannot be corrected.

```bio

# Design A (balanced): equal tumor/normal at each center
# Design B (confounded): all tumor at Center 1, all normal at Center 2

# 1. Simulate data for both designs with identical biological effects
# 2. Apply batch correction (include center as covariate in lm())
# 3. Compare: does Design A recover the true biological signal?
# 4. Does Design B? Why or why not?
```

### Exercise 3: Before/After Correction

Apply batch correction to a multi-center dataset and create a before/after PCA comparison. Quantify how much the batch effect is reduced.

```bio

# 1. Simulate 200 samples from 4 centers
# 2. pca() before correction — measure batch variance (anova on PC1 scores)
# 3. Correct by regressing out center effects with lm()
# 4. pca() after correction — measure batch variance again
# 5. What percentage of the batch effect was removed?
```

### Exercise 4: Simpson's Paradox

Simulate a drug trial where Drug A is better within every hospital, but Drug B appears better overall due to confounding. Demonstrate the paradox numerically.

```bio

# Hospital 1: treats mild cases (80% survival baseline)
#   Drug A: 85% survival, Drug B: 75% survival
# Hospital 2: treats severe cases (30% survival baseline)
#   Drug A: 35% survival, Drug B: 25% survival
# Hospital 2 uses Drug A more often

# 1. Show Drug A wins within each hospital
# 2. Combine data — show Drug B appears better overall
# 3. Explain why (confounding by disease severity)
# 4. What analysis would give the correct answer?
```

### Exercise 5: Designing a Batch-Robust Study

You have 60 tumor and 60 normal samples that must be processed across 3 days (40 per day). Design the processing schedule to minimize confounding, and simulate data to verify your design resists batch effects.

```bio
# Good design: 20 tumor + 20 normal per day
# Bad design: 40 tumor on day 1, 40 tumor on day 2, 60 normal on day 3

# 1. Create the good balanced assignment
# 2. Create the bad confounded assignment
# 3. Simulate data with identical batch and biological effects
# 4. Analyze both designs — which recovers the true biology?
```

## Key Takeaways

- **Batch effects** are systematic technical differences that can dominate biological signals — they are present in virtually all high-throughput datasets
- **PCA** is the most powerful tool for detecting batch effects: color by batch and biological variables to see which dominates
- **Confounders** create spurious associations (or mask real ones); **Simpson's paradox** is the extreme case where aggregate trends reverse within subgroups
- **Prevention** is the best strategy: use balanced, randomized designs that distribute biological conditions across batches
- **Confounded designs** (batch = biology) cannot be rescued by any statistical method — the experiment must be redesigned
- **Correction approaches:** include batch as a covariate (simplest), ComBat (empirical Bayes), SVA (discover unknown batches), or RUVseq (negative controls)
- Always perform a **balance check** before analysis: ensure no batch variable is perfectly correlated with the biological variable of interest
- **Before/after PCA** is the standard way to demonstrate that batch correction worked

## What's Next

With three full weeks of biostatistics under your belt, you're ready for the advanced topics of Week 4. Day 21 introduces **dimensionality reduction** — PCA, t-SNE, and UMAP — the tools that turn 20,000-dimensional gene expression data into interpretable 2D visualizations.
