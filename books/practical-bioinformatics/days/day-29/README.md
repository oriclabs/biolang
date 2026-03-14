# Day 29: Capstone --- RNA-seq Differential Expression Study

| | |
|---|---|
| **Difficulty** | Advanced |
| **Biology knowledge** | Advanced (gene expression, RNA-seq, statistical testing, functional genomics) |
| **Coding knowledge** | Advanced (all prior topics: pipes, tables, statistics, visualization, APIs) |
| **Time** | ~4--5 hours |
| **Prerequisites** | Days 1--28 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (simulated count matrix, sample metadata) |

## Quick Start

```bash
cd days/day-29
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Loading and validating an RNA-seq count matrix
- Quality assessment (library sizes, zero-count genes)
- Normalization (CPM, TPM)
- Differential expression analysis (log2 fold change, t-test)
- Multiple testing correction (Benjamini-Hochberg FDR)
- Volcano plot and heatmap visualization
- GO enrichment and pathway analysis
- Publication-ready summary generation

## Output Files

| File | Description |
|---|---|
| `data/output/de_genes.tsv` | Significant differentially expressed genes |
| `data/output/volcano.svg` | Volcano plot (log2FC vs -log10 padj) |
| `data/output/heatmap.svg` | Heatmap of top 20 DE genes |
| `data/output/summary.txt` | Pipeline summary statistics |
