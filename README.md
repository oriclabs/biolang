# BioLang

> **Warning**: BioLang is experimental and under active development. The language syntax, builtins, and APIs may change without notice between releases. Not recommended for production use yet. Feedback and bug reports are welcome.

A pipe-first domain-specific language (DSL) for bioinformatics.

BioLang is a DSL purpose-built for genomics and molecular biology. It brings
first-class biological types, 1000+ domain builtins, 21 bio API clients, and composable pipelines
to bioinformatics workflows. Write analysis scripts that read like the science
they describe.

```
let reads = read_fastq("sample_R1.fq.gz")
  |> filter(|r| mean_phred(r.quality) >= 25)
  |> collect()

let gc = reads |> map(|r| gc_content(r.seq)) |> mean()
print("Mean GC: " + str(gc))

reads
  |> filter(|r| len(r.seq) >= 50)
  |> write_fastq("filtered.fq.gz")
```

## Workbench

An editor, file tree, console and plots, running the same interpreter as the CLI compiled to
WebAssembly.

- **[In the browser](https://lang.bio/workbench/)** — nothing to install. Example packs install
  into the workspace from a single link, along with the sample data the documentation reads.
- **On the desktop** — the same workbench as a native app, with your own filesystem, the `bl`
  CLI, remote execution over SSH, and the APIs a browser cannot reach for want of CORS headers.

## Verified against Rosalind

[Rosalind](https://rosalind.info) is the bioinformatics problem set used in teaching
worldwide. These problems are not mine to choose, and the answers are not mine to mark: every
solution is asserted against the official answer and re-run on each commit — natively, and
again through the same WebAssembly build the website serves.

| Track | Covered | |
|---|---|---|
| [Armory](https://lang.bio/docs/examples/rosalind-armory.html) | **15 / 15** | complete |
| [Algorithmic Heights](https://lang.bio/docs/examples/rosalind-algorithmic-heights.html) | **34 / 34** | complete |
| [Stronghold](https://lang.bio/docs/examples/rosalind-stronghold.html) | **105 / 105** | complete |
| [Textbook Track](https://lang.bio/docs/examples/rosalind-textbook.html) | **124 / 124** | complete |

**278 problems; 274 are checked on every commit.** The other four reach out to NCBI or UniProt, so
they run in a separate advisory job rather than gating the build on a remote service being
up. All four tracks are complete.

Writing them is also what surfaced most of the language fixes in recent releases: `{}`
parsing as a block rather than an empty map, quadratic `push` and `unique`, and an
alignment builtin that could not reach a substitution matrix.

Solutions are MIT. Problem statements belong to rosalind.info — each example paraphrases
Given/Return in a line or two and links to the original rather than reproducing it.

## Checked against BioPython and Bioconductor

Rosalind answers the question "is this right against a published answer". The other half is
whether it agrees with the tools people already use. [`benchmarks/correctness/`](benchmarks/correctness)
holds 18 tasks — GC content, k-mer counts, VCF filtering, reverse complement, translation,
group-by aggregation, GFF feature counts, sequence stats, interval merges — each written three
times, in BioLang, in Python with BioPython, and in R with Bioconductor. All three emit JSON and
a recursive comparator checks them: floats to 1e-6, integers and strings exactly.

Nine run on generated data; the [other nine](benchmarks/correctness/real_world) use real genomes
and variant sets from NCBI, ClinVar and ENCODE, which is where the awkward cases live —
non-standard bases, multi-allelic variants, overlapping bacterial genes.

```bash
cd benchmarks/correctness && ./validate.sh          # generated data
python download_real_data.py && ./validate_real.sh  # real data (~25 MB)
```

Agreement is strong evidence, not proof: it adopts BioPython's and Bioconductor's conventions as
the reference, and a shared misreading of a format would stay invisible to it.

## Features

- **Bio-native types** -- DNA, RNA, Protein, Interval, Variant, Gene, AlignedRead, Quality
- **Pipe operator** -- `|>` inserts the left side as the first argument: `a |> f(b)` = `f(a, b)`
- **1000+ builtins** -- FASTQ/FASTA/VCF/BED/GFF I/O, sequence ops, statistics, genomic intervals, tables, 42 plot types
- **Pipe-first pipelines** -- compose operations with `|>`, `group_by`, `count_by`, `filter_by` for efficient data processing
- **21 Bio API clients** -- NCBI, Ensembl, UniProt, UCSC, KEGG, STRING, PDB, Reactome, GO, COSMIC, BioMart, NCBI Datasets, nf-core, BioContainers, Galaxy ToolShed
- **SQLite** -- built-in database for storing and querying results
- **Notifications** -- Slack, Teams, Telegram, Discord, email alerts from pipelines
- **Streams** -- lazy evaluation for large files without loading into memory
- **Tables** -- R-like data frames with filter, mutate, group_by, summarize, join
- **Knowledge graphs** -- `graph()`, `add_node()`, `add_edge()`, `shortest_path()`, `connected_components()`, `subgraph()`
- **Enrichment analysis** -- ORA with hypergeometric test, GSEA with permutation, BH correction, GMT parsing
- **PDB structures** -- fetch entries, chains, sequences from RCSB Protein Data Bank
- **PubMed** -- search articles and fetch abstracts directly from scripts
- **LLM chat** -- built-in `chat()` and `chat_code()` using Anthropic, OpenAI, or Ollama
- **BioContainers** -- pull and run 9,000+ containerized tools from your pipelines
- **Workflow catalog** -- search and view nf-core and Galaxy workflows
- **Literate notebooks** -- `.bln` format with Markdown + code, cell directives, HTML export, Jupyter import/export
- **Plugin system** -- extend with Python, TypeScript, R, or native plugins
- **Self-update** -- `bl version` checks for updates, `bl upgrade` downloads the latest release
- **LSP** -- language server with diagnostics, completion, and hover
- **Optional converter** -- the separate `bl-convert` executable validates and safely converts tabular, interval and sequence files without increasing `bl`'s size

## Install

### One line

```bash
# Linux and macOS
curl -fsSL https://lang.bio/install.sh | sh
```

```powershell
# Windows
iwr -useb https://lang.bio/install.ps1 | iex
```

Both detect your platform, verify the download against the `checksums.sha256`
published with the release, and install `bl` and `bl-lsp`. Set
`BIOLANG_INSTALL_DIR` to choose where they go — it defaults to `/usr/local/bin`
on Linux and macOS, and `%LOCALAPPDATA%\Programs\BioLang\bin` on Windows, which
needs no administrator rights.

### From releases

Pre-built binaries for every tagged release are on the
[Releases](https://github.com/oriclabs/biolang/releases) page, for Linux
(x86_64, aarch64), macOS (x86_64, Apple Silicon) and Windows (x86_64).

```bash
# Linux x86_64
curl -L https://github.com/oriclabs/biolang/releases/latest/download/biolang-linux-x86_64.tar.gz | tar xz
sudo mv bl /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/oriclabs/biolang/releases/latest/download/biolang-macos-aarch64.tar.gz | tar xz
sudo mv bl /usr/local/bin/
```

```powershell
# Windows
Invoke-WebRequest -Uri https://github.com/oriclabs/biolang/releases/latest/download/biolang-windows-x86_64.zip -OutFile biolang.zip
Expand-Archive biolang.zip -DestinationPath .
```

### From source

Needs Rust 1.82 or newer.

```bash
cargo install --git https://github.com/oriclabs/biolang bl-cli
```

Or from a clone, which is what you want if you intend to change anything:

```bash
git clone https://github.com/oriclabs/biolang.git
cd biolang
cargo install --path crates/bl-cli
```

The lightweight converter ships in the release archives beside `bl`, and can
also be built from the same source checkout. It stays a separate executable:

```bash
cargo install --path crates/bl-convert
bl-convert formats
# With both executables present, this delegates to bl-convert:
bl convert input.vcf output.bed
```

## Quick Start

```bash
# Run a script
bl run analysis.bl

# Interactive REPL
bl repl

# Discover and explicitly download registered data
bl data search "single cell" --category teaching
bl data info oriclabs/nhanes-bdsr-teaching
bl data fetch oriclabs/nhanes-bdsr-teaching
bl data path oriclabs/nhanes-bdsr-teaching

# Plot previews: auto, unicode, ascii, file, open, raw, or none
# `auto` draws in a terminal and leaves redirected output as SVG
bl --plot ascii
bl --plot file --plot-dir results

# Start language server (for editor integration)
bl lsp

# Run a literate notebook
bl notebook analysis.bln

# Export notebook to HTML
bl notebook analysis.bln --export html

# Check for updates
bl version

# Upgrade to the latest release
bl upgrade
```

### Reproducible recorded runs

For an analysis you may need to rerun, review, or describe in a methods
section, declare its inputs and outputs and save a machine-readable run record:

```bash
bl --no-gpu run analysis.bl \
  --record results/run.json \
  --input data/filtered_feature_bc_matrix \
  --output results/clusters.tsv \
  --param resolution=0.8 \
  --param label=treated \
  --seed 42
```

Parameters retain JSON types and are read inside the script without editing it:

```biolang
let resolution = run_param("resolution", 0.8)
let label = run_param("label", "sample")
```

Recorded parameters are stored verbatim. Do not pass passwords, tokens, or
other secrets with `--param`; use the environment or a credential provider.

The `biolang.run/v1` JSON records hashes for the script, imported BioLang
modules, executable, declared inputs, declared outputs, and nearest
`biolang.toml`, together with typed parameters, seed, CPU/GPU decision,
BioLang version, elapsed time, and peak resident memory. Missing declared
inputs stop the script before it can create outputs; a missing declared output
makes the completed run fail its postflight check.
Input discovery is deliberately explicit: a path is tracked only when passed
with `--input`. Keep the record beside, rather than inside, any declared input
or output directory; otherwise writing it would invalidate that directory's
hash and the CLI rejects the layout.

### Hello FASTQ

```
# hello.bl
let reads = read_fastq("sample.fq.gz") |> collect()
let total = len(reads)
let passing = reads |> filter(|r| mean_phred(r.quality) >= 30) |> len()
print("Total: " + str(total) + ", Passing Q30: " + str(passing))
```

```bash
bl run hello.bl
```

### Pipeline example

```
# Variant QC pipeline — sequential pipe-first style
let variants = read_vcf("calls.vcf") |> collect()
let filtered = variants |> filter_by("quality", ">=", 30)
let classified = filtered |> classify_variants()
let by_chrom = classified |> group_by("chrom")
let chrom_names = keys(by_chrom)

println(f"Total: {len(variants)}, Filtered: {len(filtered)}")
chrom_names |> each(|c| {
    let vs = by_chrom[c]
    let snps = vs |> filter_by("variant_type", "==", "SNP") |> len()
    println(f"  {c}: {len(vs)} variants ({snps} SNPs)")
})
```

## Language Highlights

### Bio literals

```
let seq = dna"ATCGATCG"
let rna_seq = rna"AUGCAUGC"
let protein = protein"MVLSPADKTNVKAAWGKVGAHAGEYGAEALERMFLSFPTTKTYFPHFDLSH"

gc_content(seq)        # 0.5
reverse_complement(seq) # DNA(CGATCGAT)
translate(rna_seq)     # Protein(MH)
```

### Tables

```
let samples = read_tsv("samples.tsv")
samples
  |> filter(|r| r.depth > 30)
  |> mutate("pass_rate", |r| r.passing / r.total * 100)
  |> group_by("cohort")
  |> summarize(|key, rows| { mean_depth: mean(col(rows, "depth")) })
  |> arrange("-mean_depth")
  |> print()
```

### Genomic intervals

```
let exons = read_bed("exons.bed")
let peaks = read_bed("peaks.bed")
let overlaps = interval_intersect(exons, peaks)
print("Peaks overlapping exons: " + str(len(overlaps)))
```

### API queries

```
# ncbi_gene returns a Record only when the search matches exactly one gene.
# A bare symbol matches it across organisms, so it returns the list of ids;
# pass a limit to get the summary Record, and prefer a qualified query when
# you mean one particular gene.
let ids = ncbi_gene("BRCA1")
print(len(ids))

let gene = ncbi_gene("BRCA1[sym] AND human[orgn]", 1)
print(gene.symbol + ": " + gene.description)

# ensembl_vep queries Ensembl's HGVS endpoint, so it takes HGVS notation
# rather than a colon-delimited position.
let variants = ensembl_vep("ENST00000269305.9:c.215C>G")
print(len(variants))
```

### Knowledge graphs

```
# Build a protein interaction network
let g = graph()
let g = add_edge(g, "BRCA1", "TP53", {score: 0.99})
let g = add_edge(g, "TP53", "MDM2", {score: 0.97})
let g = add_edge(g, "BRCA1", "BARD1", {score: 0.95})

neighbors(g, "BRCA1")       # ["BARD1", "TP53"]
shortest_path(g, "MDM2", "BARD1")  # ["MDM2", "TP53", "BRCA1", "BARD1"]
degree(g, "BRCA1")          # 2
```

### Enrichment analysis

```
let gene_sets = read_gmt("hallmark.gmt")
let my_genes = ["BRCA1", "TP53", "CDK2", "CCND1", "RB1"]
let results = enrich(my_genes, gene_sets, 20000)
results |> filter(|r| r.fdr < 0.05) |> print()
```

### PDB structures

```
let entry = pdb_entry("4HHB")
print(entry.title)          # "THE CRYSTAL STRUCTURE OF HUMAN DEOXYHAEMOGLOBIN"
let chains = pdb_chains("4HHB")
chains |> each(|c| print(c.description + ": " + str(len(c.sequence)) + " residues"))
```

### LLM chat

```
# Ask an LLM about your data (Anthropic, OpenAI, or Ollama)
let variants = read_vcf("filtered.vcf") |> collect()
let snps = variants |> filter(|v| is_snp(v)) |> len()

let answer = chat("I found " + str(snps) + " SNPs in my VCF. What's a typical Ti/Tv ratio for exome data?")
println(answer)

# Generate code from a description
let code = chat_code("Write a BioLang script to compute GC content per chromosome from a FASTA file")
println(code)
```

### Literate notebooks

```bash
# Run a .bln notebook (Markdown + BioLang code cells)
bl notebook analysis.bln

# Open the notebook editor with a native, contextual local kernel
bl notebook serve analysis.bln

# Export to HTML report
bl notebook analysis.bln --export html

# Export editable cells backed by BioLang WebAssembly
bl notebook analysis.bln --export html-wasm > analysis-live.html

# Convert to/from Jupyter
bl notebook analysis.bln --to-ipynb > analysis.ipynb
bl notebook imported.ipynb --from-ipynb > imported.bln
```

The live HTML export uses dependency-free editors and one shared browser session.
It preserves SVG plots and prepares a selectable canvas fallback. Native libraries,
GPU work, unrestricted file access, and very large analyses can use
`bl notebook serve`: it exposes a loopback-only, launch-token-protected local
kernel with persistent cell context and a SOMER-compatible job API subset.
Selecting a later cell automatically runs the missing earlier cells, avoiding
undefined setup variables in tutorial notebooks.

Sample `.bln` notebook:

````markdown
# QC Report

This notebook analyzes FASTQ quality metrics.

```bl
let reads = read_fastq("sample.fq.gz") |> collect()
let total = len(reads)
let q30 = reads |> filter(|r| mean_phred(r.quality) >= 30) |> len()
println(f"Total: {total}, Q30: {q30}, Rate: {round(q30 / total * 100, 1)}%")
```

## GC Distribution

```bl {plot}
reads |> map(|r| gc_content(r.seq)) |> histogram("GC Content")
```
````

## Benchmarks

Benchmarked against Python (BioPython) and R (Bioconductor) on 32 bioinformatics tasks using real-world data (NCBI, UniProt, ClinVar, ENCODE). Correctness validated on both synthetic and real biological data (E. coli K-12, S. cerevisiae, ClinVar) with 9-task three-way comparison.

| Task | BioLang | Python | Speedup |
|---|---|---|---|
| FASTA Parse (30 KB) | 0.002s | 0.153s | **76.5x** |
| ENCODE Peak Overlap | 0.154s | 2.614s | **17.0x** |
| E. coli Genome Stats | 0.010s | 0.164s | **16.4x** |
| Protein K-mers | 0.011s | 0.154s | **14.0x** |
| GC Content (51 MB) | 0.125s | 0.721s | **5.8x** |
| K-mer Counting (21-mers) | 6.119s | 19.908s | **3.3x** |

Linux (WSL2), Intel i9-12900K. Python wins on VCF/CSV text parsing (optimized C extensions). BioLang scripts average **50-70% fewer lines** of code.

Read from `benchmarks/results/latest/linux/scores.yaml`, which the suite writes
and which the file itself names as the single source for this table. The
figures published here had been stale since before v1.1.0 and understated the
measured result by two to three times.

See the [full benchmark results](https://lang.bio/benchmarks.html) for all 32 tasks across Linux and Windows, methodology, and correctness validation. Raw data and reproducible scripts in [`benchmarks/`](benchmarks/).

## Releases

Pre-built binaries are published on every tagged release for 5 platforms:

| Platform | Archive |
|---|---|
| Linux x86_64 | `biolang-linux-x86_64.tar.gz` |
| Linux ARM64 | `biolang-linux-aarch64.tar.gz` |
| macOS x86_64 | `biolang-macos-x86_64.tar.gz` |
| macOS ARM (Apple Silicon) | `biolang-macos-aarch64.tar.gz` |
| Windows x86_64 | `biolang-windows-x86_64.zip` |

Download from [Releases](https://github.com/oriclabs/biolang/releases).

Each archive contains:
- **`bl`** -- main CLI: run scripts, interactive REPL, manage plugins
- **`bl-lsp`** -- language server for editor integration (VS Code, Neovim, etc.)
- **`bl-convert`** -- optional format converter and external-tool runner, which
  is also what `bl convert ...` delegates to
- **`packages/`** -- the bundled BioLang packages, so `import "statistics"`
  resolves without a checkout

`checksums.sha256` is published alongside the archives.

Releases are built automatically via GitHub Actions when a version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
# CI builds all 4 platform binaries and creates a GitHub Release
```

### Verify downloads

```bash
sha256sum -c checksums.sha256
```

## Crate Structure

```
crates/
  bio-core/    -- Shared bio types (DNA, RNA, Protein, Variant, Gene, etc.)
  bl-core/     -- AST, Value, Table, Type, Span, Error
  bl-lexer/    -- Tokenizer
  bl-parser/   -- Recursive descent + Pratt expression parser
  bl-runtime/  -- Tree-walking interpreter, 1000+ builtins
  bl-bio/      -- FASTA/FASTQ/BED/GFF/VCF I/O
  bl-apis/     -- Bio API clients (NCBI, Ensembl, UniProt, etc.)
  bl-compiler/ -- Bytecode compiler (experimental)
  bl-jit/      -- JIT via Cranelift (feature-gated)
  bl-repl/     -- Interactive REPL
  bl-lsp/      -- Language Server Protocol
  bl-cli/      -- CLI binary (bl)
  bl-convert/  -- Optional safe format-conversion CLI (bl-convert)
```

## Documentation

- [Website](https://lang.bio) -- getting started, language reference, builtin docs
- [BL Convert guide](https://lang.bio/docs/tools/bl-convert.html) -- safe file conversion and optional external-tool backends
- [Playground](https://lang.bio/playground.html) -- try BioLang in your browser (no install required)
- [Language reference](https://lang.bio/books/language/html/) -- comprehensive guide with examples
- [Workflows](https://github.com/oriclabs/biolang-workflows) -- runnable analyses, practical courses, notebooks, benchmarks, and independent validation

## License

MIT -- see [LICENSE](LICENSE).

## Contributing

BioLang is developed by [ORIC Labs](https://github.com/oriclabs). Issues and pull requests welcome.
