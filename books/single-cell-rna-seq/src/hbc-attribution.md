# Attribution and licence

## The source

This part of the book follows the curriculum of:

> **Introduction to Single-cell RNA-seq**
> Harvard Chan Bioinformatics Core (HBC)
> Mary Piper, Meeta Mistry, Radhika Khetani, Lorena Pantano, Jihe Liu,
> Will Gammerdinger, and Noor Sohail
> <https://hbctraining.github.io/Intro-to-scRNAseq/>

The course is the work of the HBC training team. **The teaching sequence is
theirs** — the decision to explain PCA before normalization, to separate the
theory of integration from the code that runs it, to give cluster quality
control a lesson of its own rather than a paragraph, and to close with the whole
workflow written out end to end. That ordering is the most valuable thing in the
course, and it is the thing this companion borrows.

If this companion is useful to you, the course it follows is more so. It is free
to read at the link above, and it is taught with R and Seurat, which remains the
reference implementation that any other tool is measured against.

## Licence of the source

The HBC lessons are released under **Creative Commons Attribution 4.0
International (CC BY 4.0)**, declared in the front matter of each lesson:

```yaml
license: "CC-BY-4.0"
```

The licence text is at <https://creativecommons.org/licenses/by/4.0/>.

CC BY 4.0 permits adaptation, including for commercial purposes, on the
condition that credit is given, a link to the licence is provided, and **changes
are indicated**. This page is that indication. The changes are substantial and
are listed below.

## What was changed

- **The language is different.** The course teaches R with Seurat. This
  companion teaches BioLang. Every code block here is original BioLang written
  against the `singlecell` package; none of it is a line-by-line translation of
  the course's R.
- **All prose is written from scratch.** No text is copied from the HBC lessons.
  Where a passage explains the same idea, it explains it in different words.
- **No images are reproduced.** The course's figures are illustrative and
  several are credited to third parties (Trapnell 2015, Wagner 2016, Hicks
  2015); reusing them would require tracing each one's own licence. Where a
  figure carried an argument, the argument is made in text or redrawn from data
  the reader can generate.
- **No datasets are redistributed.** The course uses a PBMC dataset from a
  specific study; this companion uses the demo data bundled with BioLang, so the
  examples run offline. Cell counts and cluster numbers therefore differ from
  the course's, and no attempt is made to reproduce its exact figures.
- **The lesson boundaries are regrouped.** Fourteen lessons are covered in seven
  chapters, because some course lessons are environment setup that BioLang does
  not need. The mapping is spelled out in
  [The HBC Course in BioLang](hbc-overview.md).
- **Where BioLang cannot do something, this companion says so** rather than
  substituting a near-equivalent quietly. See the coverage table in the
  overview, which marks what is missing.

## Licence of this companion

These chapters are part of the BioLang repository and carry its MIT licence.
That covers the prose and BioLang code written here.

Because this material adapts a CC BY 4.0 work, the attribution requirement
travels with it: if you reuse these chapters, keep the credit to the HBC
training team and the link to their course.

## Citing

If you cite anything from this part of the book, cite the course it follows:

```bibtex
@misc{piper2022scrnaseq,
  title     = {hbctraining/scRNA-seq_online: scRNA-seq Lessons from HCBC},
  author    = {Piper, Mary and Mistry, Meeta and Liu, Jihe and
               Gammerdinger, William and Khetani, Radhika},
  year      = {2022},
  month     = {1},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.5826256},
  url       = {https://doi.org/10.5281/zenodo.5826256}
}
```

The HBC team asks for citations directly, and gives the reason: citations show
the community's needs, earn recognition for the work, and support further
funding for their teaching. That is a good reason, and it costs you one line.
