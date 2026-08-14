# Third-party notice

This MIT-licensed crate vendors Spotify Annoy 1.17.3 headers under Apache-2.0:

- `vendor/annoy/annoylib.h`
- `vendor/annoy/kissrandom.h`
- `vendor/annoy/mman.h`

Copyright (c) 2013 Spotify AB. The complete Apache-2.0 text is in
`vendor/annoy/LICENSE-APACHE-2.0`.

The vendored files match the Annoy headers shipped by RcppAnnoy 0.0.23, the
version used by the independent Seurat 5.5.1 validation environment. No
RcppAnnoy wrapper source is included. `src/annoy_bridge.cpp` is a new thin MIT
C ABI written for BioLang.

