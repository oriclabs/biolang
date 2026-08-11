# Seurat MIT source notice

BioLang's anchor-integration compatibility code includes adaptations of
MIT-licensed Seurat algorithms. BioLang does not link to R or require Seurat at
build time or runtime.

## Pinned source

- Package: Seurat 5.5.1
- Source: CRAN `Seurat_5.5.1.tar.gz`
- SHA-256: `9614ef02d3e1010c40be5916a309103a76c4221a667cbc4b312e5126459a5821`
- Upstream files consulted: `R/integration.R`,
  `R/dimensional_reduction.R`, `src/integration.cpp`, and
  `src/data_manipulation.cpp`
- Upstream package metadata: `License: MIT + file LICENSE`

The BioLang implementation changes R/S4 matrices and RcppEigen sparse kernels
into BioLang records and Rust dense/scalable neighbour operations. It preserves
the Seurat 5.5.1 cell-wise CCA standardization, mutual-nearest-neighbour anchor
definition, high-dimensional anchor filter, four-neighbour-set anchor score,
1%/90% score rescaling, integration-vector direction, and weighting kernel.
BioLang's large-data neighbour search is a deterministic/GPU-capable
replacement for Seurat's separately licensed Annoy dependency.

No source from `sctransform`, `uwot`, `irlba`, `RcppAnnoy`, `leidenbase`, or
`igraph` is included or translated.

## Upstream MIT licence

Copyright (c) 2021 Seurat authors

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
