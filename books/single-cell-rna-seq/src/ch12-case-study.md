# A Tumor Microenvironment Case Study

## The question

Imagine a research team asking: "Can we recover the major populations in a
small tumor-like count matrix, and are the clusters stable enough to support
marker review?"

This is an exploratory engineering and analysis question. The synthetic fixture
has known truth, so it can test workflow plumbing. It cannot validate a clinical
claim.

## Complete script

Save this as `singlecell_book_case_study.bl` in the `packages` directory after
generating `nsclc_like`.

> Requires CLI: this complete example imports the package and reads/writes local
> files.

```biolang
import "singlecell" as sc

let raw = sc.load("nsclc_like")
println("raw: " + str(raw.n_cells) + " cells x " + str(raw.n_genes) + " genes")

let obj = raw
    |> sc.filter_genes(3)
    |> sc.filter_cells(20, 2500, 5.0)
    |> sc.normalize(10000.0)
    |> sc.variable_genes(100)
    |> sc.run_pca(20)
    |> sc.neighbors(15)
    |> sc.cluster_leiden(15, 0.5)

println("kept: " + str(obj.n_cells) + " cells")
println("clusters: " + str(obj.clusters |> unique |> len))
println(sc.summary(obj))

write_text("elbow.svg", sc.plot_elbow(obj, 15))
write_text("proportions.svg", sc.plot_proportions(obj))

let truth_rows = read_lines("nsclc_like/truth.csv")
    |> drop(1)
    |> filter(|line| len(line) > 0)
    |> map(|line| split(line, ","))
let truth_barcodes = truth_rows |> map(|row| row[0])
let truth_labels = truth_rows |> map(|row| int(row[1]))
let expected = obj.barcodes
    |> map(|barcode| truth_labels[find_index(truth_barcodes, |x| x == barcode)])

println("ARI vs fixture truth: " + str(round(ari(obj.clusters, expected), 4)))

write_text("case-study-umap.svg", sc.plot_umap(obj, "Tumor-like teaching data"))
write_csv(
    table(range(0, obj.n_cells) |> map(|i| {
        barcode: obj.barcodes[i],
        cluster: obj.clusters[i],
        truth: expected[i]
    })),
    "case-study-labels.csv"
)
```

## Expected interpretation

With the current implementation and fixture, 220 cells remain, four clusters
are found, and adjusted Rand index (ARI) against the known partition is 1.0000.
ARI compares partitions while ignoring arbitrary numeric cluster names.

This means the seeded populations are cleanly recovered. It does not mean:

- every real tumor separates into four populations;
- resolution `0.5` is universally correct;
- the marker simulation resembles every technology;
- BioLang, Scanpy, and Seurat are identical for all datasets;
- the result says anything about a real patient.

## Review it as four professionals

**Biologist:** Are marker blocks coherent, and what real lineages would require
positive and negative evidence?

**Researcher:** Would alternate QC thresholds, `k`, and resolution retain the
partition?

**Programmer:** Does sparse storage remain bounded, and where does exact kNN
become expensive?

**Clinician:** What additional cohort, assay, endpoint, and governance would be
required before this could affect care?

## Extend the case study

Run resolutions `0.2`, `0.5`, `0.8`, and `1.2`. Compare cluster counts, ARI,
marker coherence, and whether small clusters contain high-mitochondrial or
high-doublet-score cells. Record all outcomes rather than retaining only the
most attractive map.
