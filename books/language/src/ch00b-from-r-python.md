# Coming from R or Python

If you already write dplyr or pandas, you know most of BioLang's table
vocabulary. The verbs were chosen to match what you type today, so this chapter
is a lookup table rather than a tutorial: find the thing you already know, read
across, keep working.

The pipe is `|>`. It does what `%>%` does in magrittr and what method chaining
does in pandas: the value on the left becomes the first argument on the right.

```biolang
let summary = read_csv("counts.csv")
    |> filter(|row| row.padj < 0.05)
    |> select("gene", "log2FoldChange")
    |> rename("log2FoldChange", "lfc")
    |> arrange("-lfc")
    |> head(20)
```

## Table verbs

| dplyr | pandas | BioLang |
| --- | --- | --- |
| `filter()` | `df[mask]` / `df.query()` | `filter()` |
| `select()` | `df[[cols]]` | `select()` |
| `mutate()` | `df.assign()` | `mutate()` |
| `rename()` | `df.rename()` | `rename()` |
| `arrange()` | `df.sort_values()` | `arrange()` — prefix `-` for descending |
| `group_by()` | `df.groupby()` | `group_by()` |
| `summarise()` | `.agg()` | `summarize()` |
| `count()` | `.value_counts()` | `count_by()` or `value_counts()` |
| `distinct()` | `.drop_duplicates()` | `distinct()` |
| `slice_head(n)` | `.head(n)` | `head(n)` |
| `slice_tail(n)` | `.tail(n)` | `tail(n)` |
| `bind_rows()` | `pd.concat()` | `concat()` |
| `bind_cols()` | `pd.concat(axis=1)` | `bind_cols()` |
| `left_join()` | `.merge(how="left")` | `left_join()` |
| `inner_join()` | `.merge()` | `inner_join()` |
| `anti_join()` | — | `anti_join()` |
| `pivot_longer()` | `.melt()` | `pivot_longer()` |
| `pivot_wider()` | `.pivot()` | `pivot_wider()` |
| `across()` | `.apply()` | `map()` over columns |

## Inspecting a table

| R | Python | BioLang |
| --- | --- | --- |
| `glimpse()` / `str()` | `df.info()` | `glimpse()` |
| `summary()` | `df.describe()` | `describe()` |
| `head()` | `df.head()` | `head()` |
| `dim()` | `df.shape` | `nrow()` and `ncol()` |
| `nrow()` / `ncol()` | `len(df)` / `df.shape[1]` | `nrow()` / `ncol()` |
| `names()` | `df.columns` | `colnames()` |

`glimpse()` prints one line per column with its type and first values, the way
`dplyr::glimpse` does. `describe()` gives the per-column dtype, null count, and
five-number summary that `pandas.describe` gives. Printing a table shows column
types under the headers, like a tibble.

Two names look familiar but mean something else. BioLang's `str()` converts a
value to a string — it is not R's structure display; use `glimpse()` for that.
`dim()` is for matrices; for tables use `nrow()` and `ncol()`.

## Reading and writing

| R | Python | BioLang |
| --- | --- | --- |
| `read.csv()` | `pd.read_csv()` | `read_csv()` |
| `readr::read_tsv()` | `pd.read_table()` | `read_tsv()` |
| `write.csv()` | `.to_csv()` | `write_csv()` |
| `Biostrings::readDNAStringSet()` | `SeqIO.parse()` | `read_fasta()` |
| `ShortRead::readFastq()` | `SeqIO.parse(..., "fastq")` | `read_fastq()` |
| `VariantAnnotation::readVcf()` | `pysam.VariantFile()` | `read_vcf()` |
| `rtracklayer::import()` | `pybedtools.BedTool()` | `bed()` / `gff()` |

## Things that work differently

**Sequences are a type, not a string.** `dna"ACGT"` is a first-class value with
`gc_content`, `reverse_complement`, and `translate` defined on it. You do not
need Biostrings or Biopython to do the obvious things.

**There is no `<-`.** Bindings use `let`, and `const` for values that must not
change.

**Anonymous functions use `|x|`.** Where R writes `\(x) x + 1` or `~ .x + 1`,
and Python writes `lambda x: x + 1`, BioLang writes `|x| x + 1`.

**Indices start at 0**, as in Python, not at 1 as in R.

**Missing values are `nil`**, not `NA` or `NaN`. Comparisons against `nil` are
explicit rather than contagious.

## Bringing existing code

`bl import` converts Python, R, Jupyter notebooks, and R Markdown:

```bash
bl import analysis.R --output analysis.bl
bl import pipeline.ipynb --output pipeline.bln
```

Anything the converter cannot translate is left in place with a `# TODO:`
marker naming what is missing, and the workbench lists those markers as
problems so you can see exactly how much is left to port.

## Keeping the parts that are not ported yet

You do not have to move everything at once. `py()` and `r()` evaluate a snippet
in a real Python or R process and bring the result back:

```biolang
let counts = read_csv("counts.csv")
let shrunk = r("DESeq2::lfcShrink(dds, coef=2, type='apeglm')", {dds: counts})
let embedding = py("import scanpy as sc; sc.tl.umap(adata)", {adata: cells})
```

Bindings are a record — `{dds: counts}` — and each key becomes a variable in the
other runtime. The value of the last expression comes back, which is what a
notebook cell does in Python and what `eval` does in R.

Values cross the boundary as JSON, so tables, records, lists, numbers, strings,
and booleans travel in both directions. A BioLang table arrives as a list of
records in Python and as a data frame in R. Objects that only exist inside the other
runtime — a Seurat object, an open file handle — do not, and you will get a
clear error rather than a silent truncation. Keep those calls narrow: pass data
in, get data out.

This is meant as a bridge. Delete the escape hatches as BioLang grows the
equivalents, and call `interop_status()` to see which interpreters were found.
`r()` needs the `jsonlite` package, and says so if it is missing.
