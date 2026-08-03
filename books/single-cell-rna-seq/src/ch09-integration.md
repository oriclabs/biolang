# Multiple Samples and Batch Effects

## Merge first, correct only when needed

Multiple samples are essential for population-level claims. They also introduce
technical variation from preparation day, chemistry, lane, sequencing depth,
and operator.

Begin by preserving sample identity. Merge objects, inspect them without
correction, and ask whether technical factors dominate relevant structure.

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let sample_a = sc.from_matrix(
    matrix([[8, 0, 1], [7, 0, 2], [0, 8, 1]]),
    ["A_MARKER", "B_MARKER", "HOUSEKEEPING"],
    ["A_1", "A_2", "A_3"]
)
let sample_b = sc.from_matrix(
    matrix([[9, 0, 1], [0, 9, 2], [0, 7, 1]]),
    ["A_MARKER", "B_MARKER", "HOUSEKEEPING"],
    ["B_1", "B_2", "B_3"]
)

let merged = sc.merge(sample_a, sample_b, 0, 1)
println(sc.summary(merged))
```

Merge requires aligned genes. Real projects should harmonize feature IDs and
genome annotations before this step.

## What integration means here

BioLang's current `sc.integrate()` performs deterministic batch centering in
PCA space:

> Requires CLI: this example imports the package.

```biolang
let prepared = merged
    |> sc.normalize()
    |> sc.variable_genes(3)
    |> sc.run_pca(2)

let corrected = sc.integrate(prepared, [0, 0, 0, 1, 1, 1])
let embedding = corrected.integrated_embedding
println(str(len(embedding)) + " cells x " + str(len(embedding[0])) + " components")
```

This is lightweight and inspectable. It is not Harmony, fastMNN, Seurat
anchors, scVI, or a proof that batches are removed. Use it when a simple
location shift is a reasonable model. For complex studies, validate a more
appropriate integration method externally or add a tested package.

## Do not correct away the question

If all diseased samples are batch A and all controls are batch B, disease and
batch cannot be separated computationally. Integration may erase disease or
preserve batch, and neither outcome can repair the design.

Evaluate integration by asking:

- Do shared known populations mix across batches?
- Are batch-specific quality problems still visible?
- Are known biological differences retained?
- Do marker patterns remain coherent?
- Are rare populations forced into an unrelated reference?

## Composition is also biology

A treatment can change the proportion of a cell type without strongly changing
expression inside that type. Count cells per sample and type, but perform
inference with a model appropriate for compositional, replicate-level data.
Do not run a simple test on pooled cells and call it a patient-level result.
