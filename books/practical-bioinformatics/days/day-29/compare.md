# Day 29: Language Comparison --- RNA-seq Differential Expression

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| Data loading | 3 (built-in) | 15 | 4 |
| Sample ID extraction | 3 | 3 | 3 |
| Low-count filter | 4 | 5 | 3 |
| Library size computation | 3 | 3 | 1 |
| CPM normalization | 8 | 6 | 3 |
| DE analysis (FC + t-test) | 16 | 22 | 18 |
| BH multiple testing | 18 | 16 | 10 |
| Significance filter | 4 | 4 | 3 |
| Volcano plot | 10 | 0 (not built-in) | 0 (needs ggplot2) |
| Heatmap | 12 | 0 (needs seaborn) | 0 (needs pheatmap) |
| Output writing | 8 | 12 | 8 |
| Summary report | 16 | 20 | 18 |
| **Total** | **~105** | **~106** | **~71** |

## Key Differences

### Data Loading

```
# BioLang --- built-in TSV support
let counts = read_tsv("data/counts.tsv")
let samples = read_tsv("data/samples.tsv")

# Python --- csv.DictReader boilerplate
def read_tsv(path):
    rows = []
    with open(path) as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            rows.append(row)
    return rows

# R --- one-liner (advantage)
counts <- read.delim("data/counts.tsv", sep = "\t")
```

### Normalization (CPM)

```
# BioLang --- pipe + map
let cpm = filtered |> map(|row| {
    let result = { gene: row.gene }
    range(0, len(sample_ids)) |> each(|i| {
        let sid = sample_ids[i]
        result[sid] = round(float(int(row[sid])) / float(lib_sizes[i]) * 1000000.0, 2)
    })
    result
})

# Python --- explicit loop
cpm = []
for row in filtered:
    cpm_row = {"gene": row["gene"]}
    for sid in sample_ids:
        cpm_row[sid] = round(int(row[sid]) / lib_sizes[sid] * 1e6, 2)
    cpm.append(cpm_row)

# R --- vectorized (advantage)
for (sid in sample_ids) {
  cpm[[sid]] <- round(filtered[[sid]] / lib_sizes[sid] * 1e6, 2)
}
```

### Visualization

```
# BioLang --- built-in volcano and heatmap
let volcano_svg = volcano(x_vals, y_vals, title, xlabel, ylabel)
let hm_svg = heatmap(matrix, title, row_labels, col_labels)

# Python --- requires matplotlib + seaborn (15+ lines each)
import matplotlib.pyplot as plt
fig, ax = plt.subplots(figsize=(10, 8))
ax.scatter(x_vals, y_vals, ...)
ax.axhline(y=-np.log10(0.05), ...)
ax.axvline(x=1, ...)
# ... many more lines

# R --- requires ggplot2 (10+ lines)
library(ggplot2)
ggplot(de_results, aes(x = log2fc, y = -log10(padj))) +
  geom_point(aes(color = significant)) +
  geom_hline(yintercept = -log10(0.05)) +
  geom_vline(xintercept = c(-1, 1)) +
  theme_minimal()
```

### Multiple Testing Correction

```
# BioLang --- manual BH implementation (18 lines)
# (no stats package dependency)

# Python --- scipy.stats or manual (16 lines)
# Or one-liner with statsmodels:
from statsmodels.stats.multitest import multipletests
_, padj, _, _ = multipletests(pvalues, method="fdr_bh")

# R --- built-in p.adjust (1 line, major advantage)
padj <- p.adjust(pvalues, method = "BH")
```

## Summary

BioLang's advantages for RNA-seq DE analysis:

1. **Built-in visualization** --- volcano plots and heatmaps require zero external dependencies, compared to matplotlib/seaborn in Python or ggplot2/pheatmap in R
2. **Pipe-based data flow** --- the analysis reads as a linear pipeline from raw counts to final report
3. **Built-in t-test** --- no scipy or stats package import needed
4. **Single-file analysis** --- no dependency management for the core pipeline

R's advantages:

1. **Vectorized operations** --- R excels at column-wise math on data frames (CPM in 1 line vs 8)
2. **Built-in p.adjust()** --- BH correction is a single function call
3. **DESeq2/edgeR** --- production RNA-seq uses specialized packages with negative binomial models, not t-tests
4. **Bioconductor ecosystem** --- mature, peer-reviewed bioinformatics packages

Python's advantages:

1. **statsmodels** --- one-line BH correction via `multipletests()`
2. **pandas** --- powerful DataFrame operations
3. **scikit-learn** --- ML integration for downstream analysis

For production RNA-seq, R with DESeq2 remains the gold standard. BioLang's strength is rapid exploratory analysis with zero dependency overhead and built-in visualization.
