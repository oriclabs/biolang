# Run the complete notebook

The complete executable analysis lives in
[`packages/singlecell/examples/hbc_course_validation.bln`](../../../packages/singlecell/examples/hbc_course_validation.bln).
Keeping one notebook as the executable specification prevents snippets in a
book from becoming a second, subtly different pipeline.

From the repository root:

```powershell
cargo build --release -p bl-cli
target/release/bl.exe notebook packages/singlecell/examples/hbc_course_validation.bln
```

The run header prints the selected compute policy and device before analysis.
Use `--no-gpu` for the deterministic f64 CPU path; do not report a result
without retaining that header or the summary manifest.

Expected deterministic console checkpoints are:

```text
filtered ctrl: 14847
filtered stim: 14782
merged: 29629
genes: 14065
selected features: 3000
```

The validated GPU run produced 15 clusters, 46,484 anchors, and 9,359 positive
marker rows in 732.009 seconds. It recorded
`countsketch_subspace_gpu` on an NVIDIA GeForce RTX 3080 through Vulkan. The CPU
backend was not rerun after separating the 30 anchor dimensions from the 50-PC
downstream calculation, so older CPU numbers are not presented as current.
These values can change when an intentionally versioned algorithm changes; the
notebook's manifest makes such changes visible.

## Generated files

| File | Purpose |
|---|---|
| `validation-results/hbc-biolang/features.csv` | selected variable genes |
| `validation-results/hbc-biolang/cells.csv` | barcode, sample, cluster, and UMAP coordinates |
| `validation-results/hbc-biolang/pcs.csv` | barcode, sample, and 40 integrated PCs |
| `validation-results/hbc-biolang/markers.csv` | positive one-versus-rest marker results |
| `validation-results/hbc-biolang/umap.svg` | cluster-colored embedding |
| `validation-results/hbc-biolang/summary.csv` | run parameters and dimensions |

Bulky full outputs remain ignored and are recreated from the notebook. A
compact [evidence snapshot](./evidence/2026-08-09/README.md) commits the run
summaries, hashes, logs, cell labels, features, markers, and final comparison so
the published measurements are auditable rather than inferred.
