# Single-Cell RNA-seq with BioLang

A tissue is not one thing. A blood sample contains many immune populations. A
tumor contains malignant cells, immune cells, blood-vessel cells, connective
tissue cells, and damaged cells. Bulk RNA sequencing mixes their RNA into one
average. Single-cell RNA sequencing (scRNA-seq) measures thousands of those
cells separately.

That extra detail is useful, but it creates a new problem: a colorful map can be
easy to produce and easy to overinterpret. This book teaches both the workflow
and the reasoning that makes the workflow defensible.

## Who this book is for

- **Biologists** can connect laboratory decisions to computational artifacts.
- **Researchers** can turn a biological question into a reproducible analysis.
- **Programmers** can understand the data model, algorithms, and performance
  boundaries without first becoming molecular biologists.
- **Clinicians** can understand what a single-cell result does and does not say
  about a patient or treatment.
- **Students and interested readers** can begin without prior RNA-seq knowledge.

Each chapter answers five questions:

1. **What** is this concept?
2. **Why** does it matter?
3. **How** is it represented or analyzed?
4. **When and where** should it be used?
5. **What can fool us?**

## What you will build

You will analyze a deterministic, tumor-like 10x count matrix with four known
cell populations. The fixture is intentionally small enough for a laptop and is
generated locally rather than stored in the repository. You will:

- inspect raw counts and quality metrics;
- remove implausible cells and rarely detected genes;
- normalize counts and select variable genes;
- compute PCA, a neighbor graph, Leiden clusters, and UMAP;
- inspect markers and propose cell-type labels;
- compare BioLang clusters with Scanpy and Seurat;
- plan a replicate-aware multi-sample analysis;
- record the decisions needed to reproduce the result.

## A necessary boundary

This book teaches research analysis. A cluster, marker, or association is not a
clinical diagnosis. Clinical use requires an independently validated assay,
quality system, predefined decision rule, appropriate population, regulatory
review, and human oversight.

BioLang's `singlecell` package is useful for transparent preprocessing,
exploration, and reproducible workflows. It does not make biological judgment
automatic. The analyst remains responsible for study design, metadata, cell
annotations, statistical units, and interpretation.

## How to read it

Read Parts I and II in order on a first pass. Part III is organized by
scientific question. Part IV should be read before publishing or handing a
result to a collaborator. Terms in **bold** are collected in the
[glossary](appendix-glossary.md), and every package call is summarized in the
[API appendix](appendix-api.md).
