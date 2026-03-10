# BioLang

> **Warning**: BioLang is experimental and under active development. The language syntax, builtins, and APIs may change without notice between releases. Not recommended for production use yet. Feedback and bug reports are welcome.

A pipe-first domain-specific language (DSL) for bioinformatics.

BioLang is a DSL purpose-built for genomics and molecular biology. It brings
first-class biological types, 750+ domain builtins, 15 bio API clients, and composable pipelines
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

## Features

- **Bio-native types** -- DNA, RNA, Protein, Interval, Variant, Gene, AlignedRead, Quality
- **Pipe operator** -- `|>` inserts the left side as the first argument: `a |> f(b)` = `f(a, b)`
- **750+ builtins** -- FASTQ/FASTA/VCF/BED/GFF I/O, sequence ops, statistics, genomic intervals, tables, 42 plot types
- **Pipe-first pipelines** -- compose operations with `|>`, `group_by`, `count_by`, `filter_by` for efficient data processing
- **15 Bio API clients** -- NCBI, Ensembl, UniProt, UCSC, KEGG, STRING, PDB, Reactome, GO, COSMIC, BioMart, NCBI Datasets, nf-core, BioContainers, Galaxy ToolShed
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

## Install

### From source

```bash
git clone https://github.com/oriclabs/biolang.git
cd biolang
cargo install --path crates/bl-cli
```

### From releases

Download pre-built binaries from [Releases](https://github.com/oriclabs/biolang/releases).

```bash
# Linux
curl -L https://github.com/oriclabs/biolang/releases/latest/download/biolang-linux-x86_64.tar.gz | tar xz
sudo mv bl /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/oriclabs/biolang/releases/latest/download/biolang-macos-aarch64.tar.gz | tar xz
sudo mv bl /usr/local/bin/

# Windows (PowerShell)
Invoke-WebRequest -Uri https://github.com/oriclabs/biolang/releases/latest/download/biolang-windows-x86_64.zip -OutFile biolang.zip
Expand-Archive biolang.zip -DestinationPath .
```

## Quick Start

```bash
# Run a script
bl run analysis.bl

# Interactive REPL
bl repl

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
  |> mutate(pass_rate: |r| r.passing / r.total * 100)
  |> group_by(|r| r.cohort)
  |> summarize(mean_depth: |g| mean(g.depth))
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
let gene = ncbi_gene("BRCA1")
print(gene.description)

let variants = ensembl_vep("17:43044295:G:A")
print(variants)
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

# Export to HTML report
bl notebook analysis.bln --export html

# Convert to/from Jupyter
bl notebook analysis.bln --export ipynb
bl notebook imported.ipynb --export bln
```

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

BioLang vs Python (BioPython) vs R (Bioconductor) on 30 bioinformatics tasks -- synthetic and real-world data.

### Linux (WSL2) -- Intel i9-12900K, 16 GB RAM, BioLang 0.2.1, Python 3.12.3, R 4.3.3

| Task | BioLang | Python | R | BL vs Py |
|---|---|---|---|---|
| FASTA Small (30 KB) | 0.138s | 0.926s | 1.243s | **6.7x** |
| FASTA gzipped (1.3 MB) | 0.141s | 0.930s | 1.327s | **6.6x** |
| Protein K-mers | 0.191s | 1.331s | 1.298s | **7.0x** |
| ENCODE Peak Overlap | 0.363s | 2.574s | -- | **7.1x** |
| E. coli Genome | 0.176s | 1.081s | 1.354s | **6.1x** |
| GC Content (51 MB) | 0.830s | 2.771s | 2.358s | **3.3x** |
| K-mer Counting (21-mers) | 6.551s | 21.01s | -- | **3.2x** |
| Chr22 21-mer Count (51 MB) | 10.72s | 28.73s | -- | **2.7x** |
| FASTQ QC Pipeline | 2.349s | 5.059s | -- | **2.2x** |
| VCF Filtering | 0.349s | 0.166s | 6.312s | Py 2.1x |
| CSV Join + Group-by | 0.281s | 0.156s | 0.312s | Py 1.8x |

### Windows 11 -- Intel i9-12900K, 32 GB RAM, BioLang 0.2.1, Python 3.12.4

| Task | BioLang | Python | BL vs Py |
|---|---|---|---|
| K-mer Counting (21-mers) | 6.04s | 18.14s | **3.0x** |
| ENCODE Peak Overlap | 1.03s | 3.03s | **2.9x** |
| Chr22 21-mer Count (51 MB) | 9.08s | 24.24s | **2.7x** |
| FASTQ QC Pipeline | 2.03s | 5.06s | **2.5x** |
| FASTQ (26 MB) | 1.02s | 2.05s | **2.0x** |
| FASTQ QC | 2.02s | 3.04s | **1.5x** |

Windows process creation adds ~1s overhead per invocation, compressing ratios for fast benchmarks. See Linux results for accurate algorithmic comparison.

BioLang scripts average **50-70% fewer lines** than equivalent Python. K-mer counting uses native 2-bit DNA encoding with canonical (strand-agnostic) counting -- strictly more work than Python's forward-only approach.

See [`benchmarks/`](benchmarks/) for full suite with 30 benchmarks, platform-specific reports, methodology, and reproducible scripts.

## Releases

Pre-built binaries are published on every tagged release for 4 platforms:

| Platform | Archive | Binaries |
|---|---|---|
| Linux x86_64 | `biolang-linux-x86_64.tar.gz` | `bl`, `bl-lsp` |
| macOS x86_64 | `biolang-macos-x86_64.tar.gz` | `bl`, `bl-lsp` |
| macOS ARM (Apple Silicon) | `biolang-macos-aarch64.tar.gz` | `bl`, `bl-lsp` |
| Windows x86_64 | `biolang-windows-x86_64.zip` | `bl.exe`, `bl-lsp.exe` |

Download from [Releases](https://github.com/oriclabs/biolang/releases).

Each release includes:
- **`bl`** -- main CLI: run scripts, interactive REPL, manage plugins
- **`bl-lsp`** -- language server for editor integration (VS Code, Neovim, etc.)
- **`checksums.sha256`** -- SHA-256 checksums for all archives

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
  bl-runtime/  -- Tree-walking interpreter, 750+ builtins
  bl-bio/      -- FASTA/FASTQ/BED/GFF/VCF I/O
  bl-apis/     -- Bio API clients (NCBI, Ensembl, UniProt, etc.)
  bl-compiler/ -- Bytecode compiler (experimental)
  bl-jit/      -- JIT via Cranelift (feature-gated)
  bl-repl/     -- Interactive REPL
  bl-lsp/      -- Language Server Protocol
  bl-cli/      -- CLI binary (bl)
```

## Documentation

- [Website](https://lang.bio) -- getting started, language reference, builtin docs
- [Playground](https://lang.bio/playground.html) -- try BioLang in your browser (no install required)
- [Book](https://lang.bio/book/) -- comprehensive guide with examples

## License

MIT -- see [LICENSE](LICENSE).

## Contributing

BioLang is developed by [ORIC Labs](https://github.com/oriclabs). Issues and pull requests welcome.
