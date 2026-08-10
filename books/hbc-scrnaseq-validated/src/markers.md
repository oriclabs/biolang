# Markers and interpretation

Integrated coordinates are appropriate for grouping cells, not for estimating
RNA fold changes. Marker discovery therefore returns to log-normalized,
uncorrected RNA while preserving the graph-derived cluster labels.

```biolang
let marker_ready = sc.normalize(clustered, 10000.0)
let markers = sc.find_all_markers(marker_ready, 0.1, 0.25, true)
```

For every cluster, the sparse native implementation compares that cluster with
all other cells, computes detection fractions and an average log2 fold change,
performs a Wilcoxon rank-sum test, and applies Benjamini-Hochberg correction.
The measured BioLang GPU partition returned 9,359 positive marker rows. The
independent Seurat partition returned 12,634 with the same `min.pct = 0.1`,
positive-only, and log2-fold-change threshold of 0.25.

## Biological checkpoint

The prespecified panel was `CD3D`, `IL7R`, `CD8A`, `GNLY`, `NKG7`, `MS4A1`,
`CD79A`, `CD14`, `LYZ`, `S100A8`, `FCGR3A`, `MS4A7`, `FCER1A`, `CST3`, and
`PPBP`. All 15 occur in both positive marker tables, showing that the broad
PBMC signals remain detectable. That is not the same as cluster-level parity:
after optimally mapping the 15 BioLang clusters to 15 of Seurat's 19 clusters,
only 5 of 15 genes peak in the same mapped cluster.

Across every positive cluster-gene pair, 3,291 pairs overlap: 35.16 percent of
the smaller table and a 17.60 percent Jaccard. Comparing the top 50 genes for
each mapped BioLang cluster gives 199 matches out of 750, or 26.53 percent.
Marker agreement is therefore partial, not the previously claimed "8 of 8"
validation.

Cluster integers are intentionally not compared literally. BioLang split the
data into 15 communities, the historical HBC object has 17, and the current
external reference run has 19. A label such as "cluster 10" is local to one
particular graph partition.

Marker recovery demonstrates retained biological signal, but it does not erase
the integration mismatch. Before assigning cell types, inspect multiple
markers, detection fractions, sample composition, and cluster stability. For
condition-level inference, aggregate counts by biological replicate rather
than treating thousands of cells from the same donor as independent samples.
