# Chapter 17: Literate Notebooks

BioLang notebooks (`.bln` files) combine Markdown prose with executable code blocks.
They're ideal for documenting analyses, creating reproducible reports, and sharing
methods with collaborators.

## Running a Notebook

```bash
bl notebook analysis.bln
```

Prose sections are rendered with ANSI formatting in the terminal. Code blocks
are executed in order, with output interleaved between prose.

## Native Local Notebook

Start the lightweight, Jupyter-like browser editor with a native BioLang kernel:

```bash
bl notebook serve analysis.bln
```

`bl` opens a page served from a random loopback port. Each notebook session owns
one interpreter, so a variable created by one cell is available to later cells.
Unlike the WebAssembly export, the local kernel can use native packages, local
files, larger memory, and the configured CPU or GPU backend. The page can edit
and save the original `.bln`, run through one selected cell, or run all cells in
a fresh session. Selecting a later cell automatically runs any missing earlier
cells, so its setup variables exist. Editing or rerunning an earlier cell starts
a clean session before rebuilding that context.

Useful launch options:

```bash
bl notebook serve analysis.bln --no-open
bl notebook serve analysis.bln --port 8765
bl --no-gpu notebook serve analysis.bln
bl notebook serve analysis.bln --root ./study
```

The server accepts loopback addresses only and generates a new bearer token for
each launch. It implements the common SOMER job routes for compatible stateless
clients, plus `/v1/notebook-sessions` for contextual cell execution. It is a
single-user local kernel, not a scheduler or remote SOMER replacement. Native
notebook code runs with the permissions of the user who started `bl`.

## The .bln Format

A `.bln` file is plain text. Everything outside a code block is Markdown prose.
Code blocks can use either fenced syntax or dash delimiters.

### Fenced Code Blocks

Use standard Markdown fences with `biolang`, `bl`, or no language tag:

````text
## Load Data

Read the FASTQ file and compute basic stats.

```biolang
let reads = read_fastq("data/reads.fastq")
println(f"Loaded {len(reads)} reads")
```

## Filter

Keep high-quality reads.

```bl
let filtered = reads |> filter(|r| mean_phred(r.quality) > 25)
println(f"Kept {len(filtered)} reads")
```
````

### Dash Delimiters

The original `.bln` syntax uses `---` on its own line:

```text
## Load Data
---
let reads = read_fastq("data/reads.fastq")
print(f"Loaded {len(reads)} reads")
---
## Results
The output above shows the read count.
```

Both styles can be mixed in the same notebook. Fenced blocks with other language
tags (e.g. ` ```python `) are treated as prose and not executed.

### State Carries Over

Variables defined in one code block are available in all later blocks. This lets
you build up an analysis step by step, explaining each part in prose.

## Cell Directives

Add special comment annotations at the top of a code block to control behavior:

| Directive | Effect |
|---|---|
| `# @hide` | Execute but don't display the code |
| `# @skip` | Don't execute this block |
| `# @echo` | Print the code before executing |
| `# @hide-output` | Execute (and show code) but suppress printed output |

Directives are stripped from the code before execution. Multiple directives can
be combined:

````text
```biolang
# @hide
let config = {min_quality: 30, reference: "GRCh38"}
```

```biolang
# @echo
let reads = read_fastq("data/reads.fastq")
  |> filter(|r| mean_phred(r.quality) >= config.min_quality)
println(f"Filtered: {len(reads)} reads")
```
````

With `# @hide`, the config block runs silently. With `# @echo`, readers see both
the code and its output.

## HTML Export

Generate a self-contained HTML report:

```bash
bl notebook analysis.bln --export html > report.html
```

The HTML output includes:
- Inline CSS with a dark theme
- BioLang syntax highlighting (keywords, strings, comments, pipes)
- Rendered Markdown prose
- Code output blocks
- No external dependencies -- a single standalone file

This is useful for sharing results via email or publishing to a website.

## Jupyter Interop

### Import from Jupyter

Convert an existing `.ipynb` notebook to `.bln`:

```bash
bl notebook experiment.ipynb --from-ipynb > experiment.bln
```

- Markdown cells become prose sections
- Code cells become fenced `biolang` blocks
- Outputs are discarded (they'll be regenerated)

### Export to Jupyter

Convert a `.bln` to `.ipynb`:

```bash
bl notebook analysis.bln --to-ipynb > analysis.ipynb
```

- Prose becomes Markdown cells
- Code blocks become code cells with `"language": "biolang"` metadata
- Uses nbformat v4, compatible with JupyterLab and VS Code

## Example: GC Content Report

Here's a complete notebook that analyzes GC content:

````text
# GC Content Analysis

This notebook computes per-contig GC content and flags outliers.

## Setup

```biolang
# @hide
let threshold = 2.0
```

## Load Data

```biolang
let seqs = read_fasta("data/sequences.fasta")
println(f"Loaded {len(seqs)} sequences")
```

## Statistics

```biolang
# @echo
let gc_values = seqs |> map(|s| gc_content(s.seq))
let mu = mean(gc_values)
let sigma = stdev(gc_values)
println(f"Mean GC: {mu:.3f} +/- {sigma:.4f}")
```

## Outliers

Contigs more than 2 std devs from the mean may indicate
contamination or horizontal gene transfer.

```biolang
let outliers = seqs
  |> filter(|s| abs(gc_content(s.seq) - mu) > threshold * sigma)
println(f"Found {len(outliers)} outlier contigs")
```
````

Run it:

```bash
bl notebook gc_analysis.bln
```

Export it:

```bash
bl notebook gc_analysis.bln --export html > gc_report.html
```

## Tips

- **Use `# @hide` for setup** -- configuration, imports, and helper functions
  that readers don't need to see
- **Use `# @echo` for key steps** -- shows both code and output for the important
  analysis logic
- **Use `# @skip` during development** -- temporarily disable expensive cells
- **Use `bl notebook serve` for native interactive work** -- cell state, saving,
  native packages, and large local analyses without a remote service
- **The HTML export is self-contained** -- share the `.html` file directly
- **Convert existing Jupyter notebooks** with `--from-ipynb` to migrate analyses
