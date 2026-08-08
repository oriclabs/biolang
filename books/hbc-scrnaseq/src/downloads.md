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
bl run ch01.bl
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
| The Biology and the Matrix | [ch01.bl](downloads/ch01.bl) | 1 |
| Quality Control | [ch02.bl](downloads/ch02.bl) | 3 |
| Normalization and PCA | [ch03.bl](downloads/ch03.bl) | 4 |
| Integration | [ch04.bl](downloads/ch04.bl) | 2 |
| Clustering | [ch05.bl](downloads/ch05.bl) | 4 |
| Markers and Annotation | [ch06.bl](downloads/ch06.bl) | 4 |
| The Whole Workflow | [ch07.bl](downloads/ch07.bl) | 4 |

> **These are not quick, and here is what they actually cost.** Measured on a
> laptop, single-threaded:
>
> | Script | Time |
> |---|---|
> | `ch01` | 2 s |
> | `ch02` | ~1 min |
> | `ch03` | ~2 min |
> | `ch04` | 3 min 35 s |
> | `ch05` | 3 min 50 s |
> | `ch06` | **over 9 min** |
> | `ch07` | **over 9 min** |
>
> Two things drive this. Each block on a page is written to stand alone, so a
> reader can copy any one without having run the others — which means a
> concatenated chapter rebuilds the pipeline from the raw matrix once per block.
> And two operations are inherently expensive on 15,000 cells: `plot_markers`
> runs a test per gene per cluster and takes **5 minutes** by itself, and
> `cluster_diagnostics` computes a silhouette and takes **3**.
>
> `ch06` and `ch07` were confirmed to run only in pieces, not as single scripts
> inside a ten-minute budget. Nothing in them is broken — every block produced
> the figure on its page — but if you want to follow along interactively, lift a
> single block rather than running a whole chapter.

## The course-aligned pipeline

**[aligned.bl](downloads/aligned.bl)** — the course's settings rather than this
book's: their four-criterion QC filter, SCTransform, 40 PCs, resolution 0.8. It
prints both cluster counts side by side so you can see what the parameters cost:

```text
cells: 14847
SCTransform / 40 PCs / res 0.8 -> 12 clusters
log1p / 30 PCs / res 0.5      -> 10 clusters
```

About 75 seconds.

**[exact.bl](downloads/exact.bl)** — the full integrated configuration: both
samples, their filter, SCTransform, Harmony, 40 PCs, resolution 0.8:

```text
merged: 29629 cells
clusters: 16   (HBC reports 17)
```

Three and a half minutes, and it peaks at about 6.3 GB. **One cluster short of
the course**, and [What Differs from the Course](differences.md) explains what
changed and why the earlier claim of an exact match did not survive scrutiny.

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
