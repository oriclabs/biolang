# The HBC Single-cell Course, in BioLang

The Harvard Chan Bioinformatics Core teaches a fourteen-lesson introduction to
single-cell RNA-seq. It is one of the better free curricula in the field, it is
taught in R with Seurat, and it is released under CC BY 4.0.

This book walks the same road in BioLang, on the same data.

## Why follow someone else's syllabus

Because the ordering is the hard part, and theirs is good.

A tool's documentation organises itself around the tool: here are the functions,
here is what each takes. A course organises itself around the learner: here is
what you cannot understand until you understand this other thing first. HBC puts
the theory of PCA *before* normalization, which looks backwards until you notice
you cannot judge whether a normalization worked without a way to look at the
result. They give cluster quality control an entire lesson, long after
clustering, because "are these clusters real?" only becomes answerable once you
have some.

## Why on their data

An earlier version of this material ran on a small synthetic fixture. It was
fast, it needed no download, and it was wrong in a way that mattered: the genes
were named `MARK0_000`, `MARK1_000`, and so on. Every mechanical step worked, and
the one thing the annotation lesson exists to teach — looking at `LYZ`, `CD14`
and `S100A8` together and concluding *monocytes* — could not be demonstrated at
all.

So this book uses the course's own PBMC dataset. It costs a download and some
runtime. In exchange, the figures show recognisable immune biology, and every
number can be compared against what the course reports.

It also surfaced things a tidy fixture never would. Real clusters range from
3,128 cells to 32, and that unevenness alone found a latent crash in the violin
plots. Real raw matrices have 737,280 droplets, which found a plotting path that
never terminated. Synthetic data tests the code you wrote; real data tests the
assumptions you did not know you had made.

## What you need

BioLang, the `singlecell` package, and about 90 MB of extracted matrices from a
3.2 GB archive. All three steps are in [Getting the Data](setup.md).

## What this is not

It is not a replacement for the course. Read the HBC lessons for the biology and
the reasoning; read these chapters when you want to run the same steps here. It
is also not a claim that BioLang reproduces Seurat exactly — it does not, and
[What Differs from the Course](differences.md) is specific about where and why.

Credit, licence, and the full list of changes are on
[Attribution and Licence](attribution.md). Read that first if you plan to reuse
any of this.

## A note on the code

None of it runs in the browser. Every example imports a package and most read or
write files, and package imports and file I/O are CLI-only — so these pages show
no Run button. Copy the code, or take the scripts and notebook from
[Downloads](downloads.md).
