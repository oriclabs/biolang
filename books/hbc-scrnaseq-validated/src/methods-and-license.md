# Methods, dependencies, and licensing

BioLang remains MIT-licensed. Seurat 5.5.1 and SeuratObject 5.4.0 are themselves
MIT-licensed, so their covered R and C++ source may be inspected and reused
provided the MIT copyright and licence notice is retained and modifications are
recorded. The current BioLang single-cell implementation has no runtime,
build-time, or FFI dependency on R or Seurat.

Several packages used by a Seurat workflow have separate copyleft licences,
including `sctransform`, `uwot`, `irlba`, `RcppAnnoy`, `leidenbase`, and
`igraph`. Their implementations are not copied, translated, linked, or
vendored in BioLang. Equivalent BioLang components must remain independently
implemented from papers or use permissively licensed sources.

GPU acceleration uses `wgpu` (MIT/Apache-2.0), `pollster` (MIT/Apache-2.0),
and a BioLang-owned WGSL compute kernel. It targets the operating system's
Vulkan, Metal, or DirectX 12 interface and does not link CUDA or a proprietary
vendor SDK. GPU use is optional: `bl --no-gpu ...` or `BIOLANG_GPU=off` selects
the f64 CPU implementation.

The current implementation is based on algorithm descriptions in the
scientific literature. Future exact-parity work may also port MIT-covered
Seurat code with file-level provenance and attribution. The detailed mapping
from each public API to its method, source, and known approximation is
maintained in
[`packages/singlecell/METHODS.md`](../../../packages/singlecell/METHODS.md).

External R packages may be used as result oracles during development. That
validation is isolated from BioLang's implementation:

- inputs are public matrices;
- cells are joined by sample and original barcode;
- validation scripts and result manifests are never imported by BioLang;
- no copyleft package source, object, serialized model, or generated
  implementation is copied into BioLang;
- BioLang tests and builds do not require the oracle.

The HBC teaching materials are separately licensed CC BY 4.0. This book credits
and links to HBC, but its prose and BioLang code were written independently.
The HBC course remains the authoritative source for its own teaching content.
