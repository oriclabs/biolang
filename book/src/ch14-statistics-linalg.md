# Chapter 12: Statistics and Linear Algebra

Bioinformatics is quantitative at its core. Whether you are testing for
differential expression, running principal component analysis on a
transcriptome, or modeling drug kinetics, you need robust numerical tools.
BioLang provides built-in statistics functions, matrix operations, and an ODE
solver so these analyses live alongside your data processing code.

## Descriptive Statistics

All descriptive stats operate on lists of numbers.

```
let coverage = tsv("sample_depth.tsv") |> col("depth")

let summary = {
  mean: mean(coverage),
  median: median(coverage),
  sd: stdev(coverage),
  var: variance(coverage),
  q25: quantile(coverage, 0.25),
  q75: quantile(coverage, 0.75),
  iqr: quantile(coverage, 0.75) - quantile(coverage, 0.25),
}

print("Coverage: mean=" + str(summary.mean) + " median=" + str(summary.median))
```

These functions handle missing values gracefully: `None` entries are skipped
with a warning.

## Hypothesis Testing

### t-test

Compare expression levels between two conditions.

```
let tumor = tsv("expr.tsv") |> filter(|r| r.group == "tumor") |> col("BRCA1")
let normal = tsv("expr.tsv") |> filter(|r| r.group == "normal") |> col("BRCA1")

let result = ttest(tumor, normal)
# result => {statistic: 4.32, pvalue: 0.00018, df: 28, mean_diff: 2.15}

if result.pvalue < 0.05 then
  print("BRCA1 significantly different (p=" + str(result.pvalue) + ")")
```

### Wilcoxon Rank-Sum

For non-normally distributed data such as read counts.

```
let treated = [1240, 890, 1560, 2100, 780, 1890, 1345]
let control = [450, 520, 380, 610, 490, 430, 550]

let result = wilcoxon(treated, control)
# result => {statistic: 49.0, pvalue: 0.0012}
```

### Chi-squared Test

Test whether observed genotype frequencies match Hardy-Weinberg expectations.

```
let observed = [210, 480, 310]  # AA, Aa, aa genotypes
let n = sum(observed)
let p = (2 * observed[0] + observed[1]) / (2 * n)
let q = 1.0 - p
let expected = [p * p * n, 2 * p * q * n, q * q * n]

let result = chi_square(observed, expected)
# result => {statistic: 3.21, pvalue: 0.073, df: 1}

if result.pvalue > 0.05 then
  print("Population is in Hardy-Weinberg equilibrium")
```

### ANOVA

Compare expression across multiple tissue types.

```
let brain = tsv("expr.tsv") |> filter(|r| r.tissue == "brain") |> col("TP53")
let liver = tsv("expr.tsv") |> filter(|r| r.tissue == "liver") |> col("TP53")
let lung = tsv("expr.tsv") |> filter(|r| r.tissue == "lung") |> col("TP53")
let kidney = tsv("expr.tsv") |> filter(|r| r.tissue == "kidney") |> col("TP53")

let result = anova(brain, liver, lung, kidney)
# result => {f_statistic: 8.74, pvalue: 0.00003, df_between: 3, df_within: 76}
```

### Fisher's Exact Test

Test enrichment of a mutation in cases versus controls.

```
# Contingency table: [[mutation+/case, mutation-/case], [mutation+/ctrl, mutation-/ctrl]]
let table = [[45, 155], [12, 188]]
let result = fisher_exact(table)
# result => {odds_ratio: 5.48, pvalue: 0.00001}
```

## Multiple Testing Correction

When testing thousands of genes, you must correct for multiple comparisons.
`p_adjust` supports Bonferroni, Benjamini-Hochberg (BH), and
Benjamini-Yekutieli (BY) methods.

```
let genes = tsv("gene_expression.tsv")
let gene_names = genes |> col("gene_name")
let tumor_cols = colnames(genes) |> filter(|c| starts_with(c, "tumor_"))
let normal_cols = colnames(genes) |> filter(|c| starts_with(c, "normal_"))

let pvalues = range(0, genes.num_rows)
  |> map(|i| {
       let t = tumor_cols |> map(|c| col(genes, c)[i])
       let n = normal_cols |> map(|c| col(genes, c)[i])
       ttest(t, n).pvalue
     })

let adjusted_bh = p_adjust(pvalues, "BH")
let adjusted_bonf = p_adjust(pvalues, "bonferroni")

let significant = range(0, len(adjusted_bh))
  |> filter(|i| adjusted_bh[i] < 0.05)
  |> map(|i| {gene: gene_names[i], p: pvalues[i], q: adjusted_bh[i]})

print(str(len(significant)) + " genes significant at FDR < 0.05")
```

## Correlation

Compute Pearson or Spearman correlation between expression profiles.

```
let expr = tsv("tpm_matrix.tsv")
let gene_a = expr |> col("EGFR")
let gene_b = expr |> col("ERBB2")

let pearson = cor(gene_a, gene_b)
# pearson => {r: 0.72, pvalue: 0.00001}

let spearman = spearman(gene_a, gene_b)
# spearman => {rho: 0.68, pvalue: 0.00004}
```

## Linear Models

Fit a linear model to predict expression from covariates.

```
let data = tsv("sample_metadata.tsv")
let expression = data |> col("gene_expr")
let age = data |> col("age")
let batch = data |> col("batch_id")

let model = lm(expression, [age, batch])
# model => {
#   coefficients: [{name: "intercept", estimate: 5.2, se: 0.8, p: 0.001},
#                  {name: "x1", estimate: 0.03, se: 0.01, p: 0.02},
#                  {name: "x2", estimate: -1.1, se: 0.4, p: 0.008}],
#   r_squared: 0.45,
#   adj_r_squared: 0.42,
#   f_statistic: 12.3,
#   pvalue: 0.00002
# }
```

## Matrix Creation and Operations

Create matrices from data and perform standard operations.

```
# Create a 3x3 count matrix (genes x samples)
let counts = matrix([
  [120, 340, 250],
  [890, 1200, 1050],
  [45, 30, 55]
])

# Transpose: samples x genes
let transposed = transpose(counts)

# Matrix multiplication
let product = mat_mul(transpose(counts), counts)  # 3x3 covariance-like

# Element-wise operations with mat_map
let log_counts = mat_map(counts, |x| log2(x + 1))

# Basic properties
print("Trace: " + str(trace(product)))
print("Rank: " + str(rank(counts)))
print("Frobenius norm: " + str(norm(counts)))
```

## Linear Algebra

### Determinant and Inverse

```
let cov = matrix([
  [1.0, 0.8, 0.3],
  [0.8, 1.0, 0.5],
  [0.3, 0.5, 1.0]
])

let det = determinant(cov)
print("Determinant: " + str(det))  # non-zero => invertible

let precision = inverse(cov)  # precision matrix
```

### Eigenvalues and Eigenvectors

```
let eigen = eigenvalues(cov)
# eigen => {values: [2.1, 0.7, 0.2], vectors: [[...], [...], [...]]}

# Proportion of variance explained by each component
let total = sum(eigen.values)
let prop = eigen.values |> map(|v| v / total)
print("PC1 explains " + str(prop[0] * 100) + "% of variance")
```

### Singular Value Decomposition

```
let result = svd(counts)
# result => {u: matrix, s: [singular values], v: matrix}
```

### Solving Linear Systems

```
# Solve Ax = b
let a = matrix([[2, 1], [1, 3]])
let b = [5, 11]
let x = solve(a, b)
# x => [1.0, 3.0]
```

## ODE Solving

`ode_solve` uses a 4th-order Runge-Kutta integrator. The signature is:

```
ode_solve(derivative_fn, initial_state, [t_start, t_end, dt])
```

It returns `{t: [...], y: [...]}` where `y` is a list of state vectors at
each time point.

## Example: Differential Expression Analysis

Run t-tests across all genes and correct for multiple testing.

```
let expr = tsv("counts_normalized.tsv")
let metadata = tsv("sample_info.tsv")

let case_ids = metadata |> filter(|r| r.condition == "treated") |> col("sample_id")
let ctrl_ids = metadata |> filter(|r| r.condition == "control") |> col("sample_id")

let gene_names = expr |> col("gene_id")
let results = gene_names |> map(|gene| {
  let case_vals = case_ids |> map(|id| expr[id][index_of(gene_names, gene)])
  let ctrl_vals = ctrl_ids |> map(|id| expr[id][index_of(gene_names, gene)])
  let test = ttest(case_vals, ctrl_vals)
  let fc = mean(case_vals) / mean(ctrl_vals)
  {gene: gene, log2fc: log2(fc), pvalue: test.pvalue}
})

let p_vals = results |> map(|r| r.pvalue)
let q_vals = p_adjust(p_vals, "BH")

let de_genes = range(0, len(results))
  |> map(|i| {...results[i], q_value: q_vals[i]})
  |> filter(|r| r.q_value < 0.05 && abs(r.log2fc) > 1.0)
  |> sort_by(|r| r.q_value)

print(str(len(de_genes)) + " differentially expressed genes")
de_genes |> take(20) |> each(|g| print(g.gene + " log2FC=" + str(g.log2fc)))
de_genes |> write_tsv("de_results.tsv")
```

## Example: PCA on Gene Expression

Use the covariance matrix eigendecomposition to project samples into PC space.

```
let expr = tsv("tpm_matrix.tsv")
let sample_ids = expr |> colnames() |> filter(|n| n != "gene_id")
let n_genes = expr.num_rows
let n_samples = len(sample_ids)

# Build samples x genes matrix
let data = sample_ids
  |> map(|id| expr |> col(id))
  |> matrix()

# Center each gene (column) to zero mean
let centered = range(0, n_genes)
  |> fold(data, |mat, j| {
       let col = mat_col(mat, j)
       let mu = mean(col)
       mat_set_col(mat, j, col |> map(|x| x - mu))
     })

# Covariance matrix (samples x samples)
let cov = mat_mul(centered, transpose(centered))
  |> mat_map(|x| x / (n_genes - 1))

# Eigendecomposition
let eigen = eigenvalues(cov)
let total_var = sum(eigen.values)

# Project onto first 2 PCs
let pc1 = mat_col(eigen.vectors, 0)
let pc2 = mat_col(eigen.vectors, 1)

let pca_coords = range(0, n_samples)
  |> map(|i| {
       sample: sample_ids[i],
       pc1: pc1[i],
       pc2: pc2[i],
     })

print("PC1: " + str(eigen.values[0] / total_var * 100) + "% variance")
print("PC2: " + str(eigen.values[1] / total_var * 100) + "% variance")
pca_coords |> write_tsv("pca_coordinates.tsv")
```

## Example: Pharmacokinetic Modeling

Model oral drug absorption and elimination using a two-compartment ODE system.

```
# Parameters for a typical oral drug
let ka = 1.0      # absorption rate (1/hr)
let ke = 0.2      # elimination rate (1/hr)
let dose = 500.0  # mg

# State: [gut_amount, plasma_concentration]
# gut depletes by absorption, plasma gains from gut and loses by elimination
let pk_model = |t, y| [
  -ka * y[0],              # dA_gut/dt = -ka * A_gut
  ka * y[0] - ke * y[1],   # dC_plasma/dt = ka * A_gut - ke * C_plasma
]

let initial = [dose, 0.0]
let solution = ode_solve(pk_model, initial, [0.0, 24.0, 0.1])

# Find Cmax and Tmax
let plasma = solution.y |> map(|state| state[1])
let cmax = max(plasma)
let tmax_idx = index_of(plasma, cmax)
let tmax = solution.t[tmax_idx]

print("Tmax: " + str(tmax) + " hr")
print("Cmax: " + str(cmax) + " mg")

# Half-life
let t_half = log(2) / ke
print("Half-life: " + str(t_half) + " hr")

# AUC by trapezoidal rule
let auc = range(0, len(plasma) - 1)
  |> map(|i| (plasma[i] + plasma[i + 1]) / 2.0 * (solution.t[i + 1] - solution.t[i]))
  |> sum()
print("AUC(0-24h): " + str(auc) + " mg*hr")
```

## Example: Population Genetics (Hardy-Weinberg)

Simulate genotype frequencies and test for equilibrium across loci.

```
# Observed genotype counts at 50 SNP loci
let loci = tsv("genotype_counts.tsv")
# columns: locus, AA, Aa, aa

let hwe_results = loci |> map(|loc| {
  let obs = [loc.AA, loc.Aa, loc.aa]
  let n = sum(obs)
  let p = (2 * loc.AA + loc.Aa) / (2 * n)
  let q = 1.0 - p

  let exp = [p * p * n, 2 * p * q * n, q * q * n]
  let test = chi_square(obs, exp)

  {
    locus: loc.locus,
    p_freq: p,
    q_freq: q,
    chi2: test.statistic,
    pvalue: test.pvalue,
  }
})

# Correct for multiple testing
let p_vals = hwe_results |> map(|r| r.pvalue)
let q_vals = p_adjust(p_vals, "BH")

let departures = range(0, len(hwe_results))
  |> map(|i| {...hwe_results[i], q_value: q_vals[i]})
  |> filter(|r| r.q_value < 0.05)

print(str(len(departures)) + " loci depart from HWE (FDR < 0.05):")
departures |> each(|r| print("  " + r.locus + " chi2=" + str(r.chi2)
                              + " q=" + str(r.q_value)))
```

## Summary

BioLang's numerical toolbox covers the statistical tests and linear algebra
operations that appear throughout computational biology. Descriptive stats,
hypothesis tests with multiple-testing correction, matrix decompositions, and
ODE integration are all available as built-in functions, keeping your analysis
in a single language from raw data to publication-ready results.
