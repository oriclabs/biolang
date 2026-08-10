# Scanpy and Seurat validation

These checks generate a deterministic 10x MEX dataset locally. No count matrix
is committed or downloaded by the package.

Copy the installed package examples into a standalone working directory:

```text
bl examples singlecell --copy singlecell-examples
cd singlecell-examples
python make_demo_10x.py --output validation_data
bl run validation/biolang_reference.bl
python validation/compare_labels.py validation_data/truth.csv validation_biolang_labels.csv
python validation/scanpy_reference.py validation_data validation_scanpy_labels.csv
python validation/compare_labels.py validation_scanpy_labels.csv validation_biolang_labels.csv
Rscript validation/seurat_reference.R validation_data validation_seurat_labels.csv
python validation/compare_labels.py validation_seurat_labels.csv validation_biolang_labels.csv
```

The generated matrix and label CSV files are written under the copied working
directory. The full BioLang source repository is not required.

Python requires Scanpy and its Leiden dependencies. Use the pinned Python 3.12
environment below instead of a system Python installation:

```text
# Windows, from the copied example directory
py -3.12 -m venv .venv-scanpy
.venv-scanpy\Scripts\python -m pip install -r validation\requirements-scanpy.txt

# macOS/Linux, from the copied example directory
python3.12 -m venv .venv-scanpy
.venv-scanpy/bin/python -m pip install -r validation/requirements-scanpy.txt
```

Then substitute the environment's Python executable for `python` in the
commands above. The Numba constraint avoids a copied-function cache regression
seen with the Numba 0.66 Windows stack.

R requires `Seurat`, `Matrix`, and the Leiden backend used by Seurat. If
`Rscript` is not on `PATH`, invoke its absolute path. Keep both validation
environments separate; they are not runtime dependencies of the BioLang
package.

Cluster IDs themselves are arbitrary. The comparison joins cells by barcode and
uses adjusted Rand index, which compares partitions without requiring matching
numeric IDs. The default acceptance threshold is `0.70`; record the exact
package versions and inspect marker agreement before treating a result as
scientific equivalence.

## Verified reference run

The seeded fixture was validated on 2026-07-27:

| Runtime | Versions | Retained cells | Clusters | ARI vs BioLang |
|---|---|---:|---:|---:|
| BioLang | repository build after sparse single-cell changes | 220 | 4 | 1.0000 vs truth |
| Scanpy | Python 3.12.10, Scanpy 1.12.3, Numba 0.65.1, igraph 1.0.0, NumPy 2.4.6, pandas 3.0.5 | 220 | 4 | 1.0000 |
| Seurat | R 4.5.2, Seurat 5.5.1, Matrix 1.7.4, leidenbase 0.1.37 | 220 | 4 | 1.0000 |

This verifies the deterministic synthetic partition and workflow plumbing. It
does not establish equivalence for every tissue, protocol, QC threshold, or
real biological dataset.

## Advanced donor-aware validation

The advanced example uses four paired donors and deliberately induces an
interferon response in T cells:

```text
bl run singlecell/examples/advanced_analysis.bl
python singlecell/examples/validation/advanced_reference.py
Rscript singlecell/examples/validation/advanced_reference.R
```

Both reference scripts independently aggregate raw counts, calculate log2 CPM,
run a paired test, and compare the resulting effects and IFIT1 p-value with
`singlecell-results/paired-de.csv`. These scripts validate the transparent
paired exploratory model. Publication analyses should additionally run the
exported raw pseudobulk counts through DESeq2 or edgeR with an explicit
`~ donor + condition` design.

## Standalone SCTransform conformance

The maintained long-form runners, real-data workflows, and generated-evidence
instructions live in
[`oriclabs/biolang-workflows`](https://github.com/oriclabs/biolang-workflows/tree/main/validation/single-cell).
This package-local copy remains as the implementation-level conformance harness
during the non-destructive repository migration.

`run_sctransform_validation.py` runs the original R package and BioLang in two
separate processes against the same count matrix. The R package is a
validation-only installation and is not imported, linked, or needed by
BioLang. The driver records model parameters, residual variance, the ranked
feature list, a residual probe drawn from the top 3,000 features, transform and
whole-process time, and peak host working set.

The runner requires the calibrated `glmGamPoi_offset` oracle backend by
default and refuses a silent fallback to `nb_offset`. Use `--oracle-method`
only when intentionally measuring a different reference profile.

Build BioLang in release mode, install `sctransform` separately in R, and run a
small deterministic check:

```powershell
cargo build --release -p bl-cli
python packages/singlecell/examples/validation/run_sctransform_validation.py `
  --mode synthetic `
  --output validation-results/sctransform-synthetic-current
```

For a real 10x MEX matrix:

```powershell
Rscript packages/singlecell/examples/validation/prepare_hbc_sctransform_fixture.R `
  ctrl_raw stim_raw validation-results/hbc-sctransform-input
python packages/singlecell/examples/validation/run_sctransform_validation.py `
  --mode tenx `
  --input validation-results/hbc-sctransform-input/ctrl `
  --output validation-results/sctransform-hbc-ctrl-current
```

The output directory must be new so observations from different builds cannot
be mixed. `comparison.json` contains every numeric and resource metric plus
independent pass/fail gates. CPU is the reproducibility default; pass
`--gpu auto` or `--gpu on` only when intentionally validating that backend.
