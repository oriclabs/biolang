# Chapter 1: Getting Started

BioLang is a pipe-first domain-specific language built for bioinformatics workflows.
This chapter walks you through installation, the interactive REPL, running scripts,
and writing your first real analysis.

## Installation

### From crates.io

```bash
cargo install biolang
```

This installs the `bl` binary, which provides both the REPL and the script runner.

### From source

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
# BioLang v0.3.0
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
BioLang v0.3.0
Type :help for commands, :quit to exit.
bl>
```

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
  |> filter(|r| mean(r.quality) >= min_qual)

print(f"Passing reads: {len(reads)}")
print(f"Mean quality: {reads |> map(|r| mean(r.quality)) |> mean()}")
```

## Your First Script: FASTA GC Content Analyzer

BioLang includes sample data in `examples/sample-data/` — see the
[Introduction](./introduction.md#sample-data) for the full list. The script
below uses `examples/sample-data/contigs.fa`.

Create a file called `gc_scan.bl`:

```biolang
# gc_scan.bl
# Read a FASTA file, compute per-sequence GC content, report statistics.

let sequences = read_fasta("data/sequences.fasta")

# Compute GC content for each sequence
let gc_table = sequences
  |> map(|seq| {
    name: seq.id,
    length: seq_len(seq.seq),
    gc: gc_content(seq.seq)
  })
  |> table()

# Summary statistics
let gc_vals = gc_table |> select("gc")
let mean_gc = mean(gc_vals)
let std_gc = stdev(gc_vals)
let min_gc = min(gc_vals)
let max_gc = max(gc_vals)
let n_seqs = len(gc_vals)

print(f"Analyzed {n_seqs} sequences")
print(f"GC content: {mean_gc:.3f} (range: {min_gc:.3f} - {max_gc:.3f})")
print(f"Standard deviation: {std_gc:.4f}")

# Flag outlier contigs (GC > 2 std devs from mean)
# |> into binds the pipe result to a variable (like let, but reads left-to-right)
gc_table
  |> filter(|row| abs(row.gc - mean_gc) > 2.0 * std_gc)
  |> sort("gc", descending: true)
  |> into outliers

print(f"\nOutlier contigs ({len(outliers)}):")
outliers |> each(|row| print(f"  {row.name}: GC={row.gc:.3f}, length={row.length}"))
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
bl init my-rnaseq-pipeline
cd my-rnaseq-pipeline
```

This creates the following structure:

```
my-rnaseq-pipeline/
  .biolang/
    config.yaml       # project configuration
    plugins/           # local plugin overrides
  src/
    main.bl            # entry point
  data/                # input data directory
  results/             # output directory
```

### `.biolang/config.yaml`

```yaml
name: my-rnaseq-pipeline
version: 0.3.0
entry: src/main.bl

paths:
  data: ./data
  results: ./results
  reference: /shared/references/GRCh38

defaults:
  min_quality: 30
  threads: 8
```

Access project config values in your scripts:

```biolang
# src/main.bl
# Access project paths via import
import "src/paths.bl" as paths

let min_qual = 30

read_fastq(f"{paths.data}/sample_R1.fastq.gz")
  |> filter(|r| mean(r.quality) >= min_qual)
  |> write_fastq(f"{paths.results}/filtered_R1.fastq.gz")
```

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
2. Project `.biolang/plugins/` directory
3. Each directory in `BIOLANG_PATH`
4. System-wide library path (`~/.biolang/lib/`)

This is useful for sharing utility modules across projects:

```biolang
# This resolves via BIOLANG_PATH if not found locally
import "genomics_utils.bl" as gutils

let kmers = dna"ATCGATCGATCG" |> gutils.kmer_frequencies(k: 3)
print(kmers)
```

## What's Next

You now have BioLang installed, know how to use the REPL for interactive exploration,
and can write and run scripts. In the next chapter, we will explore bio literals --
the first-class DNA, RNA, protein, and quality score types that make BioLang unique
for bioinformatics work.
