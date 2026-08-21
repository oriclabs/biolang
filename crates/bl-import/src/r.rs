/// R / Bioconductor → BioLang converter.
/// R already uses braces for blocks, so no indent-tracking needed.
/// Main transformations: library() → imports, <- → let, syntax normalization.
use regex::{Captures, Regex};
use std::sync::LazyLock;

struct CallMap {
    pattern: Regex,
    replacement: &'static str,
}

impl CallMap {
    fn new(pat: &str, repl: &'static str) -> Self {
        Self {
            pattern: Regex::new(pat).unwrap_or_else(|e| panic!("bad regex '{pat}': {e}")),
            replacement: repl,
        }
    }

    fn apply(&self, line: &str) -> String {
        self.pattern
            .replace_all(line, |caps: &Captures| {
                let mut out = String::from(self.replacement);
                for i in 1..=caps.len().saturating_sub(1) {
                    out = out.replace(&format!("${i}"), caps.get(i).map_or("", |m| m.as_str()));
                }
                out
            })
            .into_owned()
    }
}

fn build_call_maps() -> Vec<CallMap> {
    vec![
        // ── Biostrings ────────────────────────────────────────────
        CallMap::new(r"\bDNAString\(([^)]+?)\)", "dna($1)"),
        CallMap::new(r"\bRNAString\(([^)]+?)\)", "rna($1)"),
        CallMap::new(r"\bAAString\(([^)]+?)\)", "protein($1)"),
        CallMap::new(r"\bDNAStringSet\(([^)]+?)\)", "$1 |> map(dna)"),
        CallMap::new(r"\bRNAStringSet\(([^)]+?)\)", "$1 |> map(rna)"),
        CallMap::new(r"\bAAStringSet\(([^)]+?)\)", "$1 |> map(protein)"),
        CallMap::new(r"\breverseComplement\(([^)]+?)\)", "reverse_complement($1)"),
        CallMap::new(r"\bcomplement\(([^)]+?)\)", "complement($1)"),
        CallMap::new(r"\btranslate\(([^)]+?)\)", "translate($1)"),
        CallMap::new(
            r#"\bletterFrequency\(([^,)]+),\s*['"]GC['"]\)"#,
            "gc_content($1) * nchar($1)  # letterFrequency GC",
        ),
        CallMap::new(
            r#"\bletterFrequency\(([^,)]+),\s*['"](.)['"]\)"#,
            r#"count_char($1, "$2")"#,
        ),
        CallMap::new(
            r"\bmatchPattern\(([^,)]+),\s*([^)]+?)\)",
            "find_pattern($2, $1)",
        ),
        CallMap::new(
            r"\bvmatchPattern\(([^,)]+),\s*([^)]+?)\)",
            "find_pattern($2, $1)",
        ),
        CallMap::new(
            r"\bsubseq\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "subseq($1, $2, $3)",
        ),
        CallMap::new(r"\bnchar\(([^)]+?)\)", "len($1)"),
        CallMap::new(r"\bwidth\(([^)]+?)\)", "len($1)"),
        // ── GenomicRanges ─────────────────────────────────────────
        CallMap::new(
            r"\bGRanges\(\s*seqnames\s*=\s*([^,)]+),\s*ranges\s*=\s*IRanges\(([^,)]+),\s*([^)]+?)\)[^)]*\)",
            "interval($1, $2, $3)",
        ),
        CallMap::new(
            r"\bIRanges\(([^,)]+),\s*([^)]+?)\)",
            "interval(\".\", $1, $2)",
        ),
        CallMap::new(
            r"\bfindOverlaps\(([^,)]+),\s*([^)]+?)\)",
            "bed_intersect($1, $2)",
        ),
        CallMap::new(
            r"\bsubsetByOverlaps\(([^,)]+),\s*([^)]+?)\)",
            "bed_intersect($1, $2)",
        ),
        CallMap::new(
            r"\bcountOverlaps\(([^,)]+),\s*([^)]+?)\)",
            "bed_count_overlaps($1, $2)",
        ),
        CallMap::new(r"\bflank\(([^,)]+),\s*([^)]+?)\)", "flank($1, $2)"),
        CallMap::new(r"\bresize\(([^,)]+),\s*([^)]+?)\)", "resize($1, $2)"),
        CallMap::new(r"\bstart\(([^)]+?)\)", "$1.start"),
        CallMap::new(r"\bend\(([^)]+?)\)", "$1.end"),
        CallMap::new(r"\bwidth\(([^)]+?)\)", "$1.end - $1.start"),
        CallMap::new(r"\bseqnames\(([^)]+?)\)", "$1.chrom"),
        // BioLang tables are already data-frame-like; just unwrap these wrappers
        CallMap::new(r"\bas\.data\.frame\(([^)]+?)\)", "$1"),
        CallMap::new(r"\bmakeGRangesFromDataFrame\(([^)]+?)\)", "$1"),
        // ── DESeq2 ────────────────────────────────────────────────
        // produces an expression so it can be assigned: let dds = de.setup(...)
        CallMap::new(
            r"\bDESeqDataSetFromMatrix\(\s*countData\s*=\s*([^,)]+),\s*colData\s*=\s*([^,)]+),\s*design\s*=\s*[^)]+?\)",
            "de.setup($1, $2)  # TODO: specify design formula",
        ),
        CallMap::new(
            r"\bDESeq\(([^)]+?)\)",
            "de.run($1)  # TODO: pass counts + conditions",
        ),
        // contrast pattern MUST come before simple pattern — contrast is more specific
        // replacements use .hits not de.results() to avoid re-matching on \bresults\(
        CallMap::new(
            r"\bresults\(([^,)]+),\s*contrast\s*=\s*([^)]+?)\)",
            "$1.hits  # DESeq2 contrast=$2",
        ),
        CallMap::new(r"\bresults\(([^)]+?)\)", "$1.hits  # DESeq2 results table"),
        CallMap::new(
            r"\bvst\(([^)]+?)\)",
            r#"normalize_counts($1, method="vst")"#,
        ),
        CallMap::new(
            r"\brlog\(([^)]+?)\)",
            r#"normalize_counts($1, method="rlog")"#,
        ),
        CallMap::new(
            r"\bplotPCA\(([^)]+?)\)",
            "scatter(pca($1), color_by=\"condition\")",
        ),
        CallMap::new(
            r"\bnormalized_counts\(([^)]+?)\)",
            r#"normalize_counts($1, method="deseq2")"#,
        ),
        CallMap::new(
            r"\bestimateSizeFactors\(([^)]+?)\)",
            "$1  # size factors computed in de.run()",
        ),
        CallMap::new(
            r"\bplotMA\(([^)]+?)\)",
            "# TODO: MA plot — plot log2FC vs mean expression",
        ),
        CallMap::new(
            r"\blfcShrink\(([^)]+?)\)",
            "$1  # TODO: LFC shrinkage (apeglm) not yet in BioLang",
        ),
        // ── edgeR ─────────────────────────────────────────────────
        CallMap::new(
            r"\bDGEList\(([^)]+?)\)",
            "# TODO: edgeR not yet in BioLang — coming soon\n# $1",
        ),
        CallMap::new(
            r"\bcalcNormFactors\(([^)]+?)\)",
            "normalize_counts($1, method=\"tmm\")  # TODO: edgeR TMM",
        ),
        CallMap::new(
            r"\bglmQLFit\(([^)]+?)\)",
            "# TODO: edgeR glmQLFit not yet in BioLang",
        ),
        CallMap::new(
            r"\bglmQLFTest\(([^)]+?)\)",
            "# TODO: edgeR glmQLFTest not yet in BioLang",
        ),
        // ── limma ─────────────────────────────────────────────────
        CallMap::new(
            r"\bvoom\(([^)]+?)\)",
            "# TODO: limma::voom not yet in BioLang",
        ),
        CallMap::new(
            r"\blmFit\(([^)]+?)\)",
            "# TODO: limma::lmFit not yet in BioLang",
        ),
        CallMap::new(
            r"\beBayes\(([^)]+?)\)",
            "# TODO: limma::eBayes not yet in BioLang",
        ),
        CallMap::new(
            r"\btopTable\(([^)]+?)\)",
            "# TODO: limma::topTable not yet in BioLang",
        ),
        // ── survival ─────────────────────────────────────────────
        // survfit/coxph patterns MUST come before standalone Surv() to avoid
        // consuming the Surv() inner expression before the outer call is matched
        CallMap::new(
            r"\bsurvfit\(Surv\(([^,)]+),\s*([^)]+?)\)\s*~\s*(\w+)[^)]*\)",
            "km.fit($1, $2, group=$3)",
        ),
        CallMap::new(
            r"\bcoxph\(Surv\(([^,)]+),\s*([^)]+?)\)\s*~\s*([^,)]+)[^)]*\)",
            "cox.fit($1, $2, $3)",
        ),
        // survdiff(Surv(...)) must also come before standalone Surv()
        CallMap::new(
            r"\bsurvdiff\(Surv\(([^,)]+),\s*([^)]+?)\)\s*~\s*([^,)]+)[^)]*\)",
            "log_rank_test($1, $2, group=$3)",
        ),
        CallMap::new(
            r"\bSurv\(([^,)]+),\s*([^)]+?)\)",
            "survival_pair($1, $2)",
        ),
        // fallback for bare survdiff without Surv() inside
        CallMap::new(r"\bsurvdiff\(([^)]+?)\)", "log_rank_test($1)"),
        CallMap::new(r"\bplot\.survfit\(([^)]+?)\)", "km.plot($1)"),
        // ── clusterProfiler / GSEA ────────────────────────────────
        CallMap::new(
            r"\benrichGO\(([^)]+?)\)",
            "ora.run($1)  # TODO: map args to ora.run()",
        ),
        CallMap::new(
            r"\benrichKEGG\(([^)]+?)\)",
            "kegg_pathway($1)  # TODO: map KEGG args",
        ),
        CallMap::new(
            r"\bgseGO\(([^)]+?)\)",
            "gsea.run($1)  # TODO: map args to gsea.run()",
        ),
        CallMap::new(
            r"\bgseKEGG\(([^)]+?)\)",
            "gsea.run($1)  # TODO: map KEGG GSEA args",
        ),
        CallMap::new(
            r"\bdotplot\(([^)]+?)\)",
            "# TODO: clusterProfiler dotplot → use scatter()",
        ),
        CallMap::new(r"\bbubbleplot\(([^)]+?)\)", "scatter($1)"),
        // ── Seurat: I/O & object construction ────────────────────
        // Read10X/Read10X_h5 — 10x Genomics count matrices
        CallMap::new(
            r"\bRead10X\(([^,)]+)[^)]*\)",
            "read_10x($1)  # TODO: returns sparse matrix; pass to sc.setup()",
        ),
        CallMap::new(
            r"\bRead10X_h5\(([^,)]+)[^)]*\)",
            "read_10x_h5($1)  # TODO: returns sparse matrix",
        ),
        // CreateSeuratObject — produces an scRNA object; nearest BioLang equiv is sc.setup()
        CallMap::new(
            r"\bCreateSeuratObject\([^)]*counts\s*=\s*([^,)]+),\s*project\s*=\s*([^,)]+)[^)]*\)",
            "sc.setup($1, name=$2)  # TODO: also set min_cells, min_features",
        ),
        CallMap::new(
            r"\bCreateSeuratObject\([^)]*counts\s*=\s*([^,)]+)[^)]*\)",
            "sc.setup($1)  # TODO: set min_cells, min_features",
        ),
        // Merge multiple Seurat objects
        CallMap::new(
            r"\bmerge\(([^,)]+),\s*y\s*=\s*([^,)]+)[^)]*\)",
            "sc.merge($1, $2)  # TODO: scRNA merge — check non-Seurat merge() uses",
        ),
        // ── Seurat: QC ───────────────────────────────────────────
        CallMap::new(
            r"\bPercentageFeatureSet\(([^,)]+),\s*pattern\s*=\s*([^)]+?)\)",
            "cell_qc($1, pattern=$2).pct_counts  # PercentageFeatureSet",
        ),
        // subset() in scRNA context — filter cells by metadata/expression threshold
        CallMap::new(
            r"\bsubset\(([^,)]+),\s*subset\s*=\s*([^)]+?)\)",
            "$1 |> sc.filter(fn(cell) -> $2)  # TODO: adapt Seurat subset syntax",
        ),
        // ── Seurat: normalisation & preprocessing ────────────────
        // SCTransform (variance-stabilising normalisation) — no direct BioLang equiv yet
        CallMap::new(
            r"\bSCTransform\(([^,)]+)[^)]*\)",
            "normalize_total($1) |> log1p_transform  # TODO: SCTransform → no exact equiv; use sctransform package",
        ),
        CallMap::new(
            r"\bNormalizeData\(([^,)]+)[^)]*\)",
            "normalize_total($1) |> log1p_transform",
        ),
        CallMap::new(
            r"\bFindVariableFeatures\(([^,)]+)[^)]*\)",
            "highly_variable_genes($1)",
        ),
        CallMap::new(r"\bScaleData\(([^,)]+)[^)]*\)", "scale_matrix($1)"),
        CallMap::new(r"\bCellCycleScoring\(([^,)]+)[^)]*\)",
            "$1  # TODO: CellCycleScoring → no direct BioLang equiv yet",
        ),
        CallMap::new(
            r"\bAddModuleScore\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "module_score($1, $2)  # TODO: AddModuleScore — verify module_score() signature",
        ),
        // ── Seurat: dimensionality reduction ─────────────────────
        CallMap::new(
            r"\bRunPCA\(([^,)]+)[^)]*\)",
            "pca($1)",
        ),
        CallMap::new(
            r"\bRunUMAP\(([^,)]+),\s*dims\s*=\s*([^,)]+)[^)]*\)",
            "umap($1, dims=$2)",
        ),
        CallMap::new(r"\bRunUMAP\(([^,)]+)[^)]*\)", "umap($1)"),
        CallMap::new(r"\bRunTSNE\(([^,)]+)[^)]*\)", "tsne($1)"),
        // ── Seurat: clustering ───────────────────────────────────
        CallMap::new(
            r"\bFindNeighbors\(([^,)]+)[^)]*\)",
            "knn_graph($1)",
        ),
        CallMap::new(
            r"\bFindClusters\(([^,)]+),\s*resolution\s*=\s*([^,)]+)[^)]*\)",
            "leiden($1, resolution=$2)  # FindClusters → Leiden algorithm",
        ),
        CallMap::new(r"\bFindClusters\(([^,)]+)[^)]*\)", "leiden($1)"),
        // ── Seurat: differential expression ─────────────────────
        CallMap::new(
            r"\bFindMarkers\(([^,)]+),\s*ident\.1\s*=\s*([^,)]+),\s*ident\.2\s*=\s*([^,)]+)[^)]*\)",
            "diff_expr($1, group1=$2, group2=$3)  # FindMarkers",
        ),
        CallMap::new(
            r"\bFindMarkers\(([^,)]+),\s*ident\.1\s*=\s*([^,)]+)[^)]*\)",
            "diff_expr($1, group=$2)  # FindMarkers one vs rest",
        ),
        CallMap::new(r"\bFindMarkers\(([^,)]+)[^)]*\)", "diff_expr($1)"),
        CallMap::new(
            r"\bFindAllMarkers\(([^,)]+)[^)]*\)",
            "$1 |> sc.find_all_markers  # TODO: no direct BioLang equiv; use diff_expr() per cluster",
        ),
        CallMap::new(
            r"\bFindConservedMarkers\(([^,)]+)[^)]*\)",
            "# TODO: FindConservedMarkers — no BioLang equiv yet",
        ),
        // ── Seurat: integration (Harmony, fastMNN, LIGER) ────────
        // Harmony is the most widely used integration method
        CallMap::new(
            r"\bRunHarmony\(([^,)]+),\s*group\.by\.vars\s*=\s*([^,)]+)[^)]*\)",
            "sc.integrate($1, batch_key=$2)  # TODO: Harmony — sc.integrate() not yet implemented",
        ),
        CallMap::new(r"\bRunHarmony\(([^,)]+)[^)]*\)", "sc.integrate($1)  # TODO: Harmony"),
        CallMap::new(
            r"\bIntegrateLayers\(([^,)]+)[^)]*\)",
            "sc.integrate($1)  # TODO: Seurat v5 IntegrateLayers",
        ),
        CallMap::new(
            r"\bIntegrateData\(([^,)]+)[^)]*\)",
            "sc.integrate($1)  # TODO: Seurat v4 IntegrateData",
        ),
        CallMap::new(
            r"\bRunFastMNN\(([^,)]+)[^)]*\)",
            "sc.integrate($1, method=\"fastmnn\")  # TODO: fastMNN wrapper",
        ),
        // ── Seurat: visualization ────────────────────────────────
        CallMap::new(
            r"\bDimPlot\(([^,)]+),\s*group\.by\s*=\s*([^,)]+)[^)]*\)",
            "scatter($1, color_by=$2)  # DimPlot",
        ),
        CallMap::new(r"\bDimPlot\(([^,)]+)[^)]*\)", "scatter($1)  # DimPlot"),
        CallMap::new(
            r"\bFeaturePlot\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "scatter($1, color_by=$2)  # FeaturePlot",
        ),
        CallMap::new(r"\bFeaturePlot\(([^,)]+)[^)]*\)", "scatter($1)  # FeaturePlot"),
        CallMap::new(
            r"\bVlnPlot\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "violin($1, genes=$2)  # VlnPlot",
        ),
        CallMap::new(r"\bVlnPlot\(([^,)]+)[^)]*\)", "violin($1)"),
        CallMap::new(
            r"\bDotPlot\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "dot_plot($1, genes=$2)  # TODO: dot_plot() not yet in BioLang builtins",
        ),
        CallMap::new(r"\bDotPlot\(([^,)]+)[^)]*\)", "dot_plot($1)  # TODO: not yet in BioLang"),
        CallMap::new(
            r"\bRidgePlot\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "ridge_plot($1, genes=$2)  # TODO: ridge_plot() not yet in BioLang",
        ),
        CallMap::new(r"\bRidgePlot\(([^,)]+)[^)]*\)", "ridge_plot($1)  # TODO: not yet in BioLang"),
        CallMap::new(
            r"\bDoHeatmap\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "heatmap($1, genes=$2)",
        ),
        CallMap::new(r"\bDoHeatmap\(([^,)]+)[^)]*\)", "heatmap($1)"),
        // ── Seurat: object access / metadata ─────────────────────
        CallMap::new(
            r"\bGetAssayData\(([^,)]+),\s*slot\s*=\s*([^,)]+)[^)]*\)",
            "$1.assay[$2]  # GetAssayData",
        ),
        CallMap::new(r"\bGetAssayData\(([^,)]+)[^)]*\)", "$1.counts  # GetAssayData"),
        CallMap::new(r"\bEmbeddings\(([^,)]+),\s*reduction\s*=\s*([^,)]+)[^)]*\)", "$1.embeddings[$2]"),
        CallMap::new(r"\bEmbeddings\(([^,)]+)[^)]*\)", "$1.embeddings"),
        CallMap::new(r"\bIdents\(([^)]+?)\)", "$1.clusters  # Idents"),
        CallMap::new(r"\bCells\(([^)]+?)\)", "$1.cell_ids  # Cells"),
        CallMap::new(
            r"\bWhichCells\(([^,)]+),\s*idents\s*=\s*([^,)]+)[^)]*\)",
            "$1 |> sc.filter(fn(c) -> c.cluster == $2)  # WhichCells",
        ),
        CallMap::new(
            r"\bFetchData\(([^,)]+),\s*vars\s*=\s*([^,)]+)[^)]*\)",
            "select($1, $2)  # FetchData",
        ),
        CallMap::new(r"\bFetchData\(([^,)]+)[^)]*\)", "$1.meta  # FetchData"),
        // SetIdent / RenameIdents — cluster labelling
        CallMap::new(
            r"\bSetIdent\(([^,)]+),\s*value\s*=\s*([^,)]+)\)",
            "$1  # TODO: SetIdent → assign $1.clusters = $2",
        ),
        CallMap::new(
            r"\bRenameIdents\(([^,)]+)[^)]*\)",
            "$1  # TODO: RenameIdents → remap cluster labels",
        ),
        // ── Seurat: trajectory (Monocle3 wrapper) ────────────────
        CallMap::new(
            r"\bRunMonocle3\(([^,)]+)[^)]*\)",
            "sc.trajectory($1)  # TODO: RunMonocle3 — sc.trajectory() not yet in BioLang",
        ),
        CallMap::new(
            r"\blearn_graph\(([^,)]+)[^)]*\)",
            "sc.trajectory_graph($1)  # TODO: Monocle3 learn_graph",
        ),
        CallMap::new(
            r"\border_cells\(([^,)]+)[^)]*\)",
            "pseudotime($1)  # TODO: Monocle3 order_cells → pseudotime not yet in BioLang",
        ),
        // ── Seurat: RNA velocity (scVelo wrapper) ─────────────────
        CallMap::new(
            r"\bRunScVelo\(([^,)]+)[^)]*\)",
            "sc.velocity($1)  # TODO: scVelo — not yet in BioLang",
        ),
        // ── Seurat: doublet detection (miQC / DoubletFinder) ─────
        CallMap::new(
            r"\bDoubletFinder\(([^,)]+)[^)]*\)",
            "doublet_score($1)  # DoubletFinder → doublet_score() builtin",
        ),
        CallMap::new(
            r"\bRunMiQC\(([^,)]+)[^)]*\)",
            "doublet_score($1)  # miQC → approximate via doublet_score()",
        ),
        // ── Seurat: misc utilities ────────────────────────────────
        CallMap::new(r"\bElbowPlot\(([^,)]+)[^)]*\)", "scatter(pca_variance($1))  # ElbowPlot"),
        CallMap::new(
            r"\bJackStrawPlot\(([^,)]+)[^)]*\)",
            "scatter($1)  # TODO: JackStrawPlot — no direct BioLang equiv",
        ),
        CallMap::new(
            r"\bSpatialFeaturePlot\(([^,)]+),\s*features\s*=\s*([^,)]+)[^)]*\)",
            "spatial_plot($1, genes=$2)  # TODO: spatial_plot() not yet in BioLang",
        ),
        CallMap::new(
            r"\bSpatialDimPlot\(([^,)]+)[^)]*\)",
            "spatial_plot($1)  # TODO: spatial_plot() not yet in BioLang",
        ),
        // ── Monocle3 / trajectory ─────────────────────────────────
        // specific (with keyword args) before bare
        CallMap::new(
            r"\bnew_cell_data_set\(([^,)]+),\s*cell_metadata\s*=\s*([^,)]+),\s*gene_metadata\s*=\s*([^,)]+)[^)]*\)",
            "sc.from_matrix($1, $3.gene_short_name, $2.rownames)  # Monocle3 CDS",
        ),
        CallMap::new(r"\bnew_cell_data_set\(([^,)]+)[^)]*\)", "sc.from_matrix($1, genes, barcodes)  # TODO: provide gene/barcode lists"),
        CallMap::new(r"\bpreprocess_cds\(([^,)]+)[^)]*\)", "normalize_total($1) |> log1p_transform"),
        CallMap::new(r"\breduce_dimension\(([^,)]+)[^)]*\)", "umap($1)"),
        CallMap::new(r"\bcluster_cells\(([^,)]+)[^)]*\)", "leiden($1)"),
        CallMap::new(r"\blearn_graph\(([^,)]+)[^)]*\)", "knn_graph($1)  # trajectory graph ≈ KNN"),
        CallMap::new(
            r"\border_cells\(([^,)]+),\s*root_pr_nodes\s*=\s*([^,)]+)[^)]*\)",
            "diffusion_pseudotime($1.obsm.X_diffmap, $1.obsp.distances, $2)",
        ),
        CallMap::new(r"\border_cells\(([^,)]+)[^)]*\)", "diffusion_pseudotime($1.obsm.X_diffmap, $1.obsp.distances, 0)"),
        CallMap::new(
            r"\bplot_cells\(([^,)]+),\s*color_cells_by\s*=\s*([^,)]+)[^)]*\)",
            "scatter($1, color_by=$2)",
        ),
        CallMap::new(r"\bplot_cells\(([^,)]+)[^)]*\)", "scatter($1)"),
        CallMap::new(r"\bgraph_test\(([^,)]+)[^)]*\)", "spatial_moransi($1.matrix, $1.spatial_adj)  # Moran's I for genes along trajectory"),
        CallMap::new(r"\bpseudotime\(([^,)]+)[^)]*\)", "$1.pseudotime"),
        // ── Slingshot / trajectory ────────────────────────────────
        CallMap::new(
            r"\bslingshot\(([^,)]+),\s*clusterLabels\s*=\s*([^,)]+),\s*reducedDim\s*=\s*([^,)]+)[^)]*\)",
            "diffusion_pseudotime($1.obsm.$3, $1.obsp.distances, 0)  # TODO: set start cluster",
        ),
        CallMap::new(r"\bslingshot\(([^,)]+)[^)]*\)", "diffusion_pseudotime($1.obsm.X_pca, $1.obsp.distances, 0)"),
        CallMap::new(r"\bSlingshotDataSet\(([^,)]+)[^)]*\)", "$1  # Slingshot wrapper, no-op"),
        CallMap::new(r"\bslingPseudotime\(([^,)]+)[^)]*\)", "$1.pseudotime"),
        CallMap::new(r"\bslingLineages\(([^,)]+)[^)]*\)", "# TODO: lineage graph not in BioLang yet"),
        CallMap::new(r"\bembedCurves\(([^,)]+)[^)]*\)", "umap($1)"),
        // ── Signac / ATAC-seq ─────────────────────────────────────
        CallMap::new(
            r"\bCreateChromatinAssay\(counts\s*=\s*([^,)]+)[^)]*\)",
            "$1  # TODO: ATAC assay — store as matrix",
        ),
        CallMap::new(r"\bRunTFIDF\(([^,)]+)[^)]*\)", "atac.tfidf($1)"),
        CallMap::new(
            r"\bFindTopFeatures\(([^,)]+),\s*min\.cutoff\s*=\s*([^,)]+)[^)]*\)",
            "atac.top_features($1, $2)",
        ),
        CallMap::new(r"\bFindTopFeatures\(([^,)]+)[^)]*\)", "atac.top_features($1)"),
        CallMap::new(r"\bRunSVD\(([^,)]+)[^)]*\)", "atac.lsi($1)"),
        CallMap::new(r"\bDepthCor\(([^,)]+)[^)]*\)", "atac.depth_cor($1)"),
        CallMap::new(
            r"\bLinkPeaks\(([^,)]+)[^)]*\)",
            "# TODO: peak-gene linking not in BioLang yet",
        ),
        CallMap::new(r"\bAnnotation\(([^,)]+)\)", "$1.annotation"),
        CallMap::new(r"\bGeneActivity\(([^,)]+)[^)]*\)", "$1.gene_activity  # TODO: compute gene activity scores"),
        // ── MAST / single-cell DE ─────────────────────────────────
        CallMap::new(
            r"\bFromMatrix\(t\(([^)]+?)\),\s*cData\s*=\s*([^,)]+),\s*fData\s*=\s*([^,)]+)[^)]*\)",
            "de.setup($1, $3)  # MAST SingleCellAssay",
        ),
        CallMap::new(
            r"\bzlm\(~condition,\s*sca\s*=\s*([^,)]+)[^)]*\)",
            "de.fit($1)  # zero-inflated linear model ≈ DE fit",
        ),
        CallMap::new(r"\bzlm\(([^)]+?)\)", "de.fit($1)  # MAST zlm"),
        CallMap::new(
            r"\blrTest\(([^,)]+),\s*contrast\s*=\s*([^,)]+)[^)]*\)",
            "de.contrast($1, $2)",
        ),
        // ── SingleCellExperiment (Bioconductor) ───────────────────
        CallMap::new(
            r"\bSingleCellExperiment\(assays\s*=\s*list\(counts\s*=\s*([^)]+?)\)[^)]*\)",
            "sc.from_matrix($1, rownames($1), colnames($1))",
        ),
        CallMap::new(r"\bcounts\(([^,)]+)\)", "$1.matrix"),
        CallMap::new(r"\blogcounts\(([^,)]+)\)", "$1.norm_matrix"),
        CallMap::new(r"\bcolData\(([^,)]+)\)", "$1.obs"),
        CallMap::new(r"\browData\(([^,)]+)\)", "$1.var"),
        CallMap::new(
            r#"\breducedDim\(([^,)]+),\s*["']PCA["']\)"#,
            "$1.obsm.X_pca",
        ),
        CallMap::new(
            r#"\breducedDim\(([^,)]+),\s*["']UMAP["']\)"#,
            "$1.obsm.X_umap",
        ),
        CallMap::new(r"\breducedDim\(([^,)]+),\s*([^)]+?)\)", "$1.obsm.$2"),
        // ── scran / Bioconductor preprocessing ────────────────────
        // specific (with arg) before bare
        CallMap::new(
            r"\bgetTopHVGs\(([^,)]+),\s*n\s*=\s*([^,)]+)[^)]*\)",
            "highly_variable_genes($1, $2)",
        ),
        CallMap::new(r"\bgetTopHVGs\(([^,)]+)[^)]*\)", "highly_variable_genes($1)"),
        CallMap::new(r"\bcomputeSumFactors\(([^,)]+)[^)]*\)", "normalize_total($1)  # pooling-based size factors"),
        CallMap::new(r"\bquickCluster\(([^,)]+)[^)]*\)", "leiden($1)  # quick cluster ≈ Leiden"),
        CallMap::new(r"\bmodelGeneVar\(([^,)]+)[^)]*\)", "highly_variable_genes($1)  # variance modelling"),
        CallMap::new(r"\bdenoisePCA\(([^,)]+)[^)]*\)", "pca($1)  # denoised PCA"),
        CallMap::new(
            r"\bbuildSNNGraph\(([^,)]+),\s*k\s*=\s*([^,)]+)[^)]*\)",
            "knn_graph($1, k=$2)",
        ),
        CallMap::new(r"\bbuildSNNGraph\(([^,)]+)[^)]*\)", "knn_graph($1)"),
        // ── I/O ───────────────────────────────────────────────────
        CallMap::new(r"\bread\.csv\(([^,)]+)[^)]*\)", "read_csv($1)"),
        CallMap::new(r"\bread\.table\(([^,)]+)[^)]*\)", "read_tsv($1)"),
        CallMap::new(r"\bread\.delim\(([^,)]+)[^)]*\)", "read_tsv($1)"),
        CallMap::new(
            r"\bwrite\.csv\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "write_csv($1, $2)",
        ),
        CallMap::new(
            r"\bwrite\.table\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "write_tsv($1, $2)",
        ),
        CallMap::new(r"\breadrFasta\(([^)]+?)\)", "read_fasta($1)"),
        CallMap::new(
            r"\breadLines\(([^)]+?)\)",
            r#"read_file($1) |> split("\n")"#,
        ),
        CallMap::new(
            r"\bwriteLines\(([^,)]+),\s*([^)]+?)\)",
            r#"write_file($2, $1 |> join("\n"))"#,
        ),
        CallMap::new(
            r"\bsaveRDS\(([^,)]+),\s*([^)]+?)\)",
            "# TODO: saveRDS → use json_dump() or write_csv()",
        ),
        CallMap::new(
            r"\breadRDS\(([^)]+?)\)",
            "# TODO: readRDS → use json_parse() or read_csv()",
        ),
        CallMap::new(r"\bload\(([^)]+?)\)", "# TODO: load(.RData) not supported"),
        // ── dplyr ────────────────────────────────────────────────
        CallMap::new(
            r"\bfilter\(([^,)]+),\s*(.+)\)",
            "$1 |> filter(fn(r) -> $2)  # TODO: adjust dplyr filter",
        ),
        CallMap::new(r"\bselect\(([^,)]+),\s*(.+)\)", "select($1, [$2])"),
        CallMap::new(
            r"\bmutate\(([^,)]+),\s*(.+)\)",
            "$1 |> add_column($2)  # TODO: dplyr mutate",
        ),
        CallMap::new(r"\barrange\(([^,)]+),\s*(.+)\)", "sort_by($1, $2)"),
        CallMap::new(r"\bgroup_by\(([^,)]+),\s*(.+)\)", "group_by($1, $2)"),
        CallMap::new(
            r"\bsummarise\(([^,)]+),\s*(.+)\)",
            "summarize($1, $2)  # TODO: adjust",
        ),
        CallMap::new(
            r"\bsummarize\(([^,)]+),\s*(.+)\)",
            "summarize($1, $2)  # TODO: adjust",
        ),
        CallMap::new(
            r"\bleft_join\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "join($1, $2, how=\"left\")",
        ),
        CallMap::new(
            r"\binner_join\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "join($1, $2, how=\"inner\")",
        ),
        CallMap::new(
            r"\bfull_join\(([^,)]+),\s*([^,)]+)[^)]*\)",
            "join($1, $2, how=\"full\")",
        ),
        CallMap::new(r"\bdistinct\(([^)]+?)\)", "unique($1)"),
        CallMap::new(r"\bpull\(([^,)]+),\s*([^)]+?)\)", "$1.$2"),
        CallMap::new(r"\bslice\(([^,)]+),\s*([^)]+?)\)", "rows($1, $2)"),
        CallMap::new(
            r"\brename\(([^,)]+),\s*(.+)\)",
            "rename_col($1, $2)  # TODO: adjust",
        ),
        CallMap::new(
            r"\bpivot_wider\(([^)]+?)\)",
            "pivot_wider($1)  # TODO: adjust args",
        ),
        CallMap::new(
            r"\bpivot_longer\(([^)]+?)\)",
            "pivot_longer($1)  # TODO: adjust args",
        ),
        CallMap::new(r"\bdrop_na\(([^)]+?)\)", "drop_na($1)"),
        // ── tidyr ────────────────────────────────────────────────
        CallMap::new(
            r"\bspread\(([^)]+?)\)",
            "pivot_wider($1)  # TODO: adjust args",
        ),
        CallMap::new(
            r"\bgather\(([^)]+?)\)",
            "pivot_longer($1)  # TODO: adjust args",
        ),
        CallMap::new(r"\bunite\(([^)]+?)\)", "# TODO: unite → str_concat columns"),
        CallMap::new(r"\bseparate\(([^)]+?)\)", "# TODO: separate → split column"),
        // ── stringr ──────────────────────────────────────────────
        CallMap::new(r"\bstr_detect\(([^,)]+),\s*([^)]+?)\)", "match_re($1, $2)"),
        CallMap::new(r"\bstr_extract\(([^,)]+),\s*([^)]+?)\)", "find_re($1, $2)"),
        CallMap::new(
            r"\bstr_replace\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "replace_re($1, $2, $3)",
        ),
        CallMap::new(r"\bstr_split\(([^,)]+),\s*([^)]+?)\)", "split($1, $2)"),
        CallMap::new(r"\bstr_to_upper\(([^)]+?)\)", "to_upper($1)"),
        CallMap::new(r"\bstr_to_lower\(([^)]+?)\)", "to_lower($1)"),
        CallMap::new(r"\bstr_trim\(([^)]+?)\)", "trim($1)"),
        CallMap::new(r"\bstr_length\(([^)]+?)\)", "len($1)"),
        CallMap::new(r"\bstr_pad\(([^)]+?)\)", "# TODO: str_pad → pad()"),
        CallMap::new(r"\bnchar\(([^)]+?)\)", "len($1)"),
        CallMap::new(
            r"\bpaste0\(([^)]+?)\)",
            "# TODO: paste0($1) → string concatenation",
        ),
        CallMap::new(
            r#"\bpaste\(([^,)]+),\s*([^,)]+),\s*sep\s*=\s*['"]([^'"]*)['"]\)"#,
            r#"join([$1, $2], "$3")"#,
        ),
        CallMap::new(
            r"\bpaste\(([^)]+?)\)",
            "join([$1], \" \")  # TODO: adjust paste args",
        ),
        CallMap::new(
            r"\bsprintf\(([^)]+?)\)",
            "# TODO: sprintf → string interpolation",
        ),
        CallMap::new(
            r"\bformat\(([^,)]+),\s*digits\s*=\s*(\d+)[^)]*\)",
            "round($1, $2)",
        ),
        CallMap::new(
            r"\bgsub\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "replace_re($3, $1, $2)",
        ),
        CallMap::new(
            r"\bsub\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "replace_re($3, $1, $2)",
        ),
        CallMap::new(
            r"\bgrepl\(([^,)]+),\s*([^)]+?)\)",
            "match_re($2, $1) != null",
        ),
        CallMap::new(r"\bgrep\(([^,)]+),\s*([^)]+?)\)", "find_all_re($2, $1)"),
        CallMap::new(r"\bstrsplit\(([^,)]+),\s*([^)]+?)\)", "split($1, $2)"),
        CallMap::new(
            r"\bsubstring\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "subseq($1, $2, $3)",
        ),
        CallMap::new(
            r"\bsubstr\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "subseq($1, $2, $3)",
        ),
        // ── stats ─────────────────────────────────────────────────
        CallMap::new(r"\bt\.test\(([^,)]+),\s*([^)]+?)\)", "ttest($1, $2)"),
        CallMap::new(r"\bt\.test\(([^)]+?)\)", "ttest($1)"),
        CallMap::new(
            r"\bwilcox\.test\(([^,)]+),\s*([^)]+?)\)",
            "wilcoxon($1, $2)",
        ),
        CallMap::new(r"\baov\(([^)]+?)\)", "anova($1)  # TODO: adjust formula"),
        CallMap::new(r"\banova\(([^)]+?)\)", "anova($1)"),
        CallMap::new(r"\bchisq\.test\(([^)]+?)\)", "chi_square($1)"),
        CallMap::new(r"\bfisher\.test\(([^)]+?)\)", "fisher_exact($1)"),
        CallMap::new(r"\bcor\(([^,)]+),\s*([^,)]+)\)", "cor($1, $2)"),
        CallMap::new(r"\bcor\.test\(([^,)]+),\s*([^,)]+)\)", "cor($1, $2)"),
        CallMap::new(r"\blm\(([^)]+?)\)", "lm($1)  # TODO: adjust formula"),
        CallMap::new(
            r"\bglm\(([^)]+?)\)",
            "lm($1)  # TODO: generalized lm — check family",
        ),
        CallMap::new(
            r#"\bp\.adjust\(([^,)]+),\s*method\s*=\s*['"]([^'"]+)['"][^)]*\)"#,
            r#"p_adjust($1, method="$2")"#,
        ),
        CallMap::new(r"\bp\.adjust\(([^)]+?)\)", "p_adjust($1)"),
        CallMap::new(r"\bpnorm\(([^)]+?)\)", "pnorm($1)  # TODO: verify"),
        CallMap::new(r"\bqnorm\(([^)]+?)\)", "qnorm($1)  # TODO: verify"),
        CallMap::new(
            r"\brnorm\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "rnorm($1, mean=$2, sd=$3)",
        ),
        CallMap::new(r"\bset\.seed\(([^)]+?)\)", "set_seed($1)"),
        CallMap::new(r"\brunif\(([^)]+?)\)", "random_vec($1)"),
        CallMap::new(r"\bsample\(([^,)]+),\s*([^,)]+)\)", "sample($1, $2)"),
        // ── base R vectors / lists ────────────────────────────────
        CallMap::new(r"\bc\(([^)]+?)\)", "[$1]"),
        CallMap::new(r"\blist\(([^)]+?)\)", "{$1}"),
        CallMap::new(r#"\bvector\(['"]numeric['"],\s*([^)]+?)\)"#, "zeros($1)"),
        CallMap::new(
            r#"\bvector\(['"]character['"],\s*([^)]+?)\)"#,
            r#"replicate("", $1)"#,
        ),
        CallMap::new(r"\bnumeric\(([^)]+?)\)", "zeros($1)"),
        CallMap::new(r"\bcharacter\(([^)]+?)\)", r#"replicate("", $1)"#),
        CallMap::new(r"\binteger\(([^)]+?)\)", "zeros($1)"),
        CallMap::new(r"\bdata\.frame\(([^)]+?)\)", "table($1)"),
        CallMap::new(
            r"\bmatrix\(([^,)]+),\s*nrow\s*=\s*([^,)]+),\s*ncol\s*=\s*([^)]+?)\)",
            "reshape($1, [$2, $3])",
        ),
        CallMap::new(r"\brbind\(([^)]+?)\)", "concat_rows($1)"),
        CallMap::new(r"\bcbind\(([^)]+?)\)", "concat_cols($1)"),
        CallMap::new(r"\bappend\(([^,)]+),\s*([^)]+?)\)", "$1 + [$2]"),
        CallMap::new(r"\brev\(([^)]+?)\)", "reverse($1)"),
        CallMap::new(r"\bunique\(([^)]+?)\)", "unique($1)"),
        CallMap::new(r"\bwhich\(([^)]+?)\)", "find_indices($1)"),
        CallMap::new(r"\btable\(([^)]+?)\)", "count_values($1)"),
        CallMap::new(r"\bnames\(([^)]+?)\)", "col_names($1)"),
        CallMap::new(r"\bcolnames\(([^)]+?)\)", "col_names($1)"),
        CallMap::new(r"\brownames\(([^)]+?)\)", "row_names($1)"),
        CallMap::new(r"\bnrow\(([^)]+?)\)", "nrows($1)"),
        CallMap::new(r"\bncol\(([^)]+?)\)", "ncols($1)"),
        CallMap::new(r"\bdim\(([^)]+?)\)", "[nrows($1), ncols($1)]"),
        CallMap::new(r"\bhead\(([^,)]+),\s*(\d+)\)", "head($1, $2)"),
        CallMap::new(r"\bhead\(([^)]+?)\)", "head($1, 6)"),
        CallMap::new(r"\btail\(([^,)]+),\s*(\d+)\)", "tail($1, $2)"),
        CallMap::new(r"\btail\(([^)]+?)\)", "tail($1, 6)"),
        CallMap::new(r"\bsapply\(([^,)]+),\s*([^)]+?)\)", "$1 |> map($2)"),
        CallMap::new(r"\blapply\(([^,)]+),\s*([^)]+?)\)", "$1 |> map($2)"),
        CallMap::new(
            r"\bvapply\(([^,)]+),\s*([^,)]+),\s*[^)]+?\)",
            "$1 |> map($2)",
        ),
        CallMap::new(r"\bapply\(([^,)]+),\s*1,\s*([^)]+?)\)", "row_apply($1, $2)"),
        CallMap::new(r"\bapply\(([^,)]+),\s*2,\s*([^)]+?)\)", "col_apply($1, $2)"),
        CallMap::new(r"\bReduce\(([^,)]+),\s*([^)]+?)\)", "$2 |> reduce($1)"),
        CallMap::new(r"\bFilter\(([^,)]+),\s*([^)]+?)\)", "$2 |> filter($1)"),
        CallMap::new(r"\bMap\(([^,)]+),\s*([^)]+?)\)", "$2 |> map($1)"),
        CallMap::new(
            r"\bseq\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "range($1, $2, $3)",
        ),
        CallMap::new(r"\bseq_len\(([^)]+?)\)", "range($1)"),
        CallMap::new(r"\bseq_along\(([^)]+?)\)", "range(len($1))"),
        // ── base R output ─────────────────────────────────────────
        CallMap::new(r"\bcat\(([^)]+?)\)", "println($1)"),
        CallMap::new(r"\bprint\(([^)]+?)\)", "println($1)"),
        CallMap::new(r"\bmessage\(([^)]+?)\)", "println($1)"),
        CallMap::new(r"\bwarning\(([^)]+?)\)", "# warn: $1"),
        CallMap::new(
            r"\bstop\(([^)]+?)\)",
            "# TODO: error $1 — use error() or return error value",
        ),
        // ── Math ──────────────────────────────────────────────────
        CallMap::new(r"\bsqrt\(([^)]+?)\)", "sqrt($1)"),
        CallMap::new(r"\babs\(([^)]+?)\)", "abs($1)"),
        CallMap::new(r"\bexp\(([^)]+?)\)", "exp($1)"),
        CallMap::new(r"\blog2\(([^)]+?)\)", "log2($1)"),
        CallMap::new(r"\blog10\(([^)]+?)\)", "log10($1)"),
        CallMap::new(r"\blog\(([^)]+?)\)", "log($1)"),
        CallMap::new(r"\bceiling\(([^)]+?)\)", "ceil($1)"),
        CallMap::new(r"\bfloor\(([^)]+?)\)", "floor($1)"),
        CallMap::new(r"\bround\(([^,)]+),\s*(\d+)\)", "round($1, $2)"),
        CallMap::new(r"\bround\(([^)]+?)\)", "round($1)"),
        CallMap::new(r"\bmax\(([^)]+?)\)", "max($1)"),
        CallMap::new(r"\bmin\(([^)]+?)\)", "min($1)"),
        CallMap::new(r"\bsum\(([^)]+?)\)", "sum($1)"),
        CallMap::new(r"\bmean\(([^)]+?)\)", "mean($1)"),
        CallMap::new(r"\bmedian\(([^)]+?)\)", "median($1)"),
        CallMap::new(r"\bsd\(([^)]+?)\)", "stdev($1)"),
        CallMap::new(r"\bvar\(([^)]+?)\)", "variance($1)"),
        CallMap::new(r"\brange\(([^)]+?)\)", "[min($1), max($1)]"),
        CallMap::new(r"\bquantile\(([^,)]+),\s*([^)]+?)\)", "quantile($1, $2)"),
        // ── System / OS ───────────────────────────────────────────
        CallMap::new(r"\bSys\.getenv\(([^)]+?)\)", "env($1)"),
        CallMap::new(r"\bSys\.setenv\(([^)]+?)\)", "# TODO: Sys.setenv($1)"),
        CallMap::new(r"\bSys\.time\(\)", "now()"),
        CallMap::new(r"\bgetwd\(\)", "env(\"PWD\")"),
        CallMap::new(
            r"\bsetwd\(([^)]+?)\)",
            "# TODO: setwd not needed — use absolute paths",
        ),
        CallMap::new(r"\bfile\.exists\(([^)]+?)\)", "file_exists($1)"),
        CallMap::new(r"\bfile\.path\(([^)]+?)\)", "path_join($1)"),
        CallMap::new(r"\bdir\.create\(([^)]+?)\)", "mkdir($1)"),
        CallMap::new(r"\blist\.files\(([^)]+?)\)", "list_dir($1)"),
        CallMap::new(r"\bsystem\(([^)]+?)\)", "shell($1)"),
        CallMap::new(
            r"\bsystem2\(([^,)]+)[^)]*\)",
            "shell($1)  # TODO: adjust args",
        ),
        CallMap::new(r"\bproc\.time\(\)", "# TODO: use time_it()"),
        CallMap::new(r"\bsystem\.time\(([^)]+?)\)", "# TODO: time_it(fn() -> $1)"),
        CallMap::new(
            r"\bsource\(([^)]+?)\)",
            "import $1 as _  # TODO: adjust module path",
        ),
        CallMap::new(
            r"\brequire\(([^)]+?)\)",
            "# handled by library() conversion above",
        ),
        // ── ggplot2 (basic) ───────────────────────────────────────
        CallMap::new(
            r"\bggplot\(([^,)]+)[^)]*\)\s*\+",
            "# ggplot($1) → use BioLang plot builtins:",
        ),
        CallMap::new(r"\bgeom_point\([^)]*\)", "scatter  # TODO: scatter(x, y)"),
        CallMap::new(
            r"\bgeom_line\([^)]*\)",
            "line_plot  # TODO: line_plot(x, y)",
        ),
        CallMap::new(r"\bgeom_bar\([^)]*\)", "bar  # TODO: bar(labels, values)"),
        CallMap::new(r"\bgeom_histogram\([^)]*\)", "hist  # TODO: hist(values)"),
        CallMap::new(
            r"\bgeom_boxplot\([^)]*\)",
            "box_plot  # TODO: box_plot(groups, values)",
        ),
        CallMap::new(
            r"\bgeom_violin\([^)]*\)",
            "violin  # TODO: violin(groups, values)",
        ),
        CallMap::new(
            r"\bgeom_heatmap\([^)]*\)",
            "heatmap  # TODO: heatmap(matrix)",
        ),
        CallMap::new(r"\bggsave\(([^,)]+)[^)]*\)", "save_plot($1)"),
        CallMap::new(
            r"\btheme\([^)]*\)",
            "# TODO: ggplot theme → not needed in BioLang",
        ),
        CallMap::new(
            r"\bscale_color_manual\([^)]*\)",
            "# TODO: color scale → use color= in plot call",
        ),
        CallMap::new(
            r"\bfacet_wrap\([^)]*\)",
            "# TODO: faceting not yet in BioLang",
        ),
        // ── VariantAnnotation / vcfR ──────────────────────────────
        // Specific (with args) before bare forms
        CallMap::new(
            r"\breadVcf\(([^,)]+),\s*([^)]+?)\)",
            "vcf_parse($1)  # genome=$2 ignored — use vcf_filter() to restrict regions",
        ),
        CallMap::new(r"\breadVcf\(([^)]+?)\)", "vcf_parse($1)"),
        CallMap::new(
            r"\bvcfR::read\.vcf\(([^)]+?)\)",
            "vcf_parse($1)",
        ),
        CallMap::new(r"\bread\.vcfR\(([^)]+?)\)", "vcf_parse($1)"),
        CallMap::new(
            r"\bvcfR2tidy\(([^)]+?)\)",
            "# TODO: vcfR2tidy — use vcf_parse() + table()",
        ),
        CallMap::new(r"\bgetINFO\(([^)]+?)\)", "$1.info"),
        CallMap::new(r"\bgetGENO\(([^)]+?)\)", "$1.genotypes"),
        CallMap::new(r"\bgetREF\(([^)]+?)\)", "$1.ref"),
        CallMap::new(r"\bgetALT\(([^)]+?)\)", "$1.alt"),
        CallMap::new(r"\bgetQUAL\(([^)]+?)\)", "$1.qual"),
        CallMap::new(r"\bgetFILTER\(([^)]+?)\)", "$1.filter"),
        CallMap::new(
            r"\bfilterVcf\(([^,)]+),\s*([^,)]+),\s*([^)]+?)\)",
            "vcf_filter($1, region=$2)  # TODO: adjust filter predicate",
        ),
        CallMap::new(r"\bfilterVcf\(([^)]+?)\)", "vcf_filter($1)"),
        CallMap::new(r"\binfo\(([^)]+?)\)", "$1.info"),
        CallMap::new(r"\bgeno\(([^)]+?)\)", "$1.genotypes"),
        CallMap::new(r"\bref\(([^)]+?)\)", "$1.ref"),
        CallMap::new(r"\balt\(([^)]+?)\)", "$1.alt"),
        CallMap::new(r"\bfixed\(([^)]+?)\)", "$1.fixed"),
        CallMap::new(
            r"\bwriteVcf\(([^,)]+),\s*([^)]+?)\)",
            "# TODO: writeVcf → write_file($2, format_vcf($1))",
        ),
        CallMap::new(
            r"\bvariantSummary\(([^)]+?)\)",
            "variant_summary($1)",
        ),
        CallMap::new(r"\bti\.tv\.ratio\(([^)]+?)\)", "titv_ratio($1)"),
        // ── tximport / tximeta ────────────────────────────────────
        CallMap::new(
            r"\btximport\(files\s*=\s*([^,)]+),\s*type\s*=\s*([^,)]+)[^)]*\)",
            "parse_salmon($1)  # type=$2; use parse_featurecounts() for featureCounts input",
        ),
        CallMap::new(r"\btximport\(([^)]+?)\)", "parse_salmon($1)  # TODO: verify type"),
        CallMap::new(
            r"\btximeta\(([^)]+?)\)",
            "parse_salmon($1)  # tximeta ≈ tximport with auto-metadata",
        ),
        CallMap::new(
            r"\bscaleData\(([^)]+?)\)",
            "size_factors($1)",
        ),
        CallMap::new(
            r"\bfiltByExpr\(([^)]+?)\)",
            "filter_low_counts($1)  # edgeR::filterByExpr ≈ filter_low_counts",
        ),
        CallMap::new(
            r"\bfilterByExpr\(([^)]+?)\)",
            "filter_low_counts($1)",
        ),
        CallMap::new(r"\bcalcNormFactors\(([^)]+?)\)", "size_factors($1)"),
        CallMap::new(
            r"\bvoom\(([^)]+?)\)",
            "# TODO: limma/voom — use import \"differential\" as de",
        ),
        // ── ape / phangorn / phylogenetics ────────────────────────
        // Specific (multi-arg) before bare
        CallMap::new(
            r"\bread\.tree\(([^)]+?),\s*format\s*=\s*([^)]+?)\)",
            "nw_parse(read_file($1))  # format=$2 — only newick supported directly",
        ),
        CallMap::new(r"\bread\.tree\(([^)]+?)\)", "nw_parse(read_file($1))"),
        CallMap::new(r"\bread\.nexus\(([^)]+?)\)", "# TODO: Nexus format — convert to Newick first"),
        CallMap::new(
            r"\bcophenetic\.phylo\(([^)]+?)\)",
            "nw_to_distance_matrix($1)",
        ),
        CallMap::new(r"\bcophenetic\(([^)]+?)\)", "nw_to_distance_matrix($1)"),
        CallMap::new(
            r"\bphangorn::NJ\(([^)]+?)\)",
            "upgma($1)  # TODO: NJ ≈ upgma for small trees; NJ not yet a builtin",
        ),
        CallMap::new(r"\bNJ\(([^)]+?)\)", "upgma($1)  # TODO: NJ → upgma approximation"),
        CallMap::new(r"\bnj\(([^)]+?)\)", "upgma($1)  # TODO: NJ → upgma approximation"),
        CallMap::new(r"\bape::nj\(([^)]+?)\)", "upgma($1)  # TODO: NJ not yet a builtin"),
        CallMap::new(r"\bUPGMA\(([^)]+?)\)", "upgma($1)"),
        CallMap::new(r"\bdist\.dna\(([^)]+?)\)", "nw_to_distance_matrix($1)"),
        CallMap::new(r"\bdist\.ml\(([^)]+?)\)", "nw_to_distance_matrix($1)"),
        CallMap::new(
            r"\bphangorn::phyDat\(([^)]+?)\)",
            "# TODO: phyDat → BioLang alignment — use read_fasta()",
        ),
        CallMap::new(r"\btips\(([^)]+?)\)", "tree_leaves($1)"),
        CallMap::new(r"\bTips\(([^)]+?)\)", "tree_leaves($1)"),
        CallMap::new(r"\btip\.label", ".leaves"),
        CallMap::new(r"\bwrite\.tree\(([^,)]+)[^)]*\)", "# TODO: write_file(\"tree.nwk\", $1 |> nw_format())"),
        // ── ChIPseeker / DiffBind ─────────────────────────────────
        CallMap::new(
            r"\btoGRanges\(([^)]+?),\s*format\s*=\s*([^)]+?)\)",
            "read_bed($1)  # ChIPseeker toGRanges ≈ read_bed",
        ),
        CallMap::new(r"\btoGRanges\(([^)]+?)\)", "read_bed($1)"),
        CallMap::new(
            r"\bannotatePeak\(([^,)]+),\s*tssRegion\s*=\s*([^,)]+),\s*TxDb\s*=\s*([^,)]+)[^)]*\)",
            "peak_annotation($1)  # TODO: TxDb=$3; use import \"chipseq\" as chip",
        ),
        CallMap::new(r"\bannotatePeak\(([^)]+?)\)", "peak_annotation($1)"),
        CallMap::new(
            r"\bplotAnnoPie\(([^)]+?)\)",
            "# TODO: plotAnnoPie → use bar() on $1.annotation_counts",
        ),
        CallMap::new(
            r"\bplotDistToTSS\(([^)]+?)\)",
            "# TODO: plotDistToTSS → hist($1.dist_to_tss)",
        ),
        // DiffBind — specific before bare
        CallMap::new(
            r"\bdba\(sampleSheet\s*=\s*([^)]+?)\)",
            "# TODO: DiffBind dba() — use read_csv($1) + merge_peaks()",
        ),
        CallMap::new(r"\bdba\(([^)]+?)\)", "# TODO: dba($1) — import \"chipseq\" as chip"),
        CallMap::new(r"\bdba\.count\(([^)]+?)\)", "frip_score($1)  # TODO: adjust"),
        CallMap::new(r"\bdba\.contrast\(([^)]+?)\)", "# TODO: dba.contrast → diff_expr()"),
        CallMap::new(r"\bdba\.analyze\(([^)]+?)\)", "diff_expr($1)  # TODO: DiffBind analyze"),
        CallMap::new(
            r"\bdba\.report\(([^)]+?)\)",
            "$1 |> filter(fn(r) -> r.FDR < 0.05)  # DiffBind report ≈ filter sig peaks",
        ),
        CallMap::new(
            r"\bdba\.peakset\(([^)]+?)\)",
            "consensus_peaks($1)  # TODO: adjust",
        ),
        // ── phyloseq / vegan / microbiome ─────────────────────────
        // phyloseq constructors
        CallMap::new(
            r"\bphyloseq\(otu_table\s*=\s*([^,)]+),\s*sample_data\s*=\s*([^,)]+)[^)]*\)",
            "table($1, metadata=$2)  # TODO: import \"microbiome\" as mb",
        ),
        CallMap::new(r"\bphyloseq\(([^)]+?)\)", "# TODO: phyloseq($1) — import \"microbiome\" as mb"),
        CallMap::new(
            r"\botu_table\(([^,)]+),\s*taxa_are_rows\s*=\s*([^)]+?)\)",
            "$1  # otu_table — taxa_are_rows=$2",
        ),
        CallMap::new(r"\botu_table\(([^)]+?)\)", "$1"),
        CallMap::new(r"\bsample_data\(([^)]+?)\)", "$1.metadata"),
        CallMap::new(r"\btax_table\(([^)]+?)\)", "$1.taxonomy"),
        CallMap::new(r"\bphy_tree\(([^)]+?)\)", "nw_parse($1)"),
        CallMap::new(
            r"\btax_glom\(([^,)]+),\s*taxrank\s*=\s*([^)]+?)\)",
            "taxonomic_collapse($1, rank=$2)",
        ),
        CallMap::new(r"\btax_glom\(([^)]+?)\)", "taxonomic_collapse($1)"),
        CallMap::new(
            r"\brarefy_even_depth\(([^,)]+)[^)]*\)",
            "rarefaction($1)  # phyloseq rarefy_even_depth ≈ rarefaction()",
        ),
        CallMap::new(r"\bestimate_richness\(([^)]+?)\)", "alpha_diversity($1)"),
        CallMap::new(r"\bprune_taxa\(([^,)]+),\s*([^)]+?)\)", "filter($2, fn(t) -> t.id in $1)"),
        CallMap::new(r"\bsubset_samples\(([^,)]+),\s*([^)]+?)\)", "filter($1, fn(r) -> $2)"),
        // vegan
        CallMap::new(
            r"\bvegan::diversity\(([^,)]+),\s*index\s*=\s*([^)]+?)\)",
            "alpha_diversity($1, method=$2)",
        ),
        CallMap::new(r"\bdiversity\(([^,)]+),\s*index\s*=\s*([^)]+?)\)", "alpha_diversity($1, method=$2)"),
        CallMap::new(r"\bdiversity\(([^)]+?)\)", "alpha_diversity($1)"),
        CallMap::new(
            r"\bvegan::vegdist\(([^,)]+),\s*method\s*=\s*([^)]+?)\)",
            "beta_diversity($1, method=$2)",
        ),
        CallMap::new(r"\bvegdist\(([^,)]+),\s*method\s*=\s*([^)]+?)\)", "beta_diversity($1, method=$2)"),
        CallMap::new(r"\bvegdist\(([^)]+?)\)", "beta_diversity($1)"),
        CallMap::new(
            r"\bvegan::rrarefy\(([^,)]+),\s*([^)]+?)\)",
            "rarefaction($1, depth=$2)",
        ),
        CallMap::new(r"\brrarefy\(([^,)]+),\s*([^)]+?)\)", "rarefaction($1, depth=$2)"),
        CallMap::new(r"\bspecnumber\(([^)]+?)\)", "alpha_diversity($1, method=\"richness\")"),
        CallMap::new(
            r"\badonis2?\(([^)]+?)\)",
            "# TODO: adonis/PERMANOVA — use beta_diversity() + permutation test",
        ),
        CallMap::new(r"\border\(([^)]+?)\)", "# TODO: vegan ordination — use pca() or umap()"),
    ]
}

pub fn convert(source: &str, filename: &str) -> String {
    let call_maps = build_call_maps();
    let mut output = String::new();
    let mut todos = 0usize;

    output.push_str(&format!(
        "# Converted from R: {filename}\n\
         # Review all `# TODO:` markers before running.\n\
         # Validate with: bl check <output>.bl\n\n"
    ));

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    // Precompute cumulative paren depth at the start of each line so that
    // continuation lines of multi-line calls can be detected without tracking
    // state inside the loop. Ignores parens inside string literals (acceptable
    // approximation for typical R code).
    let mut cum_depth = vec![0i32; lines.len() + 1];
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        let delta = t.chars().filter(|&c| c == '(').count() as i32
            - t.chars().filter(|&c| c == ')').count() as i32;
        cum_depth[idx + 1] = cum_depth[idx] + delta;
    }

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();
        let curr_indent = leading_spaces(raw);
        let indent_str = " ".repeat(curr_indent);

        // ── Blank line ────────────────────────────────────────────
        if trimmed.is_empty() {
            output.push('\n');
            i += 1;
            continue;
        }

        // ── Continuation line inside a multi-line call ────────────
        // When the cumulative paren depth at the start of this line is > 0 we
        // are inside unclosed parentheses from previous line(s). Pass the
        // fragment through expression transforms but do NOT attempt assignment
        // parsing — named keyword args like `min.cells = 3` must not become
        // `let min.cells = 3`.
        if cum_depth[i] > 0 {
            let out = transform_r_expr(trimmed, &call_maps);
            if out.contains("# TODO") {
                todos += 1;
            }
            output.push_str(&format!("{indent_str}{out}\n"));
            i += 1;
            continue;
        }

        // ── Comments ──────────────────────────────────────────────
        if trimmed.starts_with('#') {
            output.push_str(&format!("{indent_str}{trimmed}\n"));
            i += 1;
            continue;
        }

        // ── library() / require() ─────────────────────────────────
        static LIB_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?:library|require)\(([^)]+?)\)").unwrap());
        if let Some(caps) = LIB_RE.captures(trimmed) {
            let pkg = caps
                .get(1)
                .map_or("", |m| m.as_str())
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            let import_line = r_package_import(pkg, &mut todos);
            output.push_str(&format!("{indent_str}{import_line}\n"));
            i += 1;
            continue;
        }

        // ── Multi-line strings / heredoc (rare in R) ──────────────
        // R doesn't have heredocs commonly, skip

        // ── Assignment: x <- expr  (always) or x = expr (top-level only) ──
        // Two separate patterns: <- is unambiguous; = is only assignment at
        // the top level (paren_depth == 0, already guaranteed above).
        static ARROW_ASSIGN_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([a-zA-Z_][\w.$]*(?:\[.*?\])?)\s*<-\s*(.+)$").unwrap());
        static EQ_ASSIGN_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([a-zA-Z_][\w.$]*(?:\[.*?\])?)\s*=\s*(.+)$").unwrap());
        let assign_re_match = ARROW_ASSIGN_RE
            .captures(trimmed)
            .or_else(|| EQ_ASSIGN_RE.captures(trimmed));
        static RARROW_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(.+)\s*->\s*([a-zA-Z_]\w*)$").unwrap()); // expr -> var

        if let Some(caps) = RARROW_RE.captures(trimmed) {
            // R rightward assignment: expr -> var
            let expr = transform_r_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
            let varname = caps.get(2).map_or("", |m| m.as_str());
            output.push_str(&format!("{indent_str}let {varname} = {expr}\n"));
            if expr.contains("# TODO") {
                todos += 1;
            }
            i += 1;
            continue;
        }

        if let Some(caps) = assign_re_match {
            let varname = caps.get(1).map_or("", |m| m.as_str());
            let rhs = caps.get(2).map_or("", |m| m.as_str()).trim();

            // function() definition on RHS → fn
            static FUNC_RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^function\s*\(([^)]*)\)\s*\{?(.*)$").unwrap());
            if let Some(fcaps) = FUNC_RE.captures(rhs) {
                let args = fcaps.get(1).map_or("", |m| m.as_str());
                let body_start = fcaps.get(2).map_or("", |m| m.as_str()).trim();
                if body_start.is_empty() || body_start == "{" {
                    output.push_str(&format!("{indent_str}fn {varname}({args}) {{\n"));
                } else {
                    // Inline function: fn varname(args) -> body
                    let body =
                        transform_r_expr(body_start.trim_end_matches('}').trim(), &call_maps);
                    output.push_str(&format!(
                        "{indent_str}let {varname} = fn({args}) -> {body}\n"
                    ));
                }
                i += 1;
                continue;
            }

            // $-access → .
            let rhs_t = transform_r_expr(rhs, &call_maps);
            if rhs_t.contains("# TODO") {
                todos += 1;
            }

            // Attribute assignment: df$col <- val → # TODO
            if varname.contains('$') {
                let varname_t = varname.replace('$', ".");
                output.push_str(&format!(
                    "{indent_str}# TODO: attribute assignment {varname_t} = {rhs_t}\n"
                ));
                todos += 1;
            } else {
                output.push_str(&format!("{indent_str}let {varname} = {rhs_t}\n"));
            }
            i += 1;
            continue;
        }

        // ── if ────────────────────────────────────────────────────
        static IF_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^if\s*\((.+)\)\s*\{?$").unwrap());
        if let Some(caps) = IF_RE.captures(trimmed) {
            let cond = transform_r_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
            output.push_str(&format!("{indent_str}if {cond} {{\n"));
            i += 1;
            continue;
        }

        // ── } else if ─────────────────────────────────────────────
        static ELIF_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\}\s*else\s+if\s*\((.+)\)\s*\{?$").unwrap());
        if let Some(caps) = ELIF_RE.captures(trimmed) {
            let cond = transform_r_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
            output.push_str(&format!("{indent_str}}} else if {cond} {{\n"));
            i += 1;
            continue;
        }

        // ── } else ────────────────────────────────────────────────
        if trimmed == "} else {" || trimmed == "} else{" || trimmed == "}else{" {
            output.push_str(&format!("{indent_str}}} else {{\n"));
            i += 1;
            continue;
        }
        if trimmed == "else {" || trimmed == "else{" {
            output.push_str(&format!("{indent_str}}} else {{\n"));
            i += 1;
            continue;
        }

        // ── for ───────────────────────────────────────────────────
        static FOR_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^for\s*\((\w+)\s+in\s+(.+?)\)\s*\{?$").unwrap());
        if let Some(caps) = FOR_RE.captures(trimmed) {
            let var = caps.get(1).map_or("", |m| m.as_str());
            let iter = transform_r_expr(caps.get(2).map_or("", |m| m.as_str()), &call_maps);
            output.push_str(&format!("{indent_str}for {var} in {iter} {{\n"));
            i += 1;
            continue;
        }

        // ── while ─────────────────────────────────────────────────
        static WHILE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^while\s*\((.+?)\)\s*\{?$").unwrap());
        if let Some(caps) = WHILE_RE.captures(trimmed) {
            let cond = transform_r_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
            output.push_str(&format!("{indent_str}while {cond} {{\n"));
            i += 1;
            continue;
        }

        // ── function definition ───────────────────────────────────
        // Standalone: function(args) { — already caught above in assignment
        static FN_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^function\s*\(([^)]*)\)\s*\{?$").unwrap());
        if let Some(caps) = FN_RE.captures(trimmed) {
            let args = caps.get(1).map_or("", |m| m.as_str());
            output.push_str(&format!("{indent_str}fn({args}) {{\n"));
            i += 1;
            continue;
        }

        // ── Pipe operator: %>% → |> ───────────────────────────────
        // tryCatch → # TODO
        if trimmed.starts_with("tryCatch(") {
            todos += 1;
            output.push_str(&format!(
                "{indent_str}# TODO: tryCatch not in BioLang — use result/error pattern\n"
            ));
            i += 1;
            continue;
        }

        // ── return ────────────────────────────────────────────────
        static RET_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^return\((.+?)\)$").unwrap());
        if let Some(caps) = RET_RE.captures(trimmed) {
            let expr = transform_r_expr(caps.get(1).map_or("", |m| m.as_str()), &call_maps);
            output.push_str(&format!("{indent_str}{expr}\n"));
            i += 1;
            continue;
        }
        if trimmed == "return()" || trimmed == "return(invisible(NULL))" {
            output.push_str(&format!("{indent_str}null\n"));
            i += 1;
            continue;
        }

        // ── Closing brace ─────────────────────────────────────────
        if trimmed == "}" || trimmed == "}," {
            output.push_str(&format!("{indent_str}}}\n"));
            i += 1;
            continue;
        }

        // ── General expression / function call ────────────────────
        let out = transform_r_expr(trimmed, &call_maps);
        if out.contains("# TODO") {
            todos += 1;
        }
        output.push_str(&format!("{indent_str}{out}\n"));
        i += 1;
    }

    // Footer
    if todos > 0 {
        output.push_str(&format!(
            "\n# Conversion complete: {} TODO item(s) require manual attention.\n",
            todos
        ));
    } else {
        output.push_str("\n# Conversion complete: no TODO items — review output before running.\n");
    }

    output
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn leading_spaces(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 2,
            _ => break,
        }
    }
    count
}

fn transform_r_expr(expr: &str, maps: &[CallMap]) -> String {
    let mut result = expr
        .replace("TRUE", "true")
        .replace("FALSE", "false")
        .replace("NULL", "null")
        .replace("NA_character_", "null")
        .replace("NA_integer_", "null")
        .replace("NA_real_", "null")
        .replace("NA", "null")
        .replace("Inf", "infinity")
        .replace("-Inf", "-infinity")
        .replace("NaN", "null")
        .replace(" <- ", " = ")
        .replace("<<-", "=") // global assignment → regular
        .replace("%in%", "|> contains") // TODO: imperfect
        .replace("%>%", "|>") // magrittr pipe
        .replace("|>", "|>") // native R pipe (same)
        .replace("$", "."); // R $ accessor → BioLang .

    for map in maps {
        result = map.apply(&result);
    }
    result
}

/// Map an R package name to a BioLang import statement or comment.
fn r_package_import(pkg: &str, todos: &mut usize) -> String {
    match pkg {
        "DESeq2" => "import differential.deseq as de".to_string(),
        "edgeR" => { *todos += 1; "# TODO: edgeR not yet in BioLang — coming soon".to_string() }
        "limma" => { *todos += 1; "# TODO: limma/voom not yet in BioLang — coming soon".to_string() }
        "survival" => "import survival.km as km\nimport survival.cox as cox".to_string(),
        "survminer" => "# survminer plots → km.plot(), cox.plot()".to_string(),
        "Biostrings" => "# Biostrings: dna(), rna(), protein(), gc_content(), reverse_complement() are builtins".to_string(),
        "GenomicRanges" => "# GenomicRanges: interval(), bed_intersect(), bed_subtract(), flank() are builtins".to_string(),
        "IRanges" => "# IRanges: interval() builtin".to_string(),
        "SummarizedExperiment" => { *todos += 1; "# TODO: SummarizedExperiment not yet in BioLang — use table()".to_string() }
        "SingleCellExperiment" => "# SingleCellExperiment: SCE class → BioLang uses Record; counts/logcounts/colData/rowData patterns converted".to_string(),
        "clusterProfiler" => "import pathway.ora as ora\nimport pathway.gsea as gsea".to_string(),
        "enrichplot" => "# enrichplot → use scatter()/bar() builtins for enrichment visualization".to_string(),
        "Seurat" => "# Seurat: singlecell builtins — normalize_total, log1p_transform, high_var_genes, pca, umap, knn_graph".to_string(),
        "scater" => "# scater: cell_qc(), gene_qc() are builtins".to_string(),
        "scran" => "# scran: use normalize_total, highly_variable_genes, knn_graph builtins".to_string(),
        "ggplot2" => "# ggplot2: use bar(), scatter(), heatmap(), line_plot(), violin() builtins".to_string(),
        "ggrepel" => "# ggrepel: labels handled automatically in BioLang plots".to_string(),
        "dplyr" => "# dplyr: filter(), select(), group_by(), summarize(), join() are builtins".to_string(),
        "tidyr" => "# tidyr: pivot_wider(), pivot_longer(), drop_na(), fill_na() are builtins".to_string(),
        "tidyverse" => "# tidyverse: most ops available as BioLang builtins — filter/select/group_by/join/pivot".to_string(),
        "purrr" => "# purrr: map(), filter(), reduce() are builtins".to_string(),
        "stringr" => "# stringr: split(), trim(), to_upper(), match_re(), replace_re() are builtins".to_string(),
        "readr" => "# readr: read_csv(), read_tsv() are builtins".to_string(),
        "tibble" => "# tibble: use table() in BioLang".to_string(),
        "pheatmap" => "# pheatmap: use heatmap() builtin".to_string(),
        "ComplexHeatmap" => { *todos += 1; "# TODO: ComplexHeatmap not yet supported — use heatmap()".to_string() }
        "EnhancedVolcano" => { *todos += 1; "# TODO: EnhancedVolcano not yet supported — use volcano_plot()".to_string() }
        "WGCNA" => { *todos += 1; "# TODO: WGCNA not yet in BioLang".to_string() }
        "SNPRelate" | "gdsfmt" => { *todos += 1; "# TODO: SNPRelate not yet in BioLang".to_string() }
        "rtracklayer" => "# rtracklayer: read_bed(), read_gff(), read_bedgraph(), write_bedgraph() are builtins".to_string(),
        "Rsamtools" => "# Rsamtools: read_bam(), read_sam(), depth(), insert_size(), mapping_rate() are builtins".to_string(),
        "BSgenome" => { *todos += 1; "# TODO: BSgenome not yet in BioLang — use ncbi_fetch() for sequences".to_string() }
        "AnnotationDbi" | "org.Hs.eg.db" | "org.Mm.eg.db" => {
            *todos += 1;
            "# TODO: AnnotationDbi not yet in BioLang — use ncbi_gene(), ensembl_gene() for annotation".to_string()
        }
        "glmnet" => { *todos += 1; "# TODO: glmnet (lasso/ridge) not yet in BioLang".to_string() }
        "MAST" => "# MAST: single-cell DE — use diff_expr() builtin or import \"differential\" as de".to_string(),
        "monocle3" | "monocle" => "# monocle3: use trajectory builtins + import \"singlecell\" as sc".to_string(),
        "slingshot" => "# slingshot: use diffusion_pseudotime() builtin".to_string(),
        "Signac" => "import \"atac\" as atac  # TF-IDF, LSI, depth correlation, fragment counting".to_string(),
        "ChIPseeker" => "# ChIPseeker: peak_annotation(), consensus_peaks() are builtins — import \"chipseq\" as chip".to_string(),
        "DiffBind" => "# DiffBind: frip_score(), merge_peaks(), consensus_peaks(), diff_expr() are builtins — import \"chipseq\" as chip".to_string(),
        "minfi" | "methylKit" => { *todos += 1; "# TODO: methylation analysis not yet in BioLang".to_string() }
        "phyloseq" => "# phyloseq: alpha_diversity(), beta_diversity(), rarefaction(), taxonomic_collapse() are builtins — import \"microbiome\" as mb".to_string(),
        "vegan" => "# vegan: alpha_diversity(), beta_diversity(), rarefaction() are builtins — import \"microbiome\" as mb".to_string(),
        "microbiome" => "# microbiome pkg: use import \"microbiome\" as mb — alpha_diversity(), relative_abundance() are builtins".to_string(),
        "DADA2" | "dada2" => { *todos += 1; "# TODO: DADA2/dada2 ASV pipeline not yet in BioLang — coming soon".to_string() }
        "tximport" => "# tximport: parse_salmon(), parse_featurecounts(), size_factors() are builtins — import \"rnaseq\" as rna".to_string(),
        "tximeta" => "# tximeta: parse_salmon() builtin handles salmon output — import \"rnaseq\" as rna".to_string(),
        "ape" => "# ape: nw_parse(), tree_leaves(), nw_to_distance_matrix(), upgma() are builtins — import \"phylo\" as ph".to_string(),
        "phangorn" => "# phangorn: upgma(), nw_to_distance_matrix() are builtins; ML trees → TODO — import \"phylo\" as ph".to_string(),
        "ggtree" => "# ggtree: use tree_leaves() + scatter()/heatmap() for tree-adjacent plots".to_string(),
        "vcfR" => "# vcfR: vcf_parse(), vcf_filter(), titv_ratio(), allele_freq(), variant_summary() are builtins — import \"variants\" as v".to_string(),
        "VariantAnnotation" => "# VariantAnnotation: vcf_parse(), vcf_filter(), variant_summary(), normalize_variant() are builtins — import \"variants\" as v".to_string(),
        "DECIPHER" => { *todos += 1; "# TODO: DECIPHER not yet in BioLang".to_string() }
        "parallel" | "foreach" | "doParallel" => "# parallel: use await_all() for parallel tasks in BioLang".to_string(),
        "stats" | "base" | "utils" | "grDevices" | "methods" => {
            "# base R: builtins covered — mean/stdev/t-test/lm/etc.".to_string()
        }
        "Matrix" => "# Matrix: use matrix() and sparse_matrix() builtins".to_string(),
        "MASS" => "# MASS: basic stats covered; lda/qda not yet in BioLang".to_string(),
        "jsonlite" => "# jsonlite: json_parse(), json_dump() are builtins".to_string(),
        "yaml" => "# yaml: use json_parse() for structured data".to_string(),
        _ => {
            *todos += 1;
            format!("# TODO: library({pkg}) — check if BioLang builtins cover this package")
        }
    }
}
