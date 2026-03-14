# Day 19: Language Comparison --- Biological Data Visualization

## Line Counts

| Operation | BioLang | Python (matplotlib+seaborn) | R (ggplot2+survival) |
|-----------|---------|----------------------------|----------------------|
| Manhattan plot | 1 | 25+ | 20+ |
| QQ plot | 2 | 10 | 8 |
| Violin plot | 1 | 8 | 8 |
| Density plot | 1 | 4 | 4 |
| PCA plot | 1 | 10 | 12 |
| Clustered heatmap | 1 | 2 | 3 |
| Kaplan-Meier | 1 | 8 | 5 |
| ROC curve | 1 | 10 | 6 |
| Forest plot | 1 | 15 | 15 |
| Circos plot | 1 | 30+ (pycircos) | 20+ (circlize) |
| Sequence logo | 1 | 6 (logomaker) | 5 (ggseqlogo) |
| Venn diagram | 1 | 5 (matplotlib-venn) | 3 (VennDiagram) |
| Oncoprint | 1 | 30+ (custom) | 15+ (ComplexHeatmap) |
| Save SVG | 1 | 2 | 2 |
| **Total script** | **~50** | **~200** | **~150** |

## Key Differences

### Manhattan Plot

```
# BioLang — one function call
let gwas = csv("data/gwas.csv")
manhattan(gwas, title: "GWAS Results")

# Python — manual chromosome offset computation, color cycling, threshold line
gwas = pd.read_csv("data/gwas.csv")
gwas["neglog10p"] = -np.log10(gwas["pvalue"])
# ... 20 more lines to compute offsets, plot, color, label ...
fig, ax = plt.subplots(figsize=(14, 5))
for i, chrom in enumerate(chrom_order):
    subset = gwas[gwas["chrom"] == chrom]
    ax.scatter(subset["genome_pos"], subset["neglog10p"], ...)
ax.axhline(y=-np.log10(5e-8), ...)
plt.savefig("manhattan.png")

# R — ggplot2 with manual offsets
gwas <- read.csv("data/gwas.csv")
# ... compute cumulative offsets ...
ggplot(gwas, aes(x = genome_pos, y = neglog10p, color = chrom)) +
  geom_point() + geom_hline(...)
```

### Kaplan-Meier Survival Curve

```
# BioLang — data + one call
let data = [{time: 12, event: 1}, ...] |> to_table()
kaplan_meier(data, title: "Overall Survival")

# Python — requires lifelines library
from lifelines import KaplanMeierFitter
kmf = KaplanMeierFitter()
kmf.fit(data["time"], event_observed=data["event"])
kmf.plot_survival_function(ax=ax)
ax.set_title("Overall Survival")
plt.savefig(...)

# R — survival + survminer packages
fit <- survfit(Surv(time, event) ~ 1, data = km_data)
ggsurvplot(fit, data = km_data, title = "Overall Survival")
```

### Sequence Logo

```
# BioLang — list of strings
sequence_logo(["TATAAAGC", "TATAATGC", ...], title: "TATA Box")

# Python — needs logomaker + matrix conversion
import logomaker
counts = logomaker.alignment_to_matrix(seqs, to_type="information")
logomaker.Logo(counts, ax=ax)
ax.set_title("TATA Box")
plt.savefig(...)

# R — needs ggseqlogo
library(ggseqlogo)
ggseqlogo(seqs) + ggtitle("TATA Box")
```

## Summary

BioLang's 21 built-in bio visualization functions collapse what typically requires 5-30 lines per plot (plus library imports and configuration) into single function calls. The trade-off is flexibility: Python and R allow unlimited customization of every visual element, while BioLang produces standardized plots optimized for common biological questions.

For exploratory analysis and standard figures, BioLang is significantly faster to write. For custom figures requiring precise control over every axis tick and annotation, Python (matplotlib) or R (ggplot2) offer more granularity.
