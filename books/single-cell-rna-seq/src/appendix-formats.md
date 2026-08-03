# File Formats and Conversion

## 10x Matrix Exchange

Expected files:

```text
matrix.mtx or matrix.mtx.gz
features.tsv or features.tsv.gz
barcodes.tsv or barcodes.tsv.gz
```

BioLang reads these natively with `sc.load(path)`. Confirm whether the directory
is raw or filtered and whether it contains gene expression alone or additional
feature types.

## AnnData Zarr

BioLang can read and write sparse AnnData Zarr stores. The current native
interchange preserves:

- sparse `X`;
- observation index names;
- variable index names.

It does not yet promise lossless transfer of arbitrary `obs`/`var` columns,
layers, embeddings, graphs, or unstructured metadata. Roundtrip a representative
object and compare dimensions, names, nonzero values, and checksums before using
it as an archive.

## H5AD

Direct `.h5ad` I/O is not built into the runtime. Convert with Python/anndata:

```python
import anndata as ad

adata = ad.read_h5ad("input.h5ad")
adata.write_zarr("input.zarr")
```

Then read the Zarr store through the BioLang AnnData functions. Record the
anndata version and conversion command.

## Seurat objects

An `.rds` Seurat object is R-specific and can contain multiple assays, layers,
reductions, and graphs. Export the exact matrix and metadata required for the
handoff, preferably with stable gene and barcode keys. Never assume row order
alone survived conversion.

## Delimited tables

CSV and TSV are useful for compact metadata and results. Note the argument
order: the table comes first, the path second.

> Requires CLI: this example writes a local file.

```biolang
let labels = table([
    {barcode: "cell_1", sample: "S01", cluster: 0},
    {barcode: "cell_2", sample: "S01", cluster: 1}
])
write_csv(labels, "cell-labels.csv")
```

Join tables by barcode and sample ID. Barcodes are not necessarily globally
unique across samples.

## Large and private inputs

Do not put protected patient matrices in a public package or book. For public
datasets, record accession, expected files, checksums, and license. For private
datasets, use approved storage and access controls, and keep a manifest that can
be audited without exposing the data.
