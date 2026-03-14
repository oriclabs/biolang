# Day 15: Language Comparison --- Visualization

## Creating a Volcano Plot

### BioLang (2 lines)

```
let de = csv("data/de_results.csv")
volcano(de, {fc_threshold: 1.0, p_threshold: 0.05, title: "Tumor vs Normal"})
```

### Python (15+ lines)

```python
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

de = pd.read_csv("data/de_results.csv")
de["-log10p"] = -np.log10(de["padj"])

fig, ax = plt.subplots(figsize=(8, 6))
colors = ["grey"] * len(de)
for i, row in de.iterrows():
    if row["padj"] < 0.05 and abs(row["log2fc"]) > 1.0:
        colors[i] = "red" if row["log2fc"] > 0 else "blue"
ax.scatter(de["log2fc"], de["-log10p"], c=colors, alpha=0.7)
ax.axhline(-np.log10(0.05), ls="--", color="grey")
ax.axvline(1.0, ls="--", color="grey")
ax.axvline(-1.0, ls="--", color="grey")
ax.set_xlabel("log2 Fold Change")
ax.set_ylabel("-log10(adjusted p-value)")
ax.set_title("Tumor vs Normal")
plt.savefig("figures/volcano.svg")
```

Requires: `pip install matplotlib pandas numpy`

### R (12+ lines)

```r
library(ggplot2)

de <- read.csv("data/de_results.csv")
de$neglog10p <- -log10(de$padj)
de$sig <- ifelse(de$padj < 0.05 & abs(de$log2fc) > 1,
                 ifelse(de$log2fc > 0, "Up", "Down"), "NS")

ggplot(de, aes(x = log2fc, y = neglog10p, color = sig)) +
  geom_point(alpha = 0.7) +
  scale_color_manual(values = c("Up" = "red", "Down" = "blue", "NS" = "grey")) +
  geom_hline(yintercept = -log10(0.05), linetype = "dashed") +
  geom_vline(xintercept = c(-1, 1), linetype = "dashed") +
  labs(title = "Tumor vs Normal", x = "log2 Fold Change", y = "-log10(padj)") +
  theme_minimal()
ggsave("figures/volcano.svg")
```

Requires: `install.packages(c("ggplot2"))`

## Key Differences

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| Lines for volcano plot | 2 | 15+ | 12+ |
| External libraries | None | matplotlib, numpy, pandas | ggplot2 |
| ASCII output | Built-in | Not standard | Not standard |
| SVG export | `format: "svg"` | `plt.savefig()` | `ggsave()` |
| Setup required | None | pip install | install.packages |
| Bio-specific plots | Built-in (volcano, manhattan, etc.) | Manual or EnhancedVolcano | Manual or EnhancedVolcano |

## Saving Figures

### BioLang

```
save_svg(volcano(de, {format: "svg"}), "figures/volcano.svg")
```

### Python

```python
plt.savefig("figures/volcano.svg", format="svg", bbox_inches="tight")
```

### R

```r
ggsave("figures/volcano.svg", width = 8, height = 6)
```

## Sparklines

### BioLang

```
print(sparkline([3, 5, 2, 8, 4, 7]))
```

### Python

No built-in equivalent. Requires `sparklines` package or manual Unicode.

### R

No built-in equivalent. Requires `sparkline` package (HTML widget, not terminal).
