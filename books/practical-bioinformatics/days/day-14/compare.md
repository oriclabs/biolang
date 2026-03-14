# Day 14: Language Comparison --- Statistics for Bioinformatics

## Line Counts

| Operation | BioLang | Python (scipy+statsmodels) | R (base) |
|-----------|---------|---------------------------|----------|
| Descriptive stats | 9 | 11 | 11 |
| t-test (two-sample) | 3 | 2 | 2 |
| t-test (paired) | 2 | 2 | 2 |
| Wilcoxon test | 2 | 2 | 2 |
| ANOVA | 2 | 2 | 4 |
| Pearson correlation | 2 | 2 | 1 |
| Spearman/Kendall | 4 | 4 | 2 |
| Linear regression | 5 | 5 | 5 |
| Multiple testing | 5 | 4 | 3 |
| Chi-square | 2 | 2 | 3 |
| Fisher's exact | 2 | 2 | 3 |
| Full experiment | ~35 | ~40 | ~35 |
| **Total script** | **~90** | **~110** | **~100** |

## Key Differences

### t-test

```
# BioLang — returns a record with .statistic and .pvalue
let result = ttest(normal, tumor)
print(result.pvalue)

# Python — returns a tuple
t_stat, p_val = stats.ttest_ind(normal, tumor)

# R — returns a list
result <- t.test(normal, tumor)
result$p.value
```

### Multiple Testing Correction

```
# BioLang — single function, returns adjusted p-values
let adj = p_adjust(raw_pvals, "BH")

# Python — returns 4 values (reject array, adjusted, alphacSidak, alphacBonf)
reject, adj_p, _, _ = multipletests(raw_pvals, method="fdr_bh")

# R — single function, returns vector
adj <- p.adjust(raw_pvals, method = "BH")
```

### Correlation

```
# BioLang — cor() returns a number, spearman() returns a record
let r = cor(gene_a, gene_b)
let rho = spearman(gene_a, gene_b)

# Python — always returns (statistic, p-value) tuple
r, p = stats.pearsonr(gene_a, gene_b)
rho, p = stats.spearmanr(gene_a, gene_b)

# R — cor() returns a number, cor.test() returns a list
r <- cor(gene_a, gene_b)
result <- cor.test(gene_a, gene_b, method = "spearman")
```

### Linear Regression

```
# BioLang — lm() returns a record with slope, intercept, r_squared, pvalue
let model = lm(x, y)
print(model.r_squared)

# Python — linregress returns 5 values
slope, intercept, r, p, se = stats.linregress(x, y)
r_squared = r**2

# R — lm() returns model object, need summary() for details
model <- lm(y ~ x)
s <- summary(model)
s$r.squared
```

## Observations

1. **BioLang** uses named fields (`.statistic`, `.pvalue`) on result records, which is more readable than positional tuple unpacking (Python) or `$`-indexed lists (R).

2. **Python** requires three separate imports (`numpy`, `scipy.stats`, `statsmodels`) for a complete statistical workflow. BioLang and R have everything built in.

3. **R** has the strongest built-in statistics ecosystem --- `p.adjust()`, `cor.test()`, and formula syntax (`y ~ x`) are very concise. BioLang is similar in spirit but uses explicit function calls.

4. **Multiple testing** is critical for genomics. BioLang and R handle this in one line; Python requires an extra library (`statsmodels`).
