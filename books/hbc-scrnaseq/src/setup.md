# Getting the data

Everything in this book runs on the HBC course's own dataset. Three steps, once.

## 1. Install BioLang

**Linux / macOS**

```sh
curl -fsSL https://lang.bio/install.sh | sh
```

**Windows (PowerShell)**

```powershell
iwr -useb https://lang.bio/install.ps1 | iex
```

Check it:

```sh
bl --version
```

## 2. Install the singlecell package

Download **[singlecell-starter.zip](downloads/singlecell-starter.zip)**, unzip
it, and from inside:

```sh
bl install ./singlecell
```

> There is no package registry yet, and `bl install --git <url>` clones a whole
> repository into the package slot — which cannot work for a package living in a
> subdirectory of one. Installing from a local path is the supported route, so
> the kit ships the package rather than pointing at it.

## 3. Download the matrices

```sh
python get-data.py
```

**Be warned about the size.** The archive is **3.2 GB**; the two directories
this book needs total about **90 MB**. Zip keeps its index at the end of the
file, so there is no way to pull two members out of a remote archive without
fetching the whole thing — HBC's own `download.sh` has the same problem. The
script downloads once, extracts what is used, and deletes the archive, so the
3.2 GB is transient rather than resident. Pass `--keep` to hold on to it.

The rest of the archive is R objects — a 2.2 GB `seurat_integrated.RData.bz2`
among them — that BioLang cannot read and this book does not need.

When it finishes you have:

```text
ctrl_raw/     barcodes.tsv.gz  features.tsv.gz  matrix.mtx.gz
stim_raw/     barcodes.tsv.gz  features.tsv.gz  matrix.mtx.gz
```

Run everything from the directory containing those.

## Check it worked

```biolang
import "singlecell" as sc

let ctrl = sc.load("ctrl_raw")
println("droplets: " + str(ctrl.n_cells))
println("genes:    " + str(ctrl.n_genes))
```

which prints:

```text
droplets: 737280
genes:    33538
```

737,280 is every barcode the chemistry can produce, not every cell — the vast
majority are empty droplets. Sorting that out is [Quality Control](ch02-quality-control.md).

## What the experiment is

Peripheral blood mononuclear cells from lupus patients (Kang et al., *Nature
Biotechnology* 2018), in two samples:

| Sample | |
|---|---|
| `ctrl_raw` | untreated control |
| `stim_raw` | stimulated with interferon-beta |

Two samples of the same cells under different conditions is what makes this
dataset worth teaching on: it has real biological differences *and* a real batch
structure, so [Integration](ch04-integration.md) has something honest to work
against rather than a planted offset.

## A note on time

This is a real dataset and BioLang runs it single-threaded. On a laptop:

| Step | Roughly |
|---|---|
| Load one raw matrix | 2 s |
| Filter to cells | 5 s |
| Normalize, HVG, PCA, neighbours, cluster | 15 s |
| The same on both samples merged | 40 s |
| Harmony on 30,000 cells | minutes |

Nothing here needs a cluster, but the two-sample chapters are not instant.
