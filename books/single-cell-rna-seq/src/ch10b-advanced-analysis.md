# Advanced Analysis and Diagnostic Plots

Advanced analysis does not mean applying more algorithms. It means testing a
biological question at the correct level, showing where the result came from,
and checking whether reasonable analysis choices change the conclusion.

The complete runnable example for this chapter is distributed with the package:

```text
bl install singlecell
bl examples singlecell --copy singlecell-examples
cd singlecell-examples
bl run advanced_analysis.bl
```

Without the install step the copied directory has no package to import and
every script fails with `module or plugin 'singlecell' not found`.

It creates `singlecell-results/` with SVG figures, differential-expression
tables, composition tests, and ranked genes. The fixture has four donors
measured in control and treated conditions, with T and B cells in every sample.
The code excerpts below use `obj`, `analysed`, `donors`, `conditions`, and
`cell_types` exactly as they are defined in that complete example.

## Inspect quality as a connected problem

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
write_text(
    "singlecell-results/qc-dashboard.svg",
    sc.plot_qc_dashboard(obj)
)
```

The dashboard combines library-size, detected-gene, and mitochondrial
distributions with the counts-versus-genes relationship. A threshold should be
chosen from these distributions, sample context, and protocol knowledge. It
should not be copied blindly from another tissue.

## Label an embedding by the question

Clusters are only one way to colour an embedding. Compare condition, donor,
curated cell type, batch, and cell-cycle phase:

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
write_text(
    "singlecell-results/umap-condition.svg",
    sc.plot_embedding(analysed, conditions, "UMAP by treatment")
)

write_text(
    "singlecell-results/feature-split.svg",
    sc.plot_feature_split(
        analysed, "IFIT1", conditions, "IFIT1 by treatment"
    )
)
```

The split feature plot uses the same expression scale in every panel. Separate
plots with independently chosen colour limits can manufacture an apparent
difference.

For gene panels grouped by cell type, condition, donor, or phase:

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
write_text(
    "singlecell-results/group-heatmap.svg",
    sc.plot_group_heatmap(
        analysed,
        ["IFIT1", "MS4A1", "CD3D"],
        cell_types,
        "Lineage and response genes"
    )
)
```

## Build donor-level profiles

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
let profiles = sc.pseudobulk_profiles(
    obj, donors, conditions, cell_types, min_cells: 5
)

println(profiles.profiles)
println(
    str(len(profiles.profiles)) + " profiles x " +
    str(len(profiles.genes)) + " genes"
)
```

Raw counts are summed for each donor x condition x cell-type combination.
Profiles with fewer than five cells are excluded. The returned record contains:

- `counts`: genes x pseudobulk profiles, preserving raw sums;
- `logcpm_matrix`: profiles x genes for transparent exploration and PCA;
- `profiles`: donor, condition, cell type, cell count, and library size;
- `genes`: row identifiers corresponding to the matrices.

## Run an explicit paired exploratory test

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
let de = sc.paired_pseudobulk_de(
    obj,
    donors,
    conditions,
    cell_types,
    target_type: "T",
    condition_a: "control",
    condition_b: "treated",
    min_cells: 5
)

let top = de
    |> sort(|a, b| if a.padj < b.padj { -1 } else { 1 })
    |> take(10)
println(table(top))
```

This function pairs each donor's control and treated log2-CPM profile, applies a
paired test per gene, and performs Benjamini-Hochberg correction. It is useful
for transparent exploration and teaching.

It is not a replacement for a publication-grade negative-binomial model. For
formal inference, export `profiles.counts` and use DESeq2, edgeR, or another
validated count model with an explicit design such as `~ donor + condition`.
Include batch and clinical covariates when the study design requires them.

## Use three complementary DE plots

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
write_text("singlecell-results/volcano.svg", sc.plot_volcano(de))
write_text("singlecell-results/ma.svg", sc.plot_ma(de))
write_text(
    "singlecell-results/paired-donors.svg",
    sc.plot_donor_expression(
        obj, "IFIT1", donors, conditions, cell_types,
        target_type: "T", condition_a: "control", condition_b: "treated",
        min_cells: 5
    )
)
```

- The **volcano plot** combines effect magnitude and adjusted significance.
- The **MA plot** reveals expression-dependent fold-change behaviour.
- The **paired-donor plot** shows whether the direction is shared by donors or
  driven by one sample.

A convincing volcano point with inconsistent donor trajectories deserves
investigation, not a stronger claim.

## Check samples in pseudobulk PCA

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
write_text(
    "singlecell-results/pseudobulk-pca.svg",
    sc.plot_pseudobulk_pca(
        obj, donors, conditions, cell_types, target_type: "T", min_cells: 5
    )
)
```

Each point is now a biological sample, not a cell. Colour shows condition and
labels identify donors. Inspect outliers, pairing, and whether treatment
separation is larger than donor-to-donor variation.

## Test cell-type composition at sample level

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
let composition = sc.composition_table(
    donors, conditions, cell_types
)
let composition_tests = sc.paired_composition_test(
    donors, conditions, cell_types,
    condition_a: "control", condition_b: "treated", min_cells: 10
)

write_text(
    "singlecell-results/composition.svg",
    sc.plot_composition(donors, conditions, cell_types)
)
println(table(composition_tests))
```

The table includes zero-count cell types so missing populations do not
disappear from the analysis. The paired test treats donors, rather than cells,
as replicates. Cell fractions are compositional and affected by capture and QC;
specialized compositional models remain preferable for complex studies.

## Check clustering quality and sensitivity

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
let diagnostics = sc.cluster_diagnostics(analysed)
let stability = sc.cluster_stability(
    analysed, resolutions: [0.2, 0.4, 0.6, 0.8, 1.0], k: 8
)

write_text(
    "singlecell-results/silhouette.svg",
    sc.plot_silhouette(diagnostics)
)
write_text(
    "singlecell-results/cluster-stability.svg",
    sc.plot_cluster_stability(stability)
)
```

Silhouette coefficients describe separation in the PCA or integrated
embedding. The resolution sweep reports cluster count, mean silhouette, and
adjusted Rand index against the previous resolution. No single score selects a
biologically correct resolution: combine diagnostics with marker coherence,
replication, and the question being asked.

## Rank genes and test pathways

> Requires CLI: an excerpt from `advanced_analysis.bl`. It continues that
> script's variables and cannot run on its own — run the complete example.

```biolang
let ranked = sc.ranked_genes(de)

# gsea() takes a Map, which is what read_gmt() returns. A `{...}` literal is a
# Record, not a Map, and is rejected.
let sets = read_gmt("gene-sets.gmt")
let enrichment = gsea(ranked, sets, seed: 1)

write_text(
    "singlecell-results/enrichment.svg",
    sc.plot_enrichment(enrichment)
)
```

`ranked_genes()` uses the signed effect and p-value to create the `gene, score`
table expected by BioLang's `gsea()` builtin. Gene identifiers, the tested
background, and gene-set versions must be recorded.

GSEA p-values come from a permutation test. `gsea()` uses a fixed default seed
so a rerun reproduces the same numbers; pass `seed:` to vary it deliberately,
and record the value you used. Repeating the analysis under several seeds is a
useful check that a pathway result is not an artifact of one permutation draw.

## Compare methods without a Venn diagram

> Requires CLI: a sketch, not an excerpt. `deseq2_results` and `edger_results`
> stand for tables you read back after running those tools externally — see
> the handoff in the previous chapter.

```biolang
# Each entry is a DE result list with `gene` and `significant` fields, so
# anything you read back with read_csv and reshape will do.
let deseq2_results = read_csv("results/deseq2.csv") |> to_records()
let edger_results  = read_csv("results/edger.csv")  |> to_records()

write_text(
    "singlecell-results/de-overlap.svg",
    sc.plot_de_overlap(
        [de, deseq2_results, edger_results],
        ["BioLang paired", "DESeq2", "edgeR"]
    )
)
```

The UpSet-style plot scales to more result sets than a Venn diagram. Agreement
is useful, but inspect effect directions and ranks as well as set membership.

## Minimum advanced report

An advanced report should contain:

1. sample and donor counts before and after QC;
2. QC distributions split by sample;
3. embeddings coloured by sample, condition, and annotation;
4. pseudobulk sample PCA;
5. the statistical design and replicate definition;
6. volcano, MA, and paired-donor views;
7. composition results at sample level;
8. clustering sensitivity diagnostics;
9. enrichment with gene-set provenance;
10. R or Python validation and session versions.
