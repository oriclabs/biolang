# Omics Packages

The BioLang source tree includes 37 importable packages spanning common omics
and bioinformatics domains. Each package provides a curated, pipe-friendly API.
Most operations use BioLang's native runtime; individual workflows may still
call files, services, or external tools documented by that package.

Install a local package with `bl install`, or declare it in `biolang.toml`.
Packages declaring `[lib].entry` can be imported by package name; other
packages use their explicit `src/mod` entry:

```bash
bl install packages/variants
```

```biolang
import "singlecell"       as sc
import "variants" as vcf
import "rnaseq"   as rna
```

For a source checkout, set `BIOLANG_PATH` to the repository's `packages`
directory before running examples. Installed packages are discovered under
`~/.biolang/packages/`. Use `bl examples <package> --copy <directory>` to make
an independent working copy of examples bundled with an installed package.

## Current Package Catalog

Every package exposes code from `src/mod.bl`; packages with `[lib].entry` also
support the short package-name import. The table is the complete catalog in the
current source tree:

| Package | Primary use | Package | Primary use |
|---|---|---|---|
| `annotation` | GTF/GFF parsing and feature annotation | `atac` | ATAC-seq QC, peaks, and accessibility |
| `cellchat` | Ligand-receptor communication scoring | `celltypes` | Marker-based cell-type annotation |
| `chipseq` | ChIP-seq peaks, FRiP, and consensus sets | `clustering` | General clustering workflows |
| `cnv` | Copy-number segments and gene-level calls | `crispr` | CRISPR screen QC and hit ranking |
| `deconvolution` | Bulk-mixture cell proportion estimates | `differential` | Differential expression testing |
| `drug` | Dose-response curves and synergy | `grn` | Gene-regulatory network inference |
| `gwas` | Summary statistics, loci, and inflation | `hic` | Hi-C matrices and contact analysis |
| `immune` | Immune signatures and repertoire summaries | `longread` | Long-read QC and length summaries |
| `metabolomics` | Feature matrices and metabolite analysis | `methylation` | CpG and region methylation analysis |
| `microbiome` | Taxonomic abundance and diversity | `motif` | Motif scans and enrichment |
| `multimodal` | Cross-modality integration | `network` | Biological graph analysis |
| `oric` | Bacterial origin-of-replication prediction | `pathway` | Pathway and gene-set enrichment |
| `phylo` | Newick trees and phylogenetic distances | `popgen` | Population-genetics statistics |
| `proteomics` | Protein abundance and differential analysis | `qpcr` | Ct, delta-Ct, and delta-delta-Ct |
| `rnaseq` | Bulk RNA-seq count processing | `scref` | Single-cell reference mapping |
| `singlecell` | Single-cell QC, normalization, and clustering | `spatial` | Spatial transcriptomics analysis |
| `statistics` | Reusable statistical helpers | `structure` | PDB and protein-structure analysis |
| `survival` | Kaplan-Meier and survival workflows | `variants` | VCF filtering and variant summaries |
| `velocity` | RNA-velocity workflows |  |  |

Use `bl metadata --format json` for core builtin signatures. Package APIs live
in each package's `src/mod.bl`, which is the authoritative public surface.

---

## 1. Variant Analysis

The `variants` package wraps VCF parsing, filtering, and quality metrics.
Use it for germline QC pipelines or to feed variant tables into downstream
population genetics analysis.

```biolang
import "variants" as vcf

let raw  = vcf.load("results/calls.vcf")
vcf.qc_report(raw)

let pass = vcf.load_filtered("results/calls.vcf", 30.0, true)
vcf.summary(pass)
vcf.titv(pass)

# Rare variant analysis (AF < 0.1%)
let rare = vcf.rare(pass, 0.001)
vcf.by_chrom(rare, "chr17")
```

Key functions: `load`, `load_filtered`, `summary`, `titv`, `rare`, `by_chrom`, `qc_report`

---

## 2. Bulk RNA-seq

The `rnaseq` package loads Salmon and featureCounts output, normalizes counts,
and prepares matrices for differential expression analysis with the `differential` package.

```biolang
import "rnaseq"       as rna
import "differential" as de

let counts = rna.from_featurecounts("results/counts.txt")
rna.library_sizes(counts)
rna.correlation(counts)

let counts = rna.filter_genes(counts, 10, 3)
let norm   = rna.normalize_sf(counts)

let conditions = ["ctrl", "ctrl", "ctrl", "treated", "treated", "treated"]
let results = de.de_wald(norm, [0, 1, 2], [3, 4, 5])
results |> filter(|r| r.padj <= 0.05 && abs(r.log2fc) >= 1.0)
```

Key functions: `from_salmon`, `from_featurecounts`, `filter_genes`, `normalize_sf`, `library_sizes`, `correlation`

---

## Single-cell RNA-seq

The `singlecell` package provides a sparse 10x workflow with synchronized cell
and gene metadata. Filtering, normalization, variable-gene selection, PCA,
neighbor-graph construction, and Leiden clustering can run without converting
the expression matrix to a dense cells-by-genes array.

```biolang
import "singlecell" as sc

let cells = sc.load("filtered_feature_bc_matrix")
let result = cells
    |> sc.filter_genes(3)
    |> sc.filter_cells(200, 5000, 20.0)
    |> sc.normalize(10000.0)
    |> sc.variable_genes(2000)
    |> sc.run_pca(30)
    |> sc.neighbors(15)
    |> sc.cluster_leiden(15, 0.5)

println(sc.summary(result))
write_text("umap.svg", sc.plot_umap(result))
```

Use `sc.merge` before integration when samples share the same gene order.
`sc.integrate` corrects the PCA embedding and records batch labels. Native
AnnData Zarr exchange preserves CSR `X` matrices and observation/variable
index names:

```biolang
write_anndata("analysis.zarr", result)
let restored = read_anndata("analysis.zarr")
```

Direct `.h5ad` I/O requires conversion with Python/anndata or a configured
container. Arbitrary AnnData metadata columns and auxiliary layers are not yet
copied by the native Zarr interchange. Validation scripts for Scanpy and Seurat live in the `biolang-workflows`
repository under `validation/single-cell`; compare cluster partitions by barcode using
ARI rather than comparing arbitrary numeric cluster IDs.

Key functions: `load`, `filter_genes`, `filter_cells`, `normalize`,
`variable_genes`, `run_pca`, `neighbors`, `cluster_leiden`, `merge`, `integrate`

---

## 3. Phylogenetics

The `phylo` package reads Newick trees, computes patristic distances, and
reconstructs trees with UPGMA — useful for viral phylodynamics and species
comparisons.

```biolang
import "phylo" as ph

let tree = ph.load("data/sarscov2_sequences.nwk")
ph.n_leaves(tree)
ph.leaves(tree)

ph.distance(tree, "Alpha", "Delta")

let dmat    = ph.distance_matrix(tree)
let rebuilt = ph.build_upgma(ph.leaves(tree), dmat)
ph.faith_pd(tree)
```

Key functions: `load`, `n_leaves`, `leaves`, `distance`, `distance_matrix`, `build_upgma`, `faith_pd`

---

## 4. ChIP-seq & ATAC-seq

The `chipseq` package processes NarrowPeak files from MACS3, computes FRiP
and TSS enrichment QC metrics, and builds consensus peak sets across replicates.

```biolang
import "chipseq" as cs

let ctrl    = cs.load_narrowpeak("results/ctrl_peaks.narrowPeak")
let treated = cs.load_narrowpeak("results/treated_peaks.narrowPeak")

cs.qc_report(ctrl, 320000, 25000000)
cs.qc_report(treated, 280000, 22000000)

let consensus = cs.consensus([cs.merge(ctrl), cs.merge(treated)], 2)
cs.mean_peak_size(consensus)
cs.n_peaks(consensus)
```

Key functions: `load_narrowpeak`, `merge`, `consensus`, `qc_report`, `n_peaks`, `mean_peak_size`

---

## 5. Microbiome

The `microbiome` package computes alpha and beta diversity indices, performs
rarefaction, and collapses OTU tables to arbitrary taxonomic ranks.

```biolang
import "microbiome" as mb

let otus     = read_csv("data/feature_table.tsv")
let taxonomy = read_csv("data/taxonomy.tsv")

mb.alpha_table(otus, "shannon")
mb.alpha_table(otus, "chao1")

let rarefied = mb.rarefy_matrix(otus, 5000)
mb.beta(rarefied, "bray_curtis")

mb.top_taxa(taxonomy, 15, "genus")
mb.top_taxa(taxonomy, 10, "phylum")
```

Key functions: `alpha_table`, `beta`, `rarefy_matrix`, `top_taxa`, `relative_abundance`

---

## 6. Statistics

The `statistics` package extends the core statistical builtins with
multiple-testing correction, permutation tests, bootstrap confidence
intervals, and genomic inflation factors for GWAS.

```biolang
import "statistics" as stat

let p_values = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 0.9]

let adj_bh   = stat.bh(p_values)
let adj_bonf = stat.bonferroni(p_values)
stat.significant(adj_bh, 0.05)
stat.inflation(p_values)

let group_a = [2.1, 2.3, 1.9, 2.5, 2.2]
let group_b = [3.1, 2.8, 3.4, 2.9, 3.2]
stat.permtest(group_a, group_b, 10000)
stat.bootstrap(group_a, 5000, 0.95)
```

Key functions: `bh`, `bonferroni`, `significant`, `inflation`, `fisher`, `chi2`, `permtest`, `bootstrap`, `pearson`

---

## 7. RT-qPCR

The `qpcr` package implements the ΔCt and ΔΔCt method, standard-curve
efficiency calculation, and geNorm stability scoring for reference gene
selection.

```biolang
import "qpcr" as qpcr

# Standard curve → amplification efficiency
let standard_cts  = [15.2, 18.5, 21.8, 25.1, 28.4]
let log_dilutions = [0.0, -1.0, -2.0, -3.0, -4.0]
qpcr.efficiency(standard_cts, log_dilutions)

# ΔCt and ΔΔCt fold change
let dct         = qpcr.dct(28.5, 22.0)
let fold_change = qpcr.ddct(dct, 6.2)

# Plate-level normalization and stability scoring
# let norm = qpcr.normalize(ct_table, ref_indices=[0, 1])
# qpcr.stability(ct_table, ref_indices=[0, 1])
```

Key functions: `dct`, `ddct`, `efficiency`, `normalize`, `stability`, `best_reference`

---

## 8. Proteomics

The `proteomics` package processes MaxQuant label-free quantification (LFQ)
output through the full preprocessing pipeline and identifies differentially
abundant proteins.

```biolang
import "proteomics" as prot

# Load proteinGroups.txt → log2 → quantile normalize → impute
let mat = prot.full_pipeline("data/proteinGroups.txt")

# Differential abundance: samples 0-2 vs 3-5
let de   = prot.ttest(mat, [0, 1, 2], [3, 4, 5])
let hits = prot.significant(de, 1.0, 0.05)

# Prepare volcano plot data
prot.volcano(de, 1.0, 0.05)
```

Key functions: `load`, `log2_transform`, `quantile_normalize`, `impute`, `preprocess`, `full_pipeline`, `ttest`, `significant`, `volcano`

---

## 9. DNA Methylation

The `methylation` package handles RRBS and EPIC array data: beta/M-value
conversion, differentially methylated region (DMR) calling, CpG density, and
epigenetic clock prediction.

```biolang
import "methylation" as meth

# beta_matrix: CpGs × samples
# let m_matrix = beta_matrix |> rows |> map(fn(r) -> meth.to_mvalue(r))

# Differential methylation (cases vs controls)
# let dmrs = meth.find_dmrs(beta_matrix, [0,1,2,3,4], [5,6,7,8,9])
# let diff = meth.diff_cpgs(beta_matrix, [0,1,2], [3,4,5])
# meth.significant_cpgs(diff, 0.15, 0.05)

# Epigenetic age (Horvath clock)
# meth.clock_age(beta_vec_353cpgs, horvath_coefs)

# CpG density in a window
let positions = [100, 120, 145, 300, 320, 340, 360]
meth.density(positions, 200)
```

Key functions: `to_mvalue`, `to_beta`, `find_dmrs`, `diff_cpgs`, `significant_cpgs`, `density`, `clock_age`

---

## 10. Protein Structure

The `structure` package parses PDB files, computes Kabsch-aligned RMSD,
builds residue contact maps, assigns secondary structure (DSSP-lite), and
extracts Ramachandran phi/psi angles.

```biolang
import "structure" as st

let pred    = st.load("data/alphafold_prediction.pdb")
let exp     = st.load("data/experimental.pdb")

let pred_ca = st.ca_atoms(pred)
let exp_ca  = st.ca_atoms(exp)

st.rmsd(pred_ca, exp_ca)
st.ss_composition(pred_ca)
st.contacts(pred_ca, 8.0)
st.ramachandran(st.backbone(pred))
```

Key functions: `load`, `ca_atoms`, `backbone`, `chain`, `residues`, `rmsd`, `contacts`, `ss`, `ss_composition`, `ramachandran`

---

## 11. Biological Networks

The `network` package loads STRING or custom PPI edge lists and computes
degree centrality, Brandes betweenness, BFS shortest paths, connected
components, and hypergeometric disease module enrichment.

```biolang
import "network" as net

let ppi = net.load_string("data/9606.protein.links.v12.0.txt")
  |> net.filter_score(0.7)

net.hub_genes(ppi, 20)
net.components(ppi)

let lcc        = net.largest_component(ppi)
let brca_genes = ["TP53", "BRCA1", "BRCA2", "ATM", "CHEK2", "PALB2"]
let disease_net = net.subnetwork(lcc, brca_genes)

net.degree(disease_net)
net.path(lcc, "TP53", "BRCA1")
net.enrichment(brca_genes, ppi, 20000)
```

Key functions: `load_string`, `load_csv`, `filter_score`, `subnetwork`, `degree`, `betweenness`, `path`, `components`, `hub_genes`, `largest_component`, `enrichment`

---

## 12. Population Genetics

The `popgen` package implements core population genetics statistics: HWE
tests, Weir-Cockerham Fst, Tajima's D, LD r², allele frequency spectrum, and
nucleotide diversity — all computed directly from genotype matrices.

```biolang
import "popgen" as pg

# Hardy-Weinberg equilibrium test
pg.hwe(360, 480, 160)

# Fst between two populations
let ac_eur = [45, 12, 89, 3, 67]
let ac_afr = [78,  5, 92, 8, 34]
pg.fst(ac_eur, ac_afr, 200, 200)

# Tajima's D neutrality test
pg.tajima(15, 20, 1000)

# LD between two variants
let geno_a = [0, 1, 1, 2, 0, 1, 2, 0]
let geno_b = [0, 1, 1, 2, 0, 1, 1, 0]
pg.ld(geno_a, geno_b)
```

Key functions: `hwe`, `fst`, `tajima`, `ld`, `ld_matrix`, `afs`, `pi`, `watterson_theta`

---

## cnv

The `cnv` package implements copy number variation analysis from whole-genome or whole-exome sequencing read-depth data. Use it to call amplifications and deletions in tumor samples, compute allele-specific copy numbers from B-allele frequencies, and summarise genome-wide CNV burden.

```biolang
import "cnv" as cnv
```

```biolang
import "cnv" as cnv

# Per-bin log2 ratios from mosdepth output
# let ratios   = cnv.ratios(tumor_depths, normal_depths)

# Circular binary segmentation
# let segments = cnv.segment(ratios)

# Integer CN calls (diploid reference)
# cnv.call(segments, ploidy=2)

# Allele-specific CN from SNP heterozygotes
# let baf = read_csv("tumor_snps.tsv") |> col("baf")
# cnv.allelic(baf, ratios)

# Genome-wide summary
# cnv.summary(segments)
```

**Key functions:** `ratios`, `segment`, `call`, `allelic`, `summary`

---

## hic

The `hic` package provides tools for Hi-C chromatin conformation capture data: ICE normalization of raw contact matrices, insulation score computation for TAD detection, and distance-decay analysis. Use it any time you need to identify topologically associating domains or inspect 3D genome organisation.

```biolang
import "hic" as hic
```

```biolang
import "hic" as hic

# contact_matrix: N×N Table (genomic bins × bins, values = contact counts)

# ICE normalization
# let norm   = hic.normalize(contact_matrix)

# Insulation score (window = 10 bins)
# let scores = hic.insulation(norm, 10)

# TAD boundaries from insulation score minima
# let tads   = hic.boundaries(scores, delta=0.15)

# Distance decay: average contacts vs genomic separation
# hic.decay(norm)
```

**Key functions:** `normalize`, `insulation`, `boundaries`, `decay`, `expected`

---

## atac

The `atac` package provides ATAC-seq quality control metrics derived from fragment size distributions. Use it to assess chromatin accessibility library quality — NFR enrichment, nucleosome phasing, and TSS enrichment — before proceeding to peak calling.

```biolang
import "atac" as atac
```

```biolang
import "atac" as atac

# Fragment lengths from filtered BAM
# let frag_lengths = read_csv("fragments.tsv") |> col("tlen") |> map(fn(x) -> abs(x))

# Full QC report
# atac.qc(frag_lengths)

# Fragment size histogram
# atac.sizes(frag_lengths)

# NFR enrichment score (> 1.5 = good quality)
# atac.nfr(frag_lengths)

# Nucleosome fraction breakdown (NFR / mono / di / tri)
# atac.nucleosomes(frag_lengths)

# TSS enrichment (requires per-fragment TSS distances)
# atac.tss(frag_lengths, tss_distances, flank=2000)
```

**Key functions:** `qc`, `nfr`, `sizes`, `nucleosomes`, `tss`

---

## drug

The `drug` package fits dose-response curves and computes drug combination synergy scores. Use it for GDSC/PRISM-style pharmacogenomics analysis, IC50 estimation from viability data, and Bliss or Loewe synergy scoring for combination screens.

```biolang
import "drug" as drug
```

```biolang
import "drug" as drug

let concentrations = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0]
let viabilities    = [98.0, 95.0, 88.0, 70.0, 45.0, 20.0, 8.0, 3.0]

# Fit 4-parameter logistic → {ic50, slope, top, bottom, r2}
let params = drug.fit(concentrations, viabilities)

# Area under dose-response curve (lower = more sensitive)
drug.auc(concentrations, viabilities)

# Combination synergy
drug.bliss(75.0, 60.0, 30.0)
drug.loewe(1.0, 2.0, 0.5, 1.0)
```

**Key functions:** `fit`, `curve`, `auc`, `bliss`, `loewe`, `rank`

---

## gwas

The `gwas` package handles GWAS summary statistics: flexible column-name auto-detection, Manhattan and QQ plot data preparation, genomic inflation, and distance-based locus clumping. Use it to process output from PLINK, BOLT-LMM, SAIGE, or any standard GWAS tool.

```biolang
import "gwas" as gwas
```

```biolang
import "gwas" as gwas

# Auto-detect column names (CHR/BP/SNP/P/BETA/SE/A1/A2/MAF)
let ss = gwas.load("data/my_gwas_sumstats.txt")

# Genomic inflation factor (should be ~1.0)
gwas.lambda(ss.pval)

# Genome-wide significant hits (p < 5×10⁻⁸)
let hits = gwas.hits(ss)

# Distance-based LD clumping → independent loci
let loci = gwas.clump(ss, 5e-8, 250)

# Data for Manhattan and QQ plots
gwas.manhattan(ss)
gwas.qq(ss.pval)
```

**Key functions:** `load`, `hits`, `clump`, `lambda`, `manhattan`, `qq`

---

## annotation

The `annotation` package parses GTF/GFF3 gene annotation files and performs genomic interval operations. Use it to annotate ChIP-seq or ATAC-seq peaks with nearest genes, extract promoter regions, compute interval overlaps, and build Ensembl-to-gene-name lookup tables.

```biolang
import "annotation" as ann
```

```biolang
import "annotation" as ann

# Load GENCODE or Ensembl GTF (auto-detects GTF vs GFF3)
let gtf   = ann.load("data/gencode.v45.annotation.gtf")

# Gene bodies and promoter windows
let genes = ann.genes(gtf)
let proms = ann.promoters(gtf, 2000, 200)

# Ensembl ID → gene name lookup
let idmap = ann.id_map(gtf)

# Annotate peaks with nearest gene + feature type
# let peaks = read_csv("data/macs3_peaks.narrowPeak")
# ann.annotate(peaks, gtf)

# Overlap ChIP-seq peaks with ATAC-seq peaks
# let atac_peaks = read_csv("data/atac_peaks.bed")
# ann.overlap(peaks, atac_peaks)
```

**Key functions:** `load`, `genes`, `promoters`, `id_map`, `annotate`, `overlap`
