# What differs from the course

A companion that claims to match and then quietly substitutes something weaker
is worse than one that lists its gaps. Here they are.

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

Where this book and the course should agree, and do:

- **Cell counts after QC.** 15,049 for the control sample; the course reports
  about 15,000. The filter is landing in the same place.
- **Cell types present.** CD4 and CD8 T, NK, B, CD14 and CD16 monocytes,
  dendritic cells, platelets — the same populations with the same canonical
  markers.
- **Which genes mark which population.** CD14/LYZ/S100A8, MS4A1/CD79A,
  GNLY/NKG7, FCGR3A/MS4A7 all land where the course says they should.

Where they will not:

- **Cluster count.** This book finds 11 in the control sample at resolution 0.5.
  Different HVG selection, different PCA implementation and a different community
  detection tie-breaking all shift where the boundaries fall.
- **Cluster numbering.** Arbitrary in both. Cluster 3 here is not cluster 3
  there.
- **UMAP coordinates.** Seeded differently, and not meaningful as coordinates
  anyway.
- **Exact p-values and fold changes.** Same tests, different implementations.

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

The last one is the general lesson. Synthetic data tests the code you wrote;
real data tests the assumptions you did not know you had made.
