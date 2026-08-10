# What differs from the course

A companion that claims to match and then quietly substitutes something weaker
is worse than one that lists its gaps. Here they are.

> **Current state, stated plainly.** BioLang's single-cell pipeline was
> substantially rewritten to match the published Seurat methods: the PCA (which
> was returning components out of order — a real defect), the neighbour graph
> (now SNN with Jaccard weights), Louvain (now with the reference's ten
> restarts), and normalization (now genuinely regularized negative binomial
> rather than a fixed overdispersion).
>
> On the course data, the current Harmony example reports **18 clusters with
> GPU auto-detection and 16 with `--no-gpu`**. The corrected CCA validation
> notebook reports 15 on the measured GPU backend. Its CPU backend has not been
> rerun since anchor and downstream PCA dimensions were separated.
> The historical rendered HBC object has 17 clusters; an independent current
> Seurat CCA run described below has 19.
>
> The numbers quoted further down this page — 17, then 16 — come from earlier
> stages of that work and are kept because the *reasoning* about them is still
> the point. But do not read them as current output.
>
> The honest implementation trajectory was 17, 16, 25, 20, 21, then **18 on
> GPU / 16 on CPU** after the current backend work. That is evidence a single
> cluster count was never a sound target. The independent Seurat run now gives
> a real comparison: all 29,629 cells join exactly, but the corrected BioLang
> GPU CCA run versus Seurat has ARI 0.5276 and 65.66% optimally mapped-cell
> accuracy. Agreement claims below are based on those cell-level measurements,
> not on matching an integer.

## The lesson map

Fourteen lessons, seven chapters. Course lessons that exist to set up an R
environment have no BioLang counterpart and are folded into their neighbours.

| HBC lesson | Covered in | Status |
|---|---|---|
| 01 Intro to scRNA-seq | [The Biology and the Matrix](ch01-biology-and-matrix.md) | Full |
| 02 Generation of the count matrix | [The Biology and the Matrix](ch01-biology-and-matrix.md) | Read-only |
| 03 Quality control setup | [Quality Control](ch02-quality-control.md) | Folded in |
| 04 Cell Ranger QC | [Quality Control](ch02-quality-control.md) | Partial |
| 05 Quality control | [Quality Control](ch02-quality-control.md) | Full |
| 06 Theory of PCA | [Normalization and PCA](ch03-normalization-pca.md) | Full |
| 07 SCTransform | [Normalization and PCA](ch03-normalization-pca.md) | Full |
| 08 Integration: CCA theory | [Integration](ch04-integration.md) | Theory only |
| 09 Integration: Harmony | [Integration](ch04-integration.md) | Full |
| 10 Clustering | [Clustering](ch05-clustering.md) | Full |
| 11 Clustering quality control | [Clustering](ch05-clustering.md) | Full |
| 12 Seurat cheatsheet | [Markers and Annotation](ch06-markers.md) | Translated |
| 13 Marker identification | [Markers and Annotation](ch06-markers.md) | Full |
| 14 The whole workflow | [The Whole Workflow](ch07-workflow.md) | Full |

## The numbers will not match exactly

Same data, same thresholds, different implementations. Expect agreement on
**shape** and disagreement on **digits**.

**The pipelines are not identical**, and the differences are specific:

| | This book | The course |
|---|---|---|
| QC filter | gene floor + mito cap | + UMI floor + complexity |
| Cells kept (ctrl) | 15,049 | **14,847** with their full filter |
| Normalization | `normalize` (CP10K + log1p) | SCTransform |
| PCs | 30 | 40 |
| Clustered on | one sample at a time | the integrated object |

The course also clusters at **resolution 0.8**, where this book uses 0.5.

## Can you align them? Close — and this is where an earlier claim broke

None of those differences are limitations. BioLang has `sc.sctransform`, takes
any PC count, and the filter can be written by hand. Matching the course's
dimensions and filters while using its documented Harmony alternative — both
samples, their four-criterion filter, SCTransform, 40 PCs, resolution 0.8 —
gives:

```text
mode          cells   clusters   wall time
GPU auto      29629      18        71.9 s
--no-gpu      29629      16        72.5 s
HBC HTML                  17
```

**The course's marker lesson assigns identities to clusters 0 through 16: 17
clusters.** That is a historical output, not a timeless Seurat truth. The
independent HBC CCA workflow run with Seurat 5.5.1 produced 19 clusters at the
same selected resolution. Large-data neighbour ranking deliberately uses
different BioLang GPU and CPU backends, so cells near a graph boundary can
move; the run header now prints the selected backend before analysis.

The executable CCA validation notebook is the direct ground-truth comparison.
The corrected GPU run produced 15 clusters versus Seurat's 19. Against Seurat
it has ARI 0.5276, adjusted mutual information 0.6924, and 65.66% one-to-one
mapped accuracy. The shared variable-feature set is 2,596 of 3,000 genes
(86.53%). Its approximate integrated-PC 15-neighbor Jaccard is only 0.0648.
Those measurements establish partial agreement, not numerical parity. The CPU
backend was not rerun after the 30-anchor/50-PCA correction, so earlier CPU
figures are historical rather than current comparison results.

### Why older versions of this page said 17, then 16

Not wrong in the sense of a mistyped number — the run really did print 17. It
was wrong in the sense that mattered: **the 17 depended on selecting variable
genes with a method that does not belong on these values**, and it stopped being
17 the moment that was corrected.

The pipeline was `sctransform()` followed by `variable_genes(3000)`.
`variable_genes` ranks by dispersion — variance divided by squared mean — which
is the standard heuristic for log-normalized counts. Pearson residuals are not
log-normalized counts. They are *centred*: `mu` is fitted to the row and column
margins, so each gene's mean residual sits near zero by construction. Dividing
by the square of a near-zero number does not rank genes by variability; it ranks
them by how close their mean landed to zero, which is arithmetic noise.

`sc.sctransform(3000)` now selects on **residual variance**, which is what
[the sctransform paper](https://doi.org/10.1186/s13059-019-1874-1) proposes and
what Seurat's `SCTransform` returns.

The following is a historical controlled comparison. Both rows were run against
the same earlier binary, on the same cells, with everything downstream held
fixed — same PCA, same Harmony, same Leiden, same resolution. Only the ranking
differed:

| Gene selection on the residuals | Clusters | Time | Peak memory |
|---|---|---|---|
| Dispersion — variance ÷ mean² | **17** | 247 s | 16.8 GB |
| Residual variance — the paper's | **16** | 206 s | 6.3 GB |

That experiment established why the 17 was not valid evidence of agreement:
the defensible method gave 16 on that binary, while the wrong method happened
to give 17. **Getting the reference's number out of a method the reference does
not use is not agreement.** Later PCA, SNN, and SCTransform changes moved the
current measured result to 18; the historical table is retained as evidence for
the one-variable experiment, not presented as today's GPU output. Its 16 is
also not the current CPU result by provenance, even though the number happens
to be the same.

Note that the wrong method is the slower and hungrier one too. Nothing about the
17 was cheap.

The current mismatch is real. Candidates are the HVG overlap, the integration
approximation, PCA, neighbour-search backend, SNN details, and optimizer
tie-breaking, all of which differ; which accounts for it is not something the
cluster count alone can say.

For contrast, on the control sample alone:

```text
cells: 14847
                                     GPU auto   --no-gpu
SCTransform / 40 PCs / res 0.8          15         16
log1p / 30 PCs / res 0.5                 8          8
```

Same cells, same code, two parameter sets. The difference was the pipeline, not
the implementation. These values were rerun after SNN became the default; the
older 12/10 values were pre-SNN and are no longer presented as current output.

`aligned.bl` in [Downloads](downloads.md) runs the single-sample comparison;
`exact.bl` runs the full integrated configuration. On the measured release
build, GPU-auto runs took 34.9 and 71.9 seconds respectively; CPU-only runs took
33.0 and 72.5 seconds.

### Runtime regression recheck

An earlier integrated run regressed from 23 to 69 minutes, with the PCA's
300-sweep ceiling and a near-degenerate spectrum suspected but not measured.
That concern was rerun rather than silently retired. On 9 August 2026,
`exact.bl` completed in 71.9 seconds wall time and reproduced 18 clusters. An
instrumented second run completed in 70.4 seconds with this breakdown:

| Stage | Time |
|---|---:|
| Load and QC | 12.1 s |
| Merge and gene filter | 0.9 s |
| SCTransform | 12.3 s |
| PCA | 13.3 s |
| Harmony | **27.4 s** |
| Neighbours | 2.4 s |
| Leiden | 2.0 s |

This was the release build with GPU auto-detection selecting an NVIDIA RTX
3080. PCA is not the dominant stage on this dataset. For inputs above 5,000
cells, its current implementation fits on 5,000 deterministic rows and uses at
most six subspace sweeps; the `MAX_SWEEPS = 300` path applies only to smaller
inputs. Its convergence check follows Rayleigh quotients, so free rotation of
basis vectors inside a near-degenerate subspace does not itself prevent
convergence. The old 69-minute regression therefore does not reproduce in the
current pipeline, while the explicit record here keeps it from disappearing as
an unexamined assumption. A separate `--no-gpu` run completed in 72.5 seconds
and produced 16 clusters; this backend-sensitive result is documented rather
than folded into a single inferred number.

### What this cost to make possible

Three memory problems, in the order they surfaced.

**The integrated run died outright.** `sc_sctransform` was paying for its output
three times: a dense copy of the sparse input, a second array for the residuals,
and then every element boxed into a `Value` inside nested lists — about
4 + 4 + 12 GB. Pearson residuals are dense by construction, so 4 GB is real; the
other 16 were not. It now streams the sparse input into one flat array and
returns a matrix.

**It then died more politely, at 3.95 GB.** Residuals were still being computed
for all 16,681 genes when the next step keeps 3,000 and discards the rest.
`sc.sctransform(3000)` ranks genes by residual variance and materialises only
those, which is what `SCTransform(variable.features.n = ...)` does. Measured on
this pipeline: **16.8 GB peak uncapped, 6.3 GB capped**, and 247 s down to 206 s.

**And the 6.3 GB was still too much.** `Value::Matrix` held its matrix inline
while `Value::SparseMatrix` had already been moved behind an `Arc`, so every
record spread and every pipeline stage deep-copied the whole thing. Peak was
9.0 GB before that fix and 6.3 GB after, with the result unchanged. It was the
same bug, in the same file, as the one that had made reading a single gene take
seven minutes — just on the other matrix type.

So the honest historical sequence is: the numbers did not match; three separate
memory bugs were in the way; fixing them let the controlled comparison run; and
that comparison said 16, not the 17 the page had been claiming. The separately
rerun current pipeline now says 18, as recorded above.

### What still will not match

Cluster **numbering** is arbitrary in both — cluster 7 here is not cluster 7
there. Beyond that, HVG selection differs, the PCA is a different
implementation, and Leiden breaks ties differently, so the boundaries between
adjacent clusters will not be identical cell for cell.

And note what the 17 episode above demonstrates: **a matching cluster count was
never strong evidence of a matching partition.** One number agreeing between two
pipelines with different gene selection, different PCA and different community
detection is a weak signal, and it turned out here to be a coincidence produced
by a method error. Treat count agreement as a sanity check that nothing is
catastrophically wrong, not as validation.

This book's chapters keep the simpler settings, because log-normalization is
easier to explain and fifteen seconds beats three and a half minutes while you
are learning the shape of the workflow. That is a teaching choice, and now an
explicit one with the cost measured.

## The three real gaps

**Lesson 02 — generating the count matrix.** This covers Cell Ranger:
demultiplexing, barcode correction, alignment, UMI collapsing. BioLang does none
of it, and neither does Seurat — it is upstream of both. BioLang starts where
Cell Ranger stops. The chapter explains what happened upstream, because you
cannot interpret a UMI count without knowing what a UMI is, but it does not
pretend to run it.

**Lesson 04 — Cell Ranger's own QC report.** The course reads the
`web_summary.html` Cell Ranger emits. BioLang has no parser for it. Nearly all
of the same quantities can be recomputed from the matrix, and the QC chapter
does — but **sequencing saturation** and **fraction of reads mapped to the
transcriptome** are properties of the reads, and the reads are gone by the time
you have a matrix. If you have the file, read it in Cell Ranger's viewer.

**Lesson 08 — CCA at realistic scale.** BioLang has a `cca` builtin and the
theory is runnable on small inputs. But Seurat's CCA works on a cells × cells
cross-product, and BioLang's `Matrix::svd` is O(n⁴) — it stalls above roughly a
hundred cells. So the theory is demonstrable and the practice is not. **Use
Harmony**, which is what lesson 09 does anyway and which BioLang implements at
full scale.

## Things this book found that the course cannot tell you

Running the course's data through a different implementation is a good way to
find bugs in the implementation. Several turned up while this book was written,
and they are worth knowing about because they shape what you should trust:

- **UMAP produced a featureless blob for any input.** The layout had attraction
  but no reachable repulsion. Fixed, with tests. If you are running an older
  build and your embedding looks like a single cloud, this is why — and note
  that the clustering and marker tables were correct throughout. **If your
  embedding and your markers disagree, believe the markers.**
- **Reading one gene across 15,000 cells took seven minutes**, because the
  sparse count matrix was deep-copied on every value clone. Fixed; it is now
  free.
- **Plotting the raw droplets never finished**, because the scatter path emitted
  one vector circle per point and accumulated them quadratically. Large scatters
  are now rasterised.
- **Violin plots crashed on unevenly sized clusters.** The padding value for
  ragged columns was `-inf`, which is not valid JSON. An evenly sized synthetic
  fixture never triggered it; real clusters ranging from 3,714 cells to 32 do
  immediately.
- **Dense matrices were deep-copied on every clone**, exactly as the sparse ones
  had been before that was fixed. Worth 2.7 GB on this pipeline. If you fix a
  bug of this kind, check whether its sibling has it too — here, nobody did for
  months.
- **`sc.sctransform` ignored its own second argument**, because the package
  facade forwarded one argument to a function that now took two. It was silent
  because **BioLang discards extra arguments to a user-defined function without
  complaining** — `f(1, 2, 3)` on a one-parameter `f` returns quietly. That is
  a language-level defect and it is still open; until it is fixed, a typo in an
  argument list is invisible.

The last two are the general lesson, and it is not the one about synthetic data.
Synthetic data tests the code you wrote and real data tests the assumptions you
did not know you had made — but both of those bugs were found by *running the
thing and watching a number that should not have moved*. Neither a test suite
nor a bigger dataset would have surfaced them.
