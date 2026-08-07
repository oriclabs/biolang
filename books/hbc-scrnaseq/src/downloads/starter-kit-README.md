# The HBC Single-cell Course, in BioLang — starter kit

Everything needed to run the book, except the data, which the included script
fetches.

The book is at <https://lang.bio/books/hbc-scrnaseq/html/index.html>.

## 1. Install BioLang

**Linux / macOS**

```sh
curl -fsSL https://lang.bio/install.sh | sh
```

**Windows (PowerShell)**

```powershell
iwr -useb https://lang.bio/install.ps1 | iex
```

```sh
bl --version
```

## 2. Install the singlecell package

From inside this directory:

```sh
bl install ./singlecell
```

That copies the package to `~/.biolang/packages/singlecell`, where
`import "singlecell" as sc` looks for it.

> There is no package registry yet, and `bl install --git <repo-url>` clones a
> whole repository into the package slot — which does not work for a package in
> a subdirectory. Installing from this local path is the supported route.

## 3. Get the data

```sh
python get-data.py
```

**The archive is 3.2 GB and about 90 MB is kept.** Zip stores its index at the
end of the file, so two members cannot be pulled from a remote archive without
fetching the whole thing; the HBC course's own `download.sh` has the same
problem. The script downloads once, extracts what the book uses, and deletes the
archive. Pass `--keep` to hold on to it.

Afterwards you have `ctrl_raw/` and `stim_raw/`, each with `barcodes.tsv.gz`,
`features.tsv.gz` and `matrix.mtx.gz`.

## 4. Run something

```sh
bl run ch01-biology-and-matrix.bl
```

Expected:

```text
droplets: 737280
genes:    33538
first genes: [MIR1302-2HG, FAM138A, OR4F5, AL627309.1, AL627309.3]
```

Then work through the chapters, or run the whole thing as one notebook:

```sh
bl notebook hbc-scrnaseq.bln
```

## What is in here

| Path | What it is |
|---|---|
| `singlecell/` | The BioLang package — install this |
| `get-data.py` | Fetches the two 10x matrices |
| `ch0*.bl` | One script per chapter |
| `hbc-scrnaseq.bln` | The whole book as a runnable notebook |

## A note on time

This is a real dataset — 737,280 droplets per sample, 15,049 cells after QC —
and BioLang runs single-threaded. Loading takes seconds and a full pipeline
about fifteen; Harmony on the merged 30,043 cells takes minutes.

The chapter scripts are slower than they look. Each block in the book is written
to stand alone so a reader can copy any one of them, which means a concatenated
chapter rebuilds the pipeline from the raw matrix once per block. Prefer the
notebook, or lift a single block, if you are working interactively.

## The data

Peripheral blood mononuclear cells from lupus patients (Kang et al., *Nature
Biotechnology* 2018) — a control sample and one stimulated with interferon-beta.
It is not redistributed here; `get-data.py` fetches it from the HBC training
team's own host.

## Attribution

This follows the curriculum of
[Introduction to single-cell RNA-seq](https://hbctraining.github.io/Intro-to-scRNAseq/)
by the Harvard Chan Bioinformatics Core — Mary Piper, Meeta Mistry, Radhika
Khetani, Lorena Pantano, Jihe Liu, Will Gammerdinger and Noor Sohail — released
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

The teaching sequence is theirs. The prose and BioLang code here are original;
no text, figures or datasets are reproduced. Full credit and the list of changes
are on the book's Attribution page.

If you cite this material, cite the course:

> Mary Piper, Meeta Mistry, Jihe Liu, William Gammerdinger, & Radhika Khetani.
> (2022). hbctraining/scRNA-seq_online: scRNA-seq Lessons from HCBC. Zenodo.
> <https://doi.org/10.5281/zenodo.5826256>

The BioLang code and prose carry the repository's MIT licence.
