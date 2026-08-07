# The HBC Course in BioLang

The Harvard Chan Bioinformatics Core teaches a fourteen-lesson introduction to
single-cell RNA-seq. It is one of the better free curricula in the field, and it
is taught in R with Seurat. This part of the book walks the same road in
BioLang.

It is a **companion, not a replacement**. Read the HBC lessons for the biology
and the reasoning; read these chapters when you want to run the same steps here.
Credit, licence, and what was changed are on the
[Attribution and licence](hbc-attribution.md) page — read that first if you plan
to reuse any of this.

## Why follow someone else's syllabus

Because the ordering is the hard part, and theirs is good.

A tool's documentation naturally organises itself around the tool: here are the
functions, here is what each one takes. A course organises itself around the
learner: here is what you cannot understand until you understand this other
thing first. HBC puts the theory of PCA *before* normalization, which looks
backwards until you notice that you cannot judge whether a normalization worked
without a way to look at the result. They give cluster quality control an entire
lesson, long after clustering, because the question "are these clusters real?"
only becomes answerable once you have some.

Chapters 1–15 of this book are organised the first way. This part is organised
the second way.

## The lesson map

Fourteen lessons, seven chapters. Course lessons that exist to set up an R
environment have no BioLang counterpart and are folded into their neighbours.

| HBC lesson | Covered in | Status |
|---|---|---|
| 01 Intro to scRNA-seq | [The Biology and the Matrix](hbc-01-setup.md) | Full |
| 02 Generation of the count matrix | [The Biology and the Matrix](hbc-01-setup.md) | Read-only — see below |
| 03 Quality control setup | [Quality Control](hbc-02-quality-control.md) | Folded in |
| 04 Cell Ranger QC | [Quality Control](hbc-02-quality-control.md) | Partial — see below |
| 05 Quality control | [Quality Control](hbc-02-quality-control.md) | Full |
| 06 Theory of PCA | [Normalization and PCA](hbc-03-normalization-pca.md) | Full |
| 07 SCTransform normalization | [Normalization and PCA](hbc-03-normalization-pca.md) | Full |
| 08 Integration: CCA theory | [Integration](hbc-04-integration.md) | Theory runs, scale does not — see below |
| 09 Integration: Harmony | [Integration](hbc-04-integration.md) | Full |
| 10 Clustering | [Clustering](hbc-05-clustering.md) | Full |
| 11 Clustering quality control | [Clustering](hbc-05-clustering.md) | Full |
| 12 Seurat cheatsheet | [Markers and Annotation](hbc-06-markers.md) | Translated to the pack API |
| 13 Marker identification | [Markers and Annotation](hbc-06-markers.md) | Full |
| 14 The whole workflow | [The Whole Workflow](hbc-07-workflow.md) | Full |

### The three gaps, stated plainly

A companion that claims full coverage and then quietly substitutes something
weaker is worse than one that admits the holes. There are three.

**Lesson 02 — generating the count matrix.** This lesson covers Cell Ranger:
demultiplexing, barcode correction, alignment, and UMI collapsing. BioLang does
not do any of it, and neither does Seurat — it is upstream of both. BioLang
starts where Cell Ranger stops, reading the matrix directory it produces. The
chapter explains what happened upstream, because you cannot interpret a UMI
count without knowing what a UMI is, but it does not pretend to run it.

**Lesson 04 — Cell Ranger's own QC report.** The course reads the `web_summary.html`
that Cell Ranger emits and interprets its metrics. BioLang has no parser for
that file. You can compute nearly all of the same quantities from the matrix
itself, and the QC chapter does, but the sequencing-level metrics — reads mapped
to the transcriptome, sequencing saturation — are not recoverable after the
fact. If you have the file, read it in Cell Ranger's viewer.

**Lesson 08 — CCA at realistic scale.** BioLang has a `cca` builtin, and this
book uses it to demonstrate the property the lesson is about: shared variation
scores highly, dataset-specific variation does not. But Seurat's CCA works on a
cells × cells cross-product, and BioLang's `Matrix::svd` is currently O(n⁴) —
it stalls above roughly a hundred cells. So the theory is runnable and the
practice is not. **Use Harmony for real integration**, which is what lesson 09
does anyway, and which BioLang implements at full scale.

## Take it offline

None of the code in this part runs in the browser: every example imports the
`singlecell` package and most write a figure to disk, and package imports and
file I/O are CLI-only. So the pages show no Run button — copy the code, or take
the starter kit.

### The starter kit (recommended)

**[singlecell-starter.zip](downloads/singlecell-starter.zip)** — 141 KB, and it
is everything: the package, the dataset already generated, every chapter script,
and the notebook. **No repository checkout and no Python required.**

```sh
# 1. Install BioLang — Linux/macOS
curl -fsSL https://lang.bio/install.sh | sh
#    Windows (PowerShell)
#    iwr -useb https://lang.bio/install.ps1 | iex

# 2. Unzip, and install the package from inside it
unzip singlecell-starter.zip && cd singlecell-starter
bl install ./singlecell

# 3. Run
bl run hbc-01-setup.bl
```

which prints:

```text
cells: 265
genes: 168
first genes:    [MARK0_000, MARK0_001, MARK0_002, MARK0_003, MARK0_004]
first barcodes: [0000AACCGGTT-1, 0001AACCGGTT-1, 0002AACCGGTT-1]
```

> **Why a local path and not a URL.** There is no package registry yet, and
> `bl install --git <url>` clones a whole repository into the package slot —
> which cannot work for a package that lives in a subdirectory of one. A local
> path is the supported route, so the kit ships the package rather than pointing
> at it.

The kit includes `make_demo_10x.py`, which regenerates the dataset if you want
to change it — more cells, more populations, a different seed. You never need to
run it: `nsclc_like/` is already generated inside the zip.

### Or take the pieces

- [hbc-companion.bln](downloads/hbc-companion.bln) — the whole part as one
  runnable notebook, 21 blocks. `bl notebook hbc-companion.bln`

**Individual chapter scripts:**

| Chapter | Script | Blocks |
|---|---|---|
| The Biology and the Matrix | [hbc-01-setup.bl](downloads/hbc-01-setup.bl) | 1 |
| Quality Control | [hbc-02-quality-control.bl](downloads/hbc-02-quality-control.bl) | 3 |
| Normalization and PCA | [hbc-03-normalization-pca.bl](downloads/hbc-03-normalization-pca.bl) | 4 |
| Integration | [hbc-04-integration.bl](downloads/hbc-04-integration.bl) | 2 |
| Clustering | [hbc-05-clustering.bl](downloads/hbc-05-clustering.bl) | 5 |
| Markers and Annotation | [hbc-06-markers.bl](downloads/hbc-06-markers.bl) | 3 |
| The Whole Workflow | [hbc-07-workflow.bl](downloads/hbc-07-workflow.bl) | 3 |

Each script is self-contained and runs with `bl run <file>`. Both forms need the
same one-time setup as below.

## Before you start

Take the starter kit above, or — if you have the repository checked out —
install from it directly and generate the fixture:

```text
bl install packages/singlecell
python packages/singlecell/examples/make_demo_10x.py --output nsclc_like
```

Either way, run everything from the directory containing `nsclc_like/`.

The fixture is 265 barcodes over 168 genes with four planted populations, from
a fixed seed. It is synthetic: no person or patient data is involved.

Then:

```biolang
import "singlecell" as sc

let raw = sc.load("nsclc_like")
println("cells: " + str(raw.n_cells))
println("genes: " + str(raw.n_genes))
```

Your numbers will not match the course's, and they are not supposed to. The
course uses a specific published PBMC dataset; this uses a small synthetic one
that runs in a browser tab. The **shapes** of the results — a bimodal UMI
distribution, an elbow in the variance plot, clusters that separate on marker
genes — are what you should compare, not the counts.

If you want to run against the course's own data, it is a 10x matrix directory
like any other, and `sc.load` takes the path to it.
