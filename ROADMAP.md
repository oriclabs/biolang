# BioLang Feature & Package Roadmap

Captured from design review — covering core runtime additions, language features,
file format support, and the package ecosystem. Ordered by impact within each section.

---

## 1. Core Runtime — Statistical Primitives

These are needed by scRNA, bulk RNA-seq, GWAS, survival, methylation, and clinical
analysis simultaneously. Putting them in any single package would force every other
package to depend on it; they belong in the runtime.

### Missing now (high priority)

| Function | Signature | Notes |
|---|---|---|
| `spearman` | `spearman(x, y)` | Rank correlation — used everywhere |
| `kendall` | `kendall(x, y)` | Kendall's tau for small/tied data |
| `mutual_information` | `mutual_information(x, y)` | Feature selection, scRNA gene–gene |
| `pca` | `pca(matrix, n_components=50)` | Returns scores, loadings, explained_variance — too general for any one package |
| `kaplan_meier` | `kaplan_meier(times, events)` | KM estimator with CI |
| `cox_ph` | `cox_ph(times, events, covariates)` | Proportional hazards — cancer critical |
| `log_rank_test` | `log_rank_test(a_t, a_e, b_t, b_e)` | Compare survival curves |
| `quantile_norm` | `quantile_norm(matrix)` | Microarray, ChIP-seq, bulk RNA |
| `batch_correct` | `batch_correct(matrix, batches)` | ComBat-style — every multi-sample study |
| `bootstrap` | `bootstrap(data, stat_fn, n=1000)` | CI for arbitrary statistics |
| `permutation_test` | `permutation_test(a, b, stat_fn, n=10000)` | Non-parametric testing |
| `power_analysis` | `power_analysis(effect, alpha, power, n?)` | Sample size — clinical/research design |
| `meta_analysis` | `meta_analysis(effects, variances, method="fixed")` | GWAS meta-analysis |

### Already added (this sprint)

| Function | Notes |
|---|---|
| `tpm` | Transcripts Per Million |
| `rpkm` | RPKM/FPKM |
| `hardy_weinberg` | HWE chi-square test |
| `fst` | Wright's FST per locus |
| `ld_decay` | LD decay binned by distance |

---

## 2. Core Runtime — Table Operations

Every analysis eventually hits missing table operations. Current workarounds
(filter + map chains) are verbose and slow.

### Missing now (critical)

| Function | Notes |
|---|---|
| `inner_join(a, b, on)` | **Highest single impact** — unblocks ~30% of real workflows |
| `left_join(a, b, on)` | |
| `right_join(a, b, on)` | |
| `anti_join(a, b, on)` | "Genes NOT in gene set" — common in enrichment |
| `pivot_wider(t, id, name, value)` | Count matrix ↔ tidy format |
| `pivot_longer(t, cols, names_to, values_to)` | |
| `unnest(table, list_col)` | Explode list column to rows — scRNA metadata |
| `cross_tab(table, row_col, col_col)` | Contingency tables |
| `rank_col(table, col, method="average")` | |
| `cumsum_col(table, col)` | |
| `rolling_mean(table, col, window)` | Time-series, coverage smoothing |
| `lag(table, col, n=1)` / `lead` | Time series operations |

### Already added (this sprint)

| Function | Notes |
|---|---|
| `bed_intersect` | Overlap between BED-format tables |
| `bed_subtract` | Remove B regions from A |
| `bed_merge` | Merge overlapping intervals |
| `bed_closest` | Nearest feature per row |
| `col_sum` / `col_min` / `col_max` | (fixed bugs) |
| `slice` | (fixed panic bug) |

---

## 3. Core Runtime — Sparse Matrix Type

Single-cell RNA-seq is entirely sparse matrices. Without a native type, every scRNA
package brings its own representation and they can't interoperate.

```biolang
# Construction
let mat = sparse_matrix(rows=10000, cols=30000,
    values=[(cell_idx, gene_idx, count), ...])

# Arithmetic (sparse-aware, not materialised)
mat |> normalize_total(10000) |> log1p()

# Slicing
mat[cell_idx, :]        # one cell (row)
mat[:, gene_idx]        # one gene (column)
mat[0..100, :]          # range of cells

# Properties
mat.nnz                 # non-zero count
mat.density             # nnz / (rows * cols)
mat.shape               # {rows, cols}

# Conversion
mat |> to_dense()       # explicit, warns if large
mat |> to_table()       # long format: {row, col, value}
```

**Impacts**: `scdata`, `dimensionality`, `singlecell`, `spatial`, `methylation` —
all need this before they can be built cleanly.

---

## 4. Core Runtime — Normalisation Dispatch

Unify `tpm` / `rpkm` / future methods under one dispatchable interface so packages
can register custom normalisers that `normalize()` routes to:

```biolang
normalize(matrix, method="total", ...)
# methods: "total", "tpm", "rpkm", "cpm", "quantile", "vst", "rlog", "scran"
# Packages register additional methods at runtime via plugin mechanism
```

---

## 5. Core Runtime — Missing Sequence Primitives

These are used by too many analysis packages to belong in any one of them.

| Function | Notes |
|---|---|
| `six_frame_translate(seq)` | All 6 ORFs — ORF finders, annotation |
| `codon_freq_table(seq)` | Codon usage bias — expression prediction |
| `cpg_islands(seq, min_len=200, gc_thr=0.5, oe_thr=0.6)` | Methylation, promoter prediction, cancer |
| `splice_sites(seq)` | GT-AG / GC-AG canonical detection — RNA-seq packages |
| `repeat_mask(seq, simple=true)` | Soft-mask low-complexity — alignment packages |
| `genomic_bins(chrom_sizes, bin_size)` | Coverage, CNV, ATAC, methylation all need this |
| `tile_genome(chrom_sizes, tile_size)` | Genome-wide tiling |
| `extend_interval(iv, upstream, downstream)` | Promoter analysis, TF binding |
| `flank(iv, size, both=true)` | Flanking regions |
| `center_interval(iv)` | Peak midpoints — Hi-C, ChIP-seq |
| `genome_coverage(intervals, chrom_sizes)` | Fractional coverage |

### Already added (this sprint)

| Function | Notes |
|---|---|
| `find_pattern` | IUPAC-aware, both strands, mismatch tolerance |
| `kmer_index` | k-mer → positions map |
| `windows` | Sliding window extraction |
| `gc_skew` | Per-window with cumulative sum |
| `restriction_sites` | 24 enzymes + "all" + custom IUPAC |
| `align` | NW / SW / semi-global with CIGAR |
| `consensus` | Profile matrix + plurality vote |
| `entropy` | Shannon entropy scan |
| `iupac_match` | Whole-sequence IUPAC check |

---

## 6. Core Runtime — Single-Cell Primitives

These are ground-work operations used by `scdata`, `singlecell`, and `spatial`.
They should live in the runtime so all three packages share the same implementation.

| Function | Notes |
|---|---|
| `normalize_total(matrix, target=10000)` | Total count normalisation |
| `log1p_transform(matrix)` | log(x + 1) — standard scRNA step |
| `highly_variable_genes(matrix, n=2000)` | HVG selection (mean/dispersion method) |
| `cell_qc(matrix, mito_prefix="MT-")` | Per-cell: n_genes, total_counts, pct_mito |
| `gene_qc(matrix)` | Per-gene: n_cells, mean_expression, pct_dropout |
| `knn_graph(embeddings, k=15)` | K-nearest-neighbour graph for clustering |
| `doublet_score(matrix)` | Basic scrublet-style doublet detection |

---

## 7. Core Runtime — Cancer-Specific Primitives

Broad enough to be shared across `variants`, `cnv`, `survival`, and `singlecell`
(tumour scRNA).

| Function | Notes |
|---|---|
| `cnv_segment(log_ratio, min_segment=5)` | Basic circular binary segmentation |
| `loh_detect(het_snp_vafs)` | Loss of heterozygosity from SNP VAFs |
| `tumor_purity(vaf_distribution)` | Estimate purity from VAF histogram |
| `clonal_analysis(vafs, cn_states)` | Clonal fraction estimation |
| `mutational_signature(mut_counts_96)` | Fit COSMIC SBS signatures |
| `vaf_to_ccf(vaf, purity, cn_total, cn_minor)` | VAF → cancer cell fraction |

---

## 8. Core Runtime — File Formats

Used by too many packages to belong in any one. These should be built-in builtins
alongside `fasta()`, `fastq()`, `vcf()`.

| Format | Function | Notes |
|---|---|---|
| 10x MEX | `read_10x_mtx(dir)` | barcodes + features + matrix.mtx.gz |
| AnnData | `read_h5ad(path)` / `write_h5ad(obj, path)` | De facto scRNA standard |
| Sparse MEX | `read_mtx(path)` | Generic sparse matrix exchange |
| BigWig | `read_bigwig(path, region?)` | Coverage — ChIP, ATAC, RNA-seq |
| PLINK | `read_plink(prefix)` | .bed/.bim/.fam — GWAS, pop genetics |
| MAF | `read_maf(path)` | TCGA cancer mutation format |
| BEDgraph | `write_bedgraph(intervals, path)` | Coverage output |
| GFF/GTF stream | `gff_stream(path)` | Lazy streaming for large annotations |

---

## 9. Language Features

### `@parallel` decorator — highest single impact

```biolang
@parallel(workers=8)
fn process_sample(sample_id) {
    load_counts(sample_id) |> normalize("tpm") |> qc_metrics()
}

# Auto-parallelises — no manual threading
let results = sample_ids |> map(process_sample)
```

Bulk RNA-seq, GWAS, bootstrapping, permutation tests all need per-sample parallelism.
Without this, every long pipeline requires external orchestration.

### Method syntax

```biolang
# Both equivalent — discoverability matters for scientists
seq.reverse_complement().gc_content()
gc_content(reverse_complement(seq))
```

### `@memoize` decorator

```biolang
@memoize(ttl=3600)
fn gene_annotations(ensembl_id) { fetch_gene(ensembl_id) }
# Repeated calls hit cache — critical for API-heavy pipelines
```

### `@vectorize` decorator

```biolang
@vectorize
fn gc_per_seq(seq) { gc_content(seq) }
# Auto-applies element-wise to a list
let gcs = gc_per_seq(sequence_list)
```

### Structured error kinds for `catch`

```biolang
try { fetch_ncbi(gene_id) }
catch NetworkError { use_local_cache(gene_id) }
catch NotFoundError { nil }
catch e { log_error(e); nil }
```

### Record spread update

```biolang
let updated_cell = { ...cell, cluster: "T-cell", annotation: "CD8+" }
```

### Named pipe binding

```biolang
counts
  |> normalize("tpm") into tpm_counts
  |> log1p()
  |> pca(50) into pca_result
  |> umap(2)
```

### `where` preconditions

```biolang
fn align_seqs(a, b) where seq_len(a) > 0 and seq_len(b) > 0 {
    align(a, b)
}
```

### Optional type annotations (documentation only, not enforced)

```biolang
fn gc_content(seq: DNA) -> Float { ... }
```

---

## 10. Visualisation (Core additions)

These are too general to belong in any domain package:

| Function | Notes |
|---|---|
| `volcano_plot(table, log2fc_col, pvalue_col)` | RNA-seq — used after every DE analysis |
| `heatmap(matrix, row_labels?, col_labels?)` | Gene expression, correlation matrices |
| `manhattan_plot(table, chrom, pos, pvalue)` | GWAS |
| `qq_plot(observed_pvalues)` | GWAS QC |
| `survival_curve(km_result)` | KM plot with CI ribbon |
| `violin_plot(table, group_col, value_col)` | Distribution comparison |
| `coverage_track(intervals, chrom_sizes)` | Genome browser-style track |
| `umap_plot(embeddings, color_by?)` | Scatter coloured by group |

---

## 11. Packages

### Must Have — language is incomplete without these

**`differential`**
DESeq2-style negative binomial testing for bulk and pseudo-bulk RNA-seq.
The single most-used analysis in molecular biology.
- Negative binomial GLM with Wald test
- Log2 fold-change shrinkage
- Pseudo-bulk aggregation for scRNA
- Tidy output: log2FC, p-value, FDR, baseMean
- Depends on: `normalize()`, `tpm()` (core)

**`scdata`**
AnnData-equivalent data structure for BioLang single-cell analysis.
Without this, every scRNA package re-invents the data container and they can't interoperate.
- Sparse count matrix (`.X`) + cell metadata (`.obs`) + gene metadata (`.var`)
- Embedding slots (`.obsm["X_pca"]`, `.obsm["X_umap"]`)
- Graph slots (`.obsp["connectivities"]`)
- Layer support (`.layers["counts"]`, `.layers["normalised"]`)
- Depends on: sparse matrix type (core), `cell_qc`, `gene_qc` (core)

**`clustering`**
Cell and sample clustering algorithms.
- k-means (fast baseline)
- Hierarchical (ward, complete, average linkage)
- Leiden / Louvain (graph-based — standard for scRNA)
- DBSCAN (outlier detection)
- Silhouette scoring, elbow method
- Depends on: `knn_graph()` (core), `scdata`

**`pathway`**
Gene set enrichment and over-representation analysis.
The default endpoint of every differential expression analysis.
- GSEA / fGSEA (pre-ranked)
- ORA / Fisher's exact (gene list)
- Bundled databases: GO (BP/MF/CC), KEGG, Reactome, MSigDB hallmarks
- Tidy output: pathway, NES, p-value, FDR, leading_edge
- Depends on: `differential`

**`variants`**
Variant annotation and cancer genomics pipeline.
- VCF functional consequence (coding, splice, UTR, intergenic)
- ClinVar pathogenicity lookup
- gnomAD allele frequency annotation
- Cancer hotspot flagging (OncoKB / cancerhotspots.org)
- COSMIC SBS mutational signature fitting
- Tumour mutational burden (TMB) calculation
- Depends on: `mutational_signature()` (core)

**`survival`**
Clinical survival analysis — essential for cancer, clinical trials, cohort studies.
- Kaplan-Meier estimator with 95% CI (Greenwood)
- Cox proportional hazards (univariate + multivariate)
- Competing risks (Fine-Gray)
- Log-rank test, C-index, time-dependent AUC
- Restricted mean survival time (RMST)
- Depends on: `kaplan_meier()`, `cox_ph()`, `log_rank_test()` (core)

**`oric`** ✅ *(already implemented)*
Origin of replication prediction for bacterial genomes.
- GC skew + DnaA box search
- Confidence scoring 0–5
- 24-enzyme restriction site lookup
- Depends on: `find_pattern()`, `gc_skew()` (core)

---

### Nice to Have — significantly expands addressable problem space

**`dimensionality`**
Non-linear dimensionality reduction for scRNA, proteomics, methylation.
- UMAP (primary method — fastest, best for scRNA)
- t-SNE (alternative visualisation)
- Diffusion maps (for trajectory analysis)
- PHATE (tree-like trajectories)
- Depends on: `pca()` (core), `scdata`

**`singlecell`**
High-level single-cell analysis workflow — the "Seurat / Scanpy for BioLang" package.
- QC → filter → normalise → HVG → PCA → KNN → cluster → markers → annotate
- Marker gene detection (Wilcoxon, logistic regression)
- Cell type annotation (marker-based, reference-based)
- Trajectory inference (PAGA-style)
- Multi-sample integration (Harmony-style batch correction)
- Doublet detection
- Depends on: `scdata`, `clustering`, `dimensionality`, `differential`

**`methylation`**
DNA methylation analysis — growing importance in cancer (CIMP, cfDNA, epigenetic clocks).
- WGBS / RRBS / EPIC array input parsing
- Per-CpG methylation calling
- CpG island / shore / shelf annotation
- DMR detection (Fisher's exact per-CpG, region smoothing)
- Global methylation levels by context (CpG, CHG, CHH)
- Epigenetic clock estimation (Horvath, Hannum)
- Depends on: `cpg_islands()` (core), `bed_*` (core)

**`phylo`**
Phylogenetic analysis beyond the built-in `neighbor_joining`.
- Maximum-likelihood tree inference (RAxML/IQ-TREE wrapper)
- Bootstrap support values
- Ancestral sequence reconstruction
- Phylogenetic signal tests (Pagel's lambda, Blomberg's K)
- Comparative methods (PGLS)
- iTOL-compatible export
- Depends on: `align()` (core), `consensus()` (core)

**`proteomics`**
Mass spectrometry proteomics workflows.
- MaxQuant / Spectronaut / DIA-NN output parsing
- LFQ, TMT, SILAC quantification
- Peptide → protein rollup (MaxLFQ)
- Protein-level normalisation and DE
- Volcano plots, heatmaps
- String DB / BioGRID network enrichment
- Depends on: `differential`, `pathway`

**`spatial`**
Spatial transcriptomics — Visium, Slide-seq, MERFISH, Xenium.
- Spatial coordinate handling
- Spatially variable gene detection (Moran's I)
- Spatial neighbourhood graph
- Cell-cell communication with spatial constraint
- H&E image co-registration hooks
- Depends on: `scdata`, `clustering`, `knn_graph()` (core)

**`gwas`**
Genome-wide association study analysis.
- PLINK binary format (.bed/.bim/.fam) reading
- Linear / logistic association testing
- Population stratification (PCA-based)
- Manhattan plot, QQ plot
- LD-based fine-mapping (conditional analysis)
- LD score regression (heritability, genetic correlation)
- Depends on: `pca()` (core), `ld_decay()` (core), `fst()` (core)

---

### Good to Have — valuable niche, clear demand

**`metagenomics`**
16S/ITS amplicon + shotgun metagenomics.
- 16S rRNA amplicon OTU/ASV table operations
- Alpha diversity: Shannon, Simpson, Chao1, ACE
- Beta diversity: Bray-Curtis, UniFrac (weighted/unweighted)
- PERMANOVA for group comparison
- Taxonomic barplots at any rank
- Decontam for contaminant removal
- Depends on: `phylo`

**`chipseq` / `atacseq`**
ChIP-seq and ATAC-seq analysis.
- Peak calling interface (MACS3 wrapper)
- IDR (irreproducible discovery rate)
- Motif enrichment (HOMER-style)
- TSS enrichment score
- FRiP (fraction of reads in peaks) calculation
- Signal normalisation and bigWig generation
- Depends on: `bed_*` (core), `find_pattern` (core)

**`cnv`**
Somatic copy number variant detection from WGS/WES/SNP array.
- Log-ratio calculation and GC correction
- Circular binary segmentation (CBS)
- Arm-level and focal CNA calls
- Allele-specific CN from SNP B-allele frequencies
- Tumour purity and ploidy estimation
- ABSOLUTE / ASCAT-style analysis
- Depends on: `cnv_segment()`, `tumor_purity()` (core)

**`network`**
Biological network analysis.
- PPI network loading (String DB, BioGRID)
- Gene regulatory network inference (GENIE3-style)
- PageRank, betweenness centrality, hub detection
- Module detection (MCL, Leiden on weighted graph)
- Network-based enrichment
- Drug target prioritisation
- Depends on: `pathway`, graph algorithms (core `de_bruijn_graph` → generalise)

**`drug`**
Cheminformatics and drug analysis.
- Molecular fingerprints (ECFP4, MACCS keys)
- Tanimoto similarity matrix
- ADMET property prediction (basic rules-based)
- Drug-target affinity estimation
- Synergy scoring (Bliss, HSA, ZIP)
- FDA approval status lookup

**`imaging`**
Computational pathology / spatial transcriptomics image analysis.
- H&E patch extraction and tiling
- Cell segmentation hooks (StarDist, Cellpose interface)
- Morphological features per cell
- Tissue region annotation
- Co-registration with spatial transcriptomics coordinates
- Depends on: `spatial`, `scdata`

**`benchmarking`**
ML evaluation utilities — small but universally applicable.
- ROC curve + AUC
- Precision-recall curve + AUPRC
- Calibration curves (reliability diagrams)
- k-fold and stratified cross-validation
- Confusion matrix with per-class metrics
- Concordance index (C-index) for survival models

**`longread`**
Oxford Nanopore / PacBio specific analysis.
- Raw signal → basecall interface (Dorado/Guppy wrapper)
- Alignment quality metrics (identity, coverage)
- Structural variant detection and phasing
- Isoform-level quantification (FLAMES / IsoQuant interface)
- Modified base (5mC, 5hmC) detection
- Depends on: `align()` (core)

---

## 12. Implementation Priority Order

If forced to sequence, highest leverage first:

1. `inner_join` + `left_join` in core — unblocks the most immediate workflows
2. `pca()` in core — needed before `dimensionality` package can be useful
3. `kaplan_meier()` + `cox_ph()` in core — cancer without survival is half-finished
4. Sparse matrix type in core — `scdata` can't be built cleanly without it
5. **`differential`** package — most-used tool in molecular biology
6. **`scdata`** package — data structure that everything single-cell depends on
7. **`pathway`** package — default endpoint of every DE analysis
8. **`survival`** package — wraps core KM/Cox into clinical-grade workflows
9. `@parallel` decorator — biggest quality-of-life improvement for long pipelines
10. `read_h5ad` / `write_h5ad` in core — scRNA interoperability with Python ecosystem
11. **`clustering`** package — Leiden clustering is the standard scRNA method
12. `pivot_wider` / `pivot_longer` in core — avoids verbose reshape workarounds
13. `batch_correct()` in core — every multi-sample study needs this
14. **`gwas`** package — large population genetics user base
15. **`metagenomics`** package — microbiome research is growing fast

---

## 13. Cross-Cutting Design Decisions

### Method syntax vs function syntax
Pick ONE pattern and be consistent across all documentation:
- Option A: function-first — `gc_content(seq)` everywhere
- Option B: method-first — `seq.gc_content()` everywhere (better discoverability)
- **Recommendation**: Support both; document method-first in tutorials

### Named arguments
Standardise on options record OR keyword arguments:
```biolang
# Option A: options record
align(a, b, {mode: "local", gap: -2})

# Option B: keyword args (more common in scripting languages)
align(a, b, mode="local", gap=-2)
```
Current docs mix both — pick one and audit all examples.

### Error taxonomy
All runtime errors should have machine-readable kinds for `catch` dispatch:
```
NetworkError, NotFoundError, ParseError, TypeError,
IndexOutOfBounds, IOError, TimeoutError, AuthError
```

### Package registry
- Start with GitHub-based (like Cargo's crates.io model)
- `biolang.toml` already in place for `oric` package
- `bl add <package>` CLI command needed
- Semantic versioning + lock file from the start

---

*Last updated: 2026-07-23*
*Source: BioLang design review session — core features, package ecosystem, implementation priorities*
