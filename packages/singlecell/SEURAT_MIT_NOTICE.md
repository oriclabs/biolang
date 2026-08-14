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
  `src/data_manipulation.cpp`; Louvain compatibility also adapts
  `src/ModularityOptimizer.h`, `src/ModularityOptimizer.cpp`, and
  `src/RModularityOptimizer.cpp`
- Upstream package metadata: `License: MIT + file LICENSE`

The BioLang implementation changes R/S4 matrices and RcppEigen sparse kernels
into BioLang records and Rust dense/scalable neighbour operations. It preserves
the Seurat 5.5.1 cell-wise CCA standardization, mutual-nearest-neighbour anchor
definition, high-dimensional anchor filter, four-neighbour-set anchor score,
1%/90% score rescaling, integration-vector direction, and weighting kernel.
Native builds use Spotify Annoy 1.17.3 for Seurat's 50-tree Euclidean
neighbour contract. Annoy is separately attributed under Apache-2.0 in the
repository third-party notices. Browser builds retain the deterministic/GPU
fallback because the C++ index is not linked into WebAssembly.

BioLang's Seurat-compatible Louvain path translates the matrix and graph
representation to safe Rust while preserving Modularity Optimizer 1.3.0's
Java-compatible random stream, permutation, local-moving schedule, multilevel
aggregation, repeated iterations, restarts, and final cluster ordering. The
upstream implementation identifies Modularity Optimizer 1.3.0 as work by Ludo
Waltman and Nees Jan van Eck.

No source from `sctransform`, `uwot`, `irlba`, the GPL-licensed RcppAnnoy
wrapper, `leidenbase`, or `igraph` is included or translated. The underlying
Spotify Annoy headers are included under their own Apache-2.0 licence.

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
