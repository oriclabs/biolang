# Day 13: Language Comparison --- Gene Expression and RNA-seq

## Line Counts

| Operation | BioLang | Python (pandas+scipy) | R (DESeq2) |
|-----------|---------|----------------------|------------|
| Load CSV | 1 | 1 | 1 |
| Library sizes | 3 | 4 | 4 |
| CPM normalization | 1 | 4 | 2 (edgeR) |
| TPM normalization | 3 | 6 | 5 |
| Mean per condition | 3 | 5 | 5 |
| Fold change | 4 | 5 | 4 |
| Differential expression | 4 | 8 | 10 (DESeq2 setup) |
| Filter significant | 3 | 2 | 2 |
| Multiple testing | 2 | 2 | 1 |
| Volcano plot | 1 | N/A (matplotlib) | 12 |
| Export CSV | 1 | 1 | 1 |
| **Total script** | **~70** | **~110** | **~120** |

## Key Differences

### CPM Normalization

```
# BioLang — built-in function, returns normalized table
let norm = cpm(counts)

# Python — manual calculation
for s in samples:
    total = counts[s].sum()
    cpm[s] = counts[s] / total * 1_000_000

# R (edgeR) — one function call
library(edgeR)
cpm_vals <- cpm(count_matrix)

# R (manual)
cpm_manual <- sweep(count_matrix, 2, colSums(count_matrix), "/") * 1e6
```

### Differential Expression

```
# BioLang — one function call with named arguments
let de = diff_expr(counts,
    control: ["normal_1", "normal_2", "normal_3"],
    treatment: ["tumor_1", "tumor_2", "tumor_3"]
)

# Python — loop through genes with scipy t-test + statsmodels correction
for _, row in counts.iterrows():
    t_stat, pvalue = stats.ttest_ind(tumor_vals, normal_vals)
reject, padj, _, _ = multipletests(pvalues, method="fdr_bh")

# R (DESeq2) — requires colData, design formula, relevel, DESeq call
col_data <- data.frame(condition = factor(c(rep("normal", 3), rep("tumor", 3))))
dds <- DESeqDataSetFromMatrix(countData = count_matrix, colData = col_data, design = ~ condition)
dds$condition <- relevel(dds$condition, ref = "normal")
dds <- DESeq(dds, quiet = TRUE)
res <- results(dds, contrast = c("condition", "tumor", "normal"))
```

### Volcano Plot

```
# BioLang — one function call
volcano(de, fc_threshold: 1.0, p_threshold: 0.05, title: "Tumor vs Normal")

# Python — requires matplotlib (20+ lines for labeled, colored plot)
# R — base plot or ggplot2 (12+ lines for labeled, colored plot)
```

### Multiple Testing Correction

```
# BioLang
let adjusted = p_adjust(raw_pvals, "BH")

# Python
from statsmodels.stats.multitest import multipletests
reject, padj, _, _ = multipletests(pvalues, method="fdr_bh")

# R — built in
padj <- p.adjust(pvalues, method = "BH")
```

## Summary

BioLang provides domain-specific functions (`cpm`, `tpm`, `diff_expr`, `volcano`, `p_adjust`) that replace multi-step procedures in Python and R. The pipe syntax keeps analysis pipelines readable and composable. R's DESeq2 provides more sophisticated statistical models (negative binomial GLM vs t-test), but BioLang's `diff_expr` handles the common case with zero boilerplate.
