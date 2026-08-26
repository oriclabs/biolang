# Chapter 1: Getting Started

BioLang is a pipe-first domain-specific language built for bioinformatics workflows.
This chapter walks you through installation, the interactive REPL, running scripts,
and writing your first real analysis.

## Installation

The quickest route needs nothing installed beforehand — not even Rust.

### Linux and macOS

```bash
curl -fsSL https://lang.bio/install.sh | sh
```

### Windows

```powershell
iwr -useb https://lang.bio/install.ps1 | iex
```

Either one works out your platform and architecture, downloads that build from
the latest GitHub release, checks it against the `checksums.sha256` published
alongside it, and installs two binaries: `bl`, which is both the REPL and the
script runner, and `bl-lsp`, the language server your editor talks to.

They install to `/usr/local/bin` on Linux and macOS, and to
`%LOCALAPPDATA%\Programs\BioLang\bin` on Windows — the Windows location needs no
administrator rights and is added to your user `PATH` for you. Set
`BIOLANG_INSTALL_DIR` to put them somewhere else.

### With Cargo

If you already have Rust 1.75 or newer:

```bash
cargo install --git https://github.com/oriclabs/biolang bl-cli
```

This builds from source, so it takes a few minutes where the installers take
seconds. BioLang is not published to crates.io yet, which is why this is a
`--git` install rather than `cargo install biolang`.

### From a clone

What you want if you intend to change anything:

```bash
git clone https://github.com/oriclabs/biolang.git
cd biolang
cargo build --release
cp target/release/bl ~/.local/bin/
```

### Verify installation

```bash
bl --version
```

### Updating

BioLang has built-in update checking. Run `bl version` to see the current version
and check if a newer release is available:

```bash
bl version
# BioLang v1.2.0
#
# Checking for updates... up to date.
```

To upgrade to the latest release:

```bash
bl upgrade
```

This downloads the correct binary for your platform from GitHub Releases and
replaces the current `bl` executable.

BioLang also checks for updates automatically in the background when you run
`bl run` or `bl repl`. If a newer version is available, a one-line notice
appears on stderr. This check runs at most once per 24 hours and never blocks
startup. Disable it with:

```bash
export BIOLANG_NO_UPDATE_CHECK=1
```

## The REPL

Launch the interactive REPL:

```bash
bl
```

You will see the BioLang prompt:

```
  ____  _       _
 | __ )(_) ___ | |    __ _ _ __   __ _
 |  _ \| |/ _ \| |   / _` | '_ \ / _` |
 | |_) | | (_) | |__| (_| | | | | (_| |
 |____/|_|\___/|_____\__,_|_| |_|\__, |
                                  |___/
 BioLang — pipe-first bioinformatics DSL
 v1.2.0  •  1018 builtins  •  NCBI[-] LLM[+]  Cache: 0B

 Commands:  :help  :builtins  :quit  ?name  Tab for completion  •  Paste DNA/FASTA auto-detected

bl>
```

`NCBI[-]` means no `NCBI_API_KEY` is set, and `LLM[+]` that an LLM provider is
configured. Both reflect your environment, so your banner may differ.

Try evaluating a bio literal directly:

```
bl> dna"ATCGATCG" |> gc_content()
0.5
```

### REPL Commands

The REPL supports several meta-commands, all prefixed with `:`.

#### `:env` -- Inspect current bindings

```
bl> let ref_genome = "GRCh38"
bl> let min_mapq = 30
bl> :env
ref_genome : Str = "GRCh38"
min_mapq   : Int = 30
```

#### `:reset` -- Clear all bindings

```
bl> :reset
Environment cleared.
```

#### `:load` and `:save` -- Session persistence

Load a script into the current session, executing every statement:

```
bl> :load preprocessing.bl
Loaded 42 bindings from preprocessing.bl
```

Save the current session bindings to a file:

```
bl> :save my_session.bl
Saved 12 bindings to my_session.bl
```

#### `:time` -- Benchmark an expression

```
bl> :time read_fastq("data/reads.fastq") |> filter(|r| mean(r.quality) >= 30) |> len()
Result: 1847293
Elapsed: 4.38s
```

#### `:type` -- Check the type of an expression

```
bl> :type dna"ATCG"
DNA
bl> :type {chrom: "chr1", start: 100, end: 200}
Record{chrom: Str, start: Int, end: Int}
```

#### `:plot` -- Choose how plots appear

A plot expression keeps its full SVG value, but the REPL previews it as a
compact Unicode chart instead of printing the SVG markup:

```text
bl> histogram([2, 3, 3, 4, 7, 8, 8, 9])
Histogram
  ⠀⠀⠀⠀⠀⢀⣀⠀⠀⠀⠀
  ⠀⢀⣀⣀⣸⣿⣀⣀⡀⠀
SVG kept in `_`; use save_plot(_, "plot.svg").
```

Use `:plot` with a mode to change later plots and redraw the last plot:

```text
:plot auto       # Unicode in a terminal; original SVG when output is redirected
:plot unicode    # high-resolution Braille preview
:plot ascii      # portable ASCII preview
:plot file       # write biolang-plots/plot-NNN.svg
:plot open       # write the SVG and open it with the platform viewer
:plot raw        # print the original SVG markup explicitly
:plot none       # suppress the plot (a note goes to standard error)
:plot status     # show the current mode and output directory
```

With no argument, `:plot` redraws the last SVG plot. For compatibility,
`:plot 20` draws a 20-bin ASCII histogram when the last result is a numeric
list or quality vector. The same display policy can be selected before starting
BioLang:

```bash
bl --plot ascii
bl --plot file --plot-dir results
bl run analysis.bl --print-result --plot unicode
```

When standard output is redirected, `auto` writes the original SVG, exactly as
earlier versions did, so `bl run figure.bl --print-result > figure.svg` keeps
working unchanged. Terminal graphics appear only when standard output is a
terminal, which is the only place they can be read.

Status lines -- where `:plot file` wrote a figure, why a preview could not be
drawn, that `:plot none` suppressed one -- go to standard error, so they never
mix into redirected output. Choosing `unicode` or `ascii` explicitly does draw
into a redirected stream, on the grounds that you asked for it.

#### `:plugins` -- List available plugins

```
bl> :plugins
fastq      read_fastq, write_fastq
fasta      read_fasta, write_fasta
sam        read_sam, read_bam
vcf        read_vcf, write_vcf
bed        read_bed, write_bed
table      csv, tsv, write_tsv
```

#### `:profile` -- Profile an expression

```
bl> :profile read_fasta("data/sequences.fasta") |> filter(|r| seq_len(r.seq) > 1000) |> len()
Total:     2.14s
  read:    1.87s (87.4%)
  filter:  0.26s (12.1%)
  len:     0.01s (0.5%)
Result: 24891
```

## Running Scripts

BioLang scripts use the `.bl` extension. Run a script with:

```bash
bl run gc_analysis.bl
```

Pass arguments to a script:

```bash
bl run qc_report.bl -- --input sample.fastq.gz --min-quality 20
```

Arguments are available inside the script via the `args` record:

```biolang
# qc_report.bl
let input_file = args.input
let min_qual = into(args.min_quality ?? "20", "Int")

let reads = read_fastq(input_file)
  |> filter(|r| mean_phred(r.quality) >= min_qual)

println(f"Passing reads: {len(reads)}")
println(f"Mean quality: {reads |> map(|r| mean_phred(r.quality)) |> mean()}")
```

## Your First Script: FASTA GC Content Analyzer

BioLang includes sample data in `examples/sample-data/` — see the
[Introduction](./introduction.md#sample-data) for the full list. The script
below uses `examples/sample-data/contigs.fa`.

Create a file called `gc_scan.bl`:

```biolang
# gc_scan.bl
# Read a FASTA file, compute per-sequence GC content, report statistics.

let sequences = read_fasta("examples/sample-data/contigs.fa")

# Compute GC content for each sequence
let gc_table = sequences
  |> map(|seq| {
    name: seq.id,
    length: seq_len(seq.seq),
    gc: gc_content(seq.seq)
  })
  |> table()

# Summary statistics
let gc_vals = col(gc_table, "gc")
let mean_gc = mean(gc_vals)
let std_gc = stdev(gc_vals)
let min_gc = min(gc_vals)
let max_gc = max(gc_vals)
let n_seqs = len(gc_vals)

println(f"Analyzed {n_seqs} sequences")
println(f"GC content: {mean_gc:.3f} (range: {min_gc:.3f} - {max_gc:.3f})")
println(f"Standard deviation: {std_gc:.4f}")

# Flag outlier contigs (GC > 2 std devs from mean)
# |> into binds the pipe result to a variable (like let, but reads left-to-right)
gc_table
  |> filter(|row| abs(row.gc - mean_gc) > 2.0 * std_gc)
  |> sort_by(|row| -row.gc)
  |> into outliers

println(f"\nOutlier contigs ({len(outliers)}):")
outliers |> each(|row| println(f"  {row.name}: GC={row.gc:.3f}, length={row.length}"))
```

Run it:

```bash
bl run gc_scan.bl
```

Example output:

```
Analyzed 847 sequences
GC content: 0.412 (range: 0.198 - 0.687)
Standard deviation: 0.0531

Outlier contigs (12):
  contig_441: GC=0.687, length=3421
  contig_002: GC=0.621, length=15789
  ...
```

## Project Structure

Initialize a BioLang project:

```bash
mkdir my-rnaseq-pipeline
cd my-rnaseq-pipeline
bl init --name my-rnaseq-pipeline
```

This creates the following structure:

```
my-rnaseq-pipeline/
  biolang.toml       # package metadata and dependencies
  main.bl            # entry point
```

Create project-specific directories and modules when the analysis needs them:

```text
my-rnaseq-pipeline/
  biolang.toml
  main.bl
  src/
    paths.bl
    qc.bl
  data/
  results/
```

The manifest records package metadata and optional path or Git dependencies:

```toml
[package]
name = "my-rnaseq-pipeline"
version = "0.1.0"

[dependencies]
shared-qc = { path = "../shared-qc" }
variants = { git = "https://github.com/example/variants.git", branch = "main" }
```

Run `bl install` in the project directory to install the declared dependencies.

### Multi-file projects

Use `import` to split your pipeline across files:

```biolang
# src/main.bl
import "src/qc.bl" as qc
import "src/alignment.bl" as align
import "src/variant_calling.bl" as vc

let samples = csv("data/sample_sheet.csv")

samples |> each(|sample| {
  let cleaned = qc.run(sample.fastq_r1, sample.fastq_r2)
  let bam = align.run(cleaned.r1, cleaned.r2, sample.reference)
  vc.run(bam, sample.reference, sample.sample_id)
})
```

```biolang
# src/qc.bl
let run = |r1, r2| {
  let filt_r1 = read_fastq(r1) |> filter(|r| mean(r.quality) >= 30) |> write_fastq(f"{r1}.filtered.fq.gz")
  let filt_r2 = read_fastq(r2) |> filter(|r| mean(r.quality) >= 30) |> write_fastq(f"{r2}.filtered.fq.gz")
  {r1: filt_r1, r2: filt_r2}
}
```

## BIOLANG_PATH

The `BIOLANG_PATH` environment variable controls where BioLang searches for imported
modules and plugins. It accepts a colon-separated (or semicolon on Windows) list of
directories:

```bash
export BIOLANG_PATH="/home/user/biolang-libs:/shared/team-modules"
```

Resolution order for `import "module.bl"`:

1. Relative to the importing file
2. The current working directory
3. Each directory in `BIOLANG_PATH`
4. `~/.biolang/stdlib/`
5. `~/.biolang/packages/`

This is useful for sharing utility modules across projects:

```biolang
# This resolves via BIOLANG_PATH if not found locally
import "genomics_utils.bl" as gutils

let kmers = dna"ATCGATCGATCG" |> gutils.kmer_frequencies(k: 3)
println(kmers)
```

## What's Next

You now have BioLang installed, know how to use the REPL for interactive exploration,
and can write and run scripts. In the next chapter, we will explore bio literals --
the first-class DNA, RNA, protein, and quality score types that make BioLang unique
for bioinformatics work.
