# Day 16: Language Comparison — Pathway and Enrichment Analysis

## ORA (Over-Representation Analysis)

### BioLang
```
let gene_sets = read_gmt("data/hallmark.gmt")
let de = csv("data/de_results.csv")
let sig_genes = de
    |> filter(|r| r.padj < 0.05 and abs(r.log2fc) > 1.0)
    |> col("gene") |> collect()
let results = enrich(sig_genes, gene_sets, 20000)
print(results |> filter(|r| r.fdr < 0.05) |> arrange("fdr"))
```

### Python (gseapy + scipy)
```python
import pandas as pd
from scipy.stats import hypergeom
from statsmodels.stats.multitest import multipletests

gene_sets = {}
with open("data/hallmark.gmt") as f:
    for line in f:
        fields = line.strip().split("\t")
        gene_sets[fields[0]] = fields[2:]

de = pd.read_csv("data/de_results.csv")
sig = de[(de.padj < 0.05) & (de.log2fc.abs() > 1.0)]
sig_genes = sig.gene.tolist()

results = []
for name, genes in gene_sets.items():
    overlap = [g for g in sig_genes if g in genes]
    k = len(overlap)
    pval = hypergeom.sf(k - 1, 20000, len(genes), len(sig_genes))
    results.append({"term": name, "overlap": k, "p_value": pval})

df = pd.DataFrame(results).sort_values("p_value")
_, fdr, _, _ = multipletests(df.p_value, method="fdr_bh")
df["fdr"] = fdr
print(df[df.fdr < 0.05])
```

### R (clusterProfiler)
```r
library(clusterProfiler)
gmt <- read.gmt("data/hallmark.gmt")
de <- read.csv("data/de_results.csv")
sig <- de[de$padj < 0.05 & abs(de$log2fc) > 1.0, ]
result <- enricher(gene = sig$gene, TERM2GENE = gmt, universe = de$gene)
print(as.data.frame(result)[result@result$p.adjust < 0.05, ])
```

## Key Differences

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| GMT loading | `read_gmt()` | Manual parsing | `read.gmt()` |
| ORA | `enrich()` | scipy + manual BH | `enricher()` |
| GSEA | `gsea()` | `gseapy.prerank()` | `GSEA()` |
| Lines of code | ~6 | ~18 | ~6 |
| Dependencies | None (built-in) | gseapy, scipy, statsmodels | clusterProfiler, BiocManager |
| GO/KEGG queries | Built-in (`go_term`, `kegg_find`) | bioservices or REST | clusterProfiler |
| Network analysis | `string_network()` + `graph()` | networkx + requests | STRINGdb package |

## Verdict

R's clusterProfiler is the gold standard for enrichment analysis and is similarly concise. Python requires more boilerplate for ORA but gseapy simplifies GSEA. BioLang combines GMT loading, ORA, GSEA, GO/KEGG/STRING queries, and graph construction into built-in functions with zero dependencies.
