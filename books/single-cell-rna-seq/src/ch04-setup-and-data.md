# Set Up BioLang and the Data

## Install and verify

Install a current BioLang release, then confirm the CLI and capabilities:

```text
bl version
bl doctor
```

From the BioLang repository, build the development binary with:

```text
cargo build -p bl-cli
```

Install the package, then copy its examples into a standalone working
directory:

```text
bl install singlecell
bl examples singlecell --copy singlecell-examples
cd singlecell-examples
```

The install step is what makes `import "singlecell" as sc` resolve. Imports are
searched in the current directory first and `~/.biolang/packages/` last, so a
copied example directory has nothing to import until the package is installed —
without it every script here fails with
`module or plugin 'singlecell' not found`.

Working from a BioLang source checkout, install and copy from the local path
instead:

```text
bl install packages/singlecell
bl examples packages/singlecell --copy singlecell-examples
```

The copied directory includes the BioLang, notebook, Python, and validation
example files; it does not require the full BioLang repository. Because the
current directory wins, a checkout still overrides the installed copy when you
run from `packages/` — reinstall after editing the package if you want the
change visible elsewhere.

## Generate the teaching matrix

The fixture contains four populations with distinct marker blocks, shared
background genes, mitochondrial genes, and low-information droplets. It is
synthetic: no person or patient data is involved.

```text
python make_demo_10x.py --output nsclc_like
```

`--output` is relative to the current directory. With the commands above, the
fixture is created at:

```text
<your working directory>/nsclc_like/
```

It contains `matrix.mtx.gz`, `features.tsv.gz`, `barcodes.tsv.gz`, and
`truth.csv`. The later `sc.load("nsclc_like")` calls resolve that same directory
because the examples are run from the copied working directory.

Expected summary:

```text
168 genes x 265 barcodes (25 junk)
output: <your working directory>/nsclc_like
```

The generator uses a fixed seed. Re-running it produces the same logical
dataset, making it suitable for tests and comparisons.

## Load it

> Requires CLI: package imports and local filesystem access are not available in
> the browser runner.

```biolang
import "singlecell" as sc

let cells = sc.load("nsclc_like")
println(sc.summary(cells))
```

The object is a BioLang record. Important fields are:

| Field | Meaning |
|---|---|
| `matrix` | Raw cells-by-genes count matrix |
| `layers.counts` | Preserved raw counts |
| `genes` | Gene names in matrix order |
| `barcodes` | Cell barcodes in matrix order |
| `obs` | Cell metadata table |
| `var` | Gene metadata table |
| `n_cells`, `n_genes` | Current dimensions |

The count matrix remains sparse after 10x loading, filtering, normalization, and
HVG selection. Compact PCA scores are dense because every cell has a score on
every retained component.

## Use an in-memory matrix

For a tiny test, construct cells directly:

> Requires CLI: this example imports the `singlecell` package.

```biolang
import "singlecell" as sc

let counts = matrix([
    [8, 0, 1, 0],
    [7, 0, 2, 0],
    [0, 9, 0, 1],
    [0, 8, 0, 2]
])
let tiny = sc.from_matrix(
    counts,
    ["T_MARKER", "B_MARKER", "HOUSEKEEPING", "MT-ND1"],
    ["cell_1", "cell_2", "cell_3", "cell_4"]
)
println(sc.summary(tiny))
```

## Keep source and generated data separate

A reproducible project can use:

```text
project/
  README.md
  biolang.toml
  scripts/
  data/raw/          # immutable or checksummed inputs
  data/derived/      # generated matrices
  results/tables/
  results/figures/
  validation/
```

Do not commit private or very large matrices merely to make a script look
self-contained. Record a stable accession or an approved storage location,
checksum the input, and provide a deterministic download or generation step.
