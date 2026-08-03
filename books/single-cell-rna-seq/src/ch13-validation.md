# Validate with Scanpy and Seurat

## What validation can establish

Cross-library validation can detect mistakes in orientation, filtering,
barcodes, graph construction, and label export. It cannot prove that one
workflow is biologically correct merely because another package agrees.

The package includes independent reference scripts. Install the package, copy
its examples, and enter the resulting working directory:

```text
bl install singlecell
bl examples singlecell --copy singlecell-examples
cd singlecell-examples

validation/
  biolang_reference.bl
  scanpy_reference.py
  seurat_reference.R
  compare_labels.py
```

## Run the three workflows

Create the isolated Scanpy environment once in the copied example directory:

```text
# Windows
py -3.12 -m venv .venv-scanpy
.venv-scanpy\Scripts\python -m pip install -r validation\requirements-scanpy.txt

# macOS/Linux
python3.12 -m venv .venv-scanpy
.venv-scanpy/bin/python -m pip install -r validation/requirements-scanpy.txt
```

Then run from `singlecell-examples`:

```text
.venv-scanpy\Scripts\python make_demo_10x.py --output validation_data
bl run validation/biolang_reference.bl
.venv-scanpy\Scripts\python validation/scanpy_reference.py validation_data validation_scanpy_labels.csv
Rscript validation/seurat_reference.R validation_data validation_seurat_labels.csv
```

The generated matrix is `<your working directory>/validation_data/`.
Every relative data and label path in this section is resolved from the
copied example directory.

The commands above show Windows paths. On macOS/Linux, use
`.venv-scanpy/bin/python` and `bl`. On Windows, use the full
`Rscript.exe` path if it is not on `PATH`.

Compare by barcode and ARI:

```text
.venv-scanpy\Scripts\python validation/compare_labels.py validation_data/truth.csv validation_biolang_labels.csv
.venv-scanpy\Scripts\python validation/compare_labels.py validation_scanpy_labels.csv validation_biolang_labels.csv
.venv-scanpy\Scripts\python validation/compare_labels.py validation_seurat_labels.csv validation_biolang_labels.csv
```

## Verified result

The seeded fixture was checked on 2026-07-27:

| Runtime | Key versions | Cells | Clusters | ARI vs BioLang |
|---|---|---:|---:|---:|
| BioLang | repository sparse workflow | 220 | 4 | 1.0000 vs truth |
| Scanpy | Python 3.12.10; Scanpy 1.12.3; Numba 0.65.1; igraph 1.0.0 | 220 | 4 | 1.0000 |
| Seurat | 5.5.1; Matrix 1.7.4 | 220 | 4 | 1.0000 |

Cluster numbers need not match. ARI compares which cells are grouped together.
PCA signs can also differ without changing the geometry.

The current official workflows remain useful comparators:
[Scanpy preprocessing and clustering](https://scanpy.readthedocs.io/en/latest/tutorials/basics/clustering.html)
and the [Seurat guided clustering tutorial](https://satijalab.org/seurat/articles/pbmc3k_tutorial.html).

## Validate more than one number

For a real dataset compare:

- retained cells and genes by stable identifier;
- per-cell totals, genes detected, and mitochondrial percentage;
- normalized value summaries;
- HVG overlap, allowing for method differences;
- PCA variance explained and pairwise structure;
- graph degree and connected components;
- cluster ARI or NMI;
- marker effect directions and ranks;
- exported file dimensions and checksums.

Set acceptance criteria before seeing the result. Investigate disagreement
instead of tuning until the tools match.

## Validate donor-aware differential expression

After copying the examples, run:

```text
bl run advanced_analysis.bl
python validation/advanced_reference.py
Rscript validation/advanced_reference.R
```

On Windows, use the full `Rscript.exe` path when necessary. The scripts
independently aggregate the same paired donor fixture, calculate log2 CPM, run
paired tests, and compare BioLang's IFIT1 effect and p-value.

The reference check on 2026-07-27 agreed to numerical tolerance with Python
3.13.14, NumPy 2.4.6, SciPy 1.18.0, and R 4.6.1:

```text
IFIT1 log2fc = 1.701812
paired p     = 0.000242968
```

This validates implementation parity for the stated exploratory model. It does
not turn a paired t-test on log2 CPM into DESeq2. A real study should separately
validate raw pseudobulk counts, sample design, contrasts, dispersion estimates,
effect directions, adjusted p-values, and gene identifiers.
