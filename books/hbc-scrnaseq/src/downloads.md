# Downloads

None of the code in this book runs in the browser: every example imports the
`singlecell` package and most read or write files, and package imports and file
I/O are CLI-only. So the pages show no Run button — copy the code, or take one
of these.

## The starter kit

**[singlecell-starter.zip](downloads/singlecell-starter.zip)** — the package, the
data-download script, every chapter script and the notebook.

```sh
unzip singlecell-starter.zip && cd singlecell-starter
bl install ./singlecell
python get-data.py
bl run ch01-biology-and-matrix.bl
```

Full setup, including installing BioLang itself, is in
[Getting the Data](setup.md).

## The data script

**[get-data.py](downloads/get-data.py)** — fetches the two 10x matrices from the
HBC course's own host. 3.2 GB down, ~90 MB kept; the archive is deleted unless
you pass `--keep`. Also included in the kit.

## Chapter scripts

One per chapter, each self-contained.

| Chapter | Script | Blocks |
|---|---|---|
| The Biology and the Matrix | [ch01-biology-and-matrix.bl](downloads/ch01-biology-and-matrix.bl) | 1 |
| Quality Control | [ch02-quality-control.bl](downloads/ch02-quality-control.bl) | 3 |
| Normalization and PCA | [ch03-normalization-pca.bl](downloads/ch03-normalization-pca.bl) | 4 |
| Integration | [ch04-integration.bl](downloads/ch04-integration.bl) | 2 |
| Clustering | [ch05-clustering.bl](downloads/ch05-clustering.bl) | 5 |
| Markers and Annotation | [ch06-markers.bl](downloads/ch06-markers.bl) | 4 |
| The Whole Workflow | [ch07-workflow.bl](downloads/ch07-workflow.bl) | 4 |

> **These are not quick.** Each block on a page is written to stand alone, so a
> reader can copy any one of them without having run the others — which means a
> concatenated chapter script rebuilds the pipeline from the raw matrix once per
> block. `ch05` does it five times, and adds a resolution sweep and a stability
> check on top. Expect minutes, not seconds, and expect `ch04` to be the slowest
> because Harmony runs on 30,043 cells.
>
> If you are working interactively, prefer the notebook below, or lift a single
> block rather than running a whole chapter.

## The notebook

**[hbc-scrnaseq.bln](downloads/hbc-scrnaseq.bln)** — prose and code together,
runnable end to end:

```sh
bl notebook hbc-scrnaseq.bln
```

Several chapters write `.svg` figures into the current directory. The notebook
carries the same content as the book, minus the figures, whose paths would not
resolve beside a downloaded file.

## What you cannot download here

The dataset itself. It belongs to the study authors and is hosted by the HBC
training team; `get-data.py` fetches it from them rather than redistributing a
copy. See [Attribution and Licence](attribution.md).
