# Single-Cell RNA-seq with BioLang — starter kit

Everything needed to run Part V of *Single-Cell RNA-seq with BioLang*
(**The HBC Course in BioLang**) without cloning the repository.

The book is at <https://lang.bio/books/single-cell-rna-seq/html/hbc-overview.html>.

## 1. Install BioLang

**Linux / macOS**

```sh
curl -fsSL https://lang.bio/install.sh | sh
```

**Windows (PowerShell)**

```powershell
iwr -useb https://lang.bio/install.ps1 | iex
```

Check it worked:

```sh
bl --version
```

## 2. Install the singlecell package

From inside this directory:

```sh
bl install ./singlecell
```

That copies the package to `~/.biolang/packages/singlecell`, which is where
`import "singlecell" as sc` looks for it.

> There is no package registry yet, and `bl install --git <repo-url>` clones a
> whole repository into the package slot — which does not work for a package
> that lives in a subdirectory. Installing from this local path is the
> supported route.

## 3. Run something

```sh
bl run hbc-01-setup.bl
```

Expected:

```text
cells: 265
genes: 168
first genes:    [MARK0_000, MARK0_001, MARK0_002, MARK0_003, MARK0_004]
first barcodes: [0000AACCGGTT-1, 0001AACCGGTT-1, 0002AACCGGTT-1]
```

Then work through the chapters in order, or run the whole thing as one
notebook:

```sh
bl notebook hbc-companion.bln
```

Several chapters write `.svg` figures into the current directory.

## What is in here

| Path | What it is |
|---|---|
| `singlecell/` | The BioLang package — install this |
| `nsclc_like/` | The teaching dataset, **pre-generated** |
| `make_demo_10x.py` | Regenerates `nsclc_like/`; optional |
| `hbc-0*.bl` | One script per chapter |
| `hbc-companion.bln` | The whole part as one runnable notebook |

**You do not need Python.** `nsclc_like/` is already generated and included.
`make_demo_10x.py` is here only if you want to change the fixture — a different
number of cells, more populations, a different seed:

```sh
python make_demo_10x.py --output nsclc_like
```

It writes a 10x Genomics MEX directory (`barcodes.tsv.gz`, `features.tsv.gz`,
`matrix.mtx.gz`) plus `truth.csv` giving each cell's true population, which is
what lets you check whether clustering recovered the right answer.

The dataset is **synthetic** — 265 barcodes over 168 genes with four planted
populations, from a fixed seed. No person or patient data is involved. Your
numbers will not match the HBC course's, which uses a published PBMC dataset;
the shapes are what you compare.

To use your own data instead, `sc.load` takes the path to any 10x MEX
directory:

```biolang
let obj = sc.load("path/to/filtered_feature_bc_matrix")
```

## Attribution

Part V follows the curriculum of
[Introduction to single-cell RNA-seq](https://hbctraining.github.io/Intro-to-scRNAseq/)
by the Harvard Chan Bioinformatics Core — Mary Piper, Meeta Mistry, Radhika
Khetani, Lorena Pantano, Jihe Liu, Will Gammerdinger and Noor Sohail — released
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

The teaching sequence is theirs. The prose and BioLang code here are original;
no text, figures or datasets are reproduced from the course. Full credit and
the list of changes are on the book's Attribution page.

If you cite this material, cite the course:

> Mary Piper, Meeta Mistry, Jihe Liu, William Gammerdinger, & Radhika Khetani.
> (2022). hbctraining/scRNA-seq_online: scRNA-seq Lessons from HCBC. Zenodo.
> <https://doi.org/10.5281/zenodo.5826256>

The BioLang code and prose carry the repository's MIT licence.
