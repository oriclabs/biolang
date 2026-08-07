# Integration

*Follows HBC lessons 08 (CCA theory) and 09 (Harmony).*

The course splits integration into two lessons — one for the theory, one for the
code — and the split is worth preserving, because integration is the step where
a plausible-looking result is most likely to be wrong.

## The problem

You have two samples. You cluster them together and every cluster contains cells
from one sample only. Two readings:

1. The samples contain genuinely different cell populations.
2. A technical difference between samples is larger than the difference between
   cell types.

These look identical in a UMAP. Reading 2 is more often correct, and the
distinguishing evidence is not in the plot — it is in the design. If your
samples were processed on different days, by different people, or on different
chemistry versions, you have a batch effect. If a batch is perfectly confounded
with a condition, no algorithm can separate them and no amount of integration
will save the experiment.

## What CCA is for

Canonical Correlation Analysis is the idea behind Seurat's original integration,
and the reason it works is worth stating precisely.

PCA on either dataset alone finds the directions of greatest variance *in that
dataset*, and a batch effect is often the single largest source of variance. So
PC1 becomes the batch. CCA instead looks for directions that are **correlated
between the two datasets**. A batch effect present in only one dataset cannot
correlate with anything in the other, so it scores poorly. Shared biology —
T cells behaving like T cells in both samples — scores well.

That property is directly testable, and BioLang's `cca` builtin demonstrates it:

```biolang
import "singlecell" as sc

# Two small datasets over the same genes, the second carrying an offset that
# exists only in it. CCA should not lead with that offset.
let a = [[3.0, 0.2, 0.2], [3.1, 0.3, 0.2], [0.2, 3.0, 0.2], [0.2, 0.2, 3.0]]
let b = [[4.5, 1.7, 1.7], [4.6, 1.8, 1.7], [1.7, 4.5, 1.7], [1.7, 1.7, 4.5]]

let shared = cca(a, b, {k: 2})
println("u rows: " + str(len(shared.u)) + ", v rows: " + str(len(shared.v)))
```

> **This does not scale, and you should not plan around it.** Seurat's CCA works
> on a cells × cells cross-product, and BioLang's `Matrix::svd` is currently
> O(n⁴) — it handles a few dozen cells in a debug build and stalls above roughly
> a hundred in release. The builtin exists to make the theory runnable, not to
> integrate an experiment. For real work, use Harmony below. This is a known
> limitation of the eigensolver, not of the method.

## Harmony, which does scale

Harmony (Korsunsky et al., 2019) works in PCA space rather than gene space,
which is why it is fast. It alternates two steps:

1. **Soft cluster** the cells, with a penalty that rewards clusters containing a
   mixture of batches. A cluster that is 100% one donor is penalised; the
   penalty pushes toward clusters that represent cell types rather than samples.
2. **Correct within each cluster** by regressing out the batch, producing a
   per-cell shift.

The second step is the one that matters, and the reason is easy to miss.
Subtracting one global per-batch offset assumes the batch effect points the same
way for every cell type. It usually does not — a batch can shift T cells one way
and monocytes another. Correcting *within* clusters lets the correction differ
per cell type, which a single global offset cannot do.

```biolang
import "singlecell" as sc

let obj = sc.load("nsclc_like")
    |> sc.filter_genes(3)
    |> sc.filter_cells(20, 2500, 5.0)
    |> sc.normalize()
    |> sc.variable_genes(50)
    |> sc.run_pca(20)

# One label per cell naming the sample it came from. Here the fixture is a
# single batch, so this is a demonstration of the call, not a real correction.
let batches = range(0, obj.n_cells) |> map(|i| if i % 2 == 0 { "donorA" } else { "donorB" })
let fixed = sc.integrate(obj, batches)

println("integrated embedding for " + str(fixed.n_cells) + " cells")
```

## How to tell whether it worked

This is the part that gets skipped, and it is the part that matters.

A batch-mixing metric alone is **maximised by collapsing every cell onto a single
point**. Perfect mixing, zero biology. So mixing is never sufficient evidence.
Every check must come in a pair:

- **Batches mix** — each cluster now contains cells from every sample, in
  roughly the proportions the samples contribute.
- **Cell types stay apart** — the populations you could distinguish before
  integration are still distinguishable after.

If you only ever check the first, you cannot detect over-correction, which is
the characteristic failure of every integration method. A UMAP that looks
beautifully mixed and has quietly merged your CD4 and CD8 T cells is a worse
outcome than no integration at all, because it looks like success.

Check both. The BioLang test suite for `harmony_integrate` asserts them as a
pair for exactly this reason.

## When not to integrate

If your samples are biological replicates that were processed together and they
already overlap, integrating adds a correction where there is nothing to
correct — and every correction costs some real variation.

Cluster first without integration. If the clusters already mix, stop. Integration
is a repair, not a routine step.

## Next

With cells positioned in a corrected space, you can group them:
[Clustering](hbc-05-clustering.md).
