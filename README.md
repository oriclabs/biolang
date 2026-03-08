# BioLang

> **Warning**: BioLang is experimental and under active development. The language syntax, builtins, and APIs may change without notice between releases. Not recommended for production use yet. Feedback and bug reports are welcome.

A pipe-first domain-specific language (DSL) for bioinformatics.

BioLang is a DSL purpose-built for genomics and molecular biology. It brings
first-class biological types, 400+ domain builtins, and composable pipelines
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
- **400+ builtins** -- FASTQ/FASTA/VCF/BED/GFF I/O, sequence ops, statistics, genomic intervals, tables
- **Pipelines** -- first-class `pipeline` blocks with stages, DAG execution, `parallel for`, `defer`
- **Parameterized pipelines** -- reusable templates: `pipeline align(sample, ref) { ... }`
- **Bio API clients** -- NCBI, Ensembl, UniProt, UCSC, KEGG, STRING, PDB, Reactome, GO, COSMIC
- **SQLite** -- built-in database for storing and querying results
- **Notifications** -- Slack, Teams, Telegram, Discord, email alerts from pipelines
- **Streams** -- lazy evaluation for large files without loading into memory
- **Tables** -- R-like data frames with filter, mutate, group_by, summarize, join
- **Plugin system** -- extend with Python, TypeScript, R, or native plugins
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
pipeline variant_qc {
  stage align {
    shell("bwa-mem2 mem -t 16 GRCh38.fa R1.fq.gz R2.fq.gz | samtools sort -o aligned.bam")
    "aligned.bam"
  }

  stage call {
    shell("gatk HaplotypeCaller -R GRCh38.fa -I " + align + " -O raw.vcf.gz")
    "raw.vcf.gz"
  }

  stage stats {
    let variants = read_vcf(call) |> collect()
    let snps = variants |> filter(|v| v.is_snp) |> len()
    let indels = variants |> filter(|v| v.is_indel) |> len()
    print("SNPs: " + str(snps) + ", Indels: " + str(indels))
  }
}
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
  |> sort_by(|r| r.mean_depth, desc: true)
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
  bl-runtime/  -- Tree-walking interpreter, 400+ builtins
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
- [Book](https://lang.bio/book/) -- comprehensive guide with examples

## License

MIT -- see [LICENSE](LICENSE).

## Contributing

BioLang is developed by [ORIC Labs](https://github.com/oriclabs). Issues and pull requests welcome.

Built with Claude (vibe coding). 🧬
