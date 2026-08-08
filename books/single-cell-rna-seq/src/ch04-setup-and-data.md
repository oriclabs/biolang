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
bl install ./singlecell
bl examples ./singlecell --copy singlecell-examples
cd singlecell-examples
```

`bl install` takes a **path**, not a name. There is no package registry yet, and
`bl install --git <url>` clones a whole repository into the package slot, which
cannot work for a package living in a subdirectory of one. If you do not have a
checkout, the
[starter kit](../../hbc-scrnaseq/html/downloads/singlecell-starter.zip) ships
the package ready to install.

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

## Get the data

This book runs on a real experiment: peripheral blood mononuclear cells from
lupus patients (Kang et al., *Nature Biotechnology* 2018), in two samples — an
untreated control and one stimulated with interferon-beta. It is the dataset the
Harvard Chan Bioinformatics Core teaches on, and they host a packaged copy.

```text
python get-data.py
```

`get-data.py` is in the
[starter kit](../../hbc-scrnaseq/html/downloads/singlecell-starter.zip), or on
its own at
[get-data.py](../../hbc-scrnaseq/html/downloads/get-data.py).

**Be warned about the size.** The archive is 3.2 GB and the two directories this
book needs total about 90 MB. Zip stores its index at the end of the file, so
there is no way to fetch two members from a remote archive without pulling the
whole thing. The script downloads once, extracts what is used, and deletes the
archive, so the 3.2 GB is transient rather than resident.

Afterwards you have:

```text
ctrl_raw/     barcodes.tsv.gz  features.tsv.gz  matrix.mtx.gz
stim_raw/     barcodes.tsv.gz  features.tsv.gz  matrix.mtx.gz
```

Run everything from the directory containing those.

> **Why not a synthetic fixture?** An earlier edition of this book generated one,
> and it was faster and needed no download. It was also wrong in a way that
> mattered: its genes were named `MARK0_000`, `MARK1_000`, and so on. Every
> mechanical step worked, and the one thing the annotation chapter exists to
> teach — reading `LYZ`, `CD14` and `S100A8` together and concluding *monocytes*
> — could not be shown at all. Real data also exercises the code differently:
> uneven cluster sizes, 737,280 droplets, and genuinely sparse matrices found
> four separate bugs that evenly sized toy data never touched.

## Load it

> Requires CLI: package imports and local filesystem access are not available in
> the browser runner.

```biolang
import "singlecell" as sc

let cells = sc.load("ctrl_raw")
println("droplets: " + str(cells.n_cells))
println("genes:    " + str(cells.n_genes))
println("first genes: " + str(sc.get_genes(cells) |> take(5)))
```

```text
droplets: 737280
genes:    33538
first genes: [MIR1302-2HG, FAM138A, OR4F5, AL627309.1, AL627309.3]
```

737,280 is every barcode the chemistry can produce, not every cell — the
overwhelming majority are empty droplets. Sorting that out is
[Quality Control](ch05-quality-control.md).

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
