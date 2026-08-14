# Third-party notices

## Seurat 5.5.1

Parts of BioLang's single-cell compatibility implementation adapt algorithms
from Seurat 5.5.1, distributed by the Seurat authors under the MIT licence.
The adapted Rust code is principally in:

- `crates/bl-runtime/src/singlecell.rs` (CCA anchors, SNN, and integration);
- `crates/bio-core/src/cluster_ops.rs` (Seurat `algorithm = 1` Louvain
  compatibility, including the Modularity Optimizer 1.3.0 random stream and
  move schedule).

Pinned archive: CRAN `Seurat_5.5.1.tar.gz`, SHA-256
`9614ef02d3e1010c40be5916a309103a76c4221a667cbc4b312e5126459a5821`.
Consulted upstream files include `R/integration.R`,
`R/dimensional_reduction.R`, `src/integration.cpp`,
`src/data_manipulation.cpp`, `src/ModularityOptimizer.h`,
`src/ModularityOptimizer.cpp`, and `src/RModularityOptimizer.cpp`.

The Modularity Optimizer implementation identifies its lineage as version
1.3.0 by Ludo Waltman and Nees Jan van Eck. BioLang translates its matrix,
record, memory-management, and error-handling interfaces to safe Rust; the
Java-compatible RNG, permutation, local-moving, multilevel, restart, and
cluster-ordering behaviour is intentionally preserved for compatibility.

Copyright (c) 2021 Seurat authors

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Spotify Annoy 1.17.3

`crates/bl-seurat-compat/vendor/annoy` contains the Spotify Annoy 1.17.3
headers used for Seurat-compatible Euclidean nearest-neighbour indexing.
Spotify Annoy is Copyright (c) 2013 Spotify AB and distributed under the
Apache License 2.0. The full licence is included at
`crates/bl-seurat-compat/vendor/annoy/LICENSE-APACHE-2.0`.

The vendored header bytes match those distributed in RcppAnnoy 0.0.23, but no
GPL-licensed Rcpp/R wrapper source is included or linked. BioLang's
`annoy_bridge.cpp` and Rust interface are new MIT code. The compatibility path
uses 50 trees, the default Annoy seed, Euclidean float vectors, and
`search_k=-1`, matching Seurat 5.5.1's `NNHelper(method = "annoy")` contract.

## Optional GPL SCTransform provider

`bl-sctransform` / `sctransform-rs` is a separate GPL-3.0-only executable. It is
not linked into BioLang and is not covered by BioLang's MIT licence. When a user
installs it, BioLang may launch it as a separate process and exchange neutral
files. Distribution of that executable must include its corresponding GPL
source and notices. BioLang's native, paper-derived SCTransform implementation
remains available without that provider.
