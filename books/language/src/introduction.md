# Introduction

> **Download PDF**: [The BioLang Programming Language (PDF)](the-biolang-book.pdf)

BioLang is a domain-specific language built for bioinformatics. It is not a general-purpose language that happens to have biology libraries — every feature, from its type system to its operators, exists because bioinformatics workflows demand it.

## Why a DSL?

Bioinformaticians spend most of their time gluing tools together: reading FASTA files, filtering variants, piping data through transformations, querying biological databases, and building reproducible pipelines. General-purpose languages require dozens of imports, boilerplate, and framework knowledge before you can write your first useful script.

BioLang eliminates that friction. A complete variant filtering workflow:

```
read_vcf("variants.vcf")
    |> filter(|v| v.qual > 30)
    |> filter(|v| is_snp(v))
    |> filter(|v| is_transition(v))
    |> map(|v| {chrom: v.chrom, pos: v.pos, ref: v.ref_allele, alt: v.alt_allele, qual: v.qual})
    |> write_tsv("filtered_transitions.csv")
```

No imports. No boilerplate. No framework. DNA, RNA, proteins, intervals, variants, and quality scores are first-class types — not strings pretending to be biology.

## Design Principles

**Pipe-first.** The `|>` operator is the backbone of BioLang. Bioinformatics is inherently a series of transformations: raw reads become aligned reads become variant calls become annotated reports. Pipes make this natural:

```
samples
    |> par_map(|s| read_fastq(s.path))
    |> map(|reads| filter(reads, |r| mean(r.quality) > 20))
    |> map(|reads| gc_content(reads))
    |> summarize(|key, vals| {mean_gc: mean(vals)})
```

**Biology is built in.** DNA, RNA, protein, and quality score literals are part of the syntax. Genomic intervals and variants have dedicated types with domain-specific methods. You do not need to install a package to reverse-complement a sequence:

```
let seq = dna"ATCGATCGATCG"
let rc = reverse_complement(seq)    # dna"CGATCGATCGAT"
let rna = transcribe(seq)           # rna"AUCGAUCGAUCG"
let protein = translate(rna)        # protein"IDRS"
```

**Tables are native.** Bioinformatics lives in tables — sample sheets, count matrices, variant lists, BED files. BioLang tables support column operations, grouping, summarization, and joins without any library:

```
let counts = csv("gene_counts.csv")
counts
    |> mutate("log_count", |row| log2(row.count + 1))
    |> group_by("sample")
    |> summarize(|sample, rows| {mean_expr: mean(col(rows, "log_count")), n_genes: len(rows)})
```

**Fifteen bio databases at your fingertips.** NCBI, Ensembl, UniProt, UCSC, KEGG, STRING, PDB, Reactome, Gene Ontology, COSMIC, BioMart, NCBI Datasets, nf-core, BioContainers, and Galaxy ToolShed are built-in — no API keys required for most (NCBI key optional for higher rate limits, COSMIC key required):

```
# requires: internet connection
# requires: NCBI_API_KEY (optional, increases rate limit)
let gene = ncbi_gene("TP53")
let pathways = reactome_pathways("TP53")
let interactions = string_network(["TP53"], 9606)
```

**Pipelines are natural.** Data flows through pipes just like a bioinformatics workflow. No separate workflow engine needed — compose operations with `|>` and use `group_by`, `count_by`, and `filter_by` for efficient data processing:

```
# Complete QC pipeline in 5 lines
let reads = read_fastq("sample.fq") |> collect()
let passed = reads |> filter(|r| mean_phred(r.quality) >= 25)
let gc_vals = passed |> map(|r| gc_content(r.seq))
println(f"Passed: {len(passed)}/{len(reads)}")
println(f"Mean GC: {mean(gc_vals)}")
```

**Fast without trying.** BioLang is implemented in Rust with native I/O. On benchmarks against Python (BioPython) and R (Bioconductor) across 30 tasks, BioLang is up to **7.1x** faster on ENCODE peak overlap, **7.0x** on protein k-mers, **6.7x** on FASTA parsing, **3.2x** on k-mer counting — while using 50–70% fewer lines of code. Python wins on text-heavy VCF/CSV parsing. See [Benchmarks & Correctness](./ch19-benchmarks.md) for full results, methodology, and three-way correctness validation.

**Streams handle scale.** File readers return lazy streams. Process a 100 GB FASTQ without loading it into memory. Parallel maps distribute work across cores:

```
read_fastq("massive_file.fq")
    |> filter(|r| mean(r.quality) > 25)
    |> par_map(|r| gc_content(r.seq))
    |> summarize(|key, vals| {mean_gc: mean(vals), n_reads: len(vals)})
```

## Who This Book Is For

This book is for bioinformaticians, computational biologists, and genomics researchers who want to write analysis scripts faster and more clearly. You do not need deep programming experience — if you have used R, Python, or shell scripts for bioinformatics, you will feel at home.

Every example in this book uses real biological data and real analysis tasks. There are no toy examples. By the end, you will be able to:

- Process sequencing data (FASTA, FASTQ, BAM, VCF, BED, GFF)
- Query biological databases directly from your scripts
- Build reproducible analysis pipelines
- Perform statistical analysis on expression and variant data
- Scale to large datasets using streams and parallel operations
- Extend BioLang with plugins in Python, R, or TypeScript

## Running the Examples

Install BioLang:

```bash
cargo install biolang
```

Or build from source:

```bash
git clone https://github.com/oriclabs/biolang
cd biolang
cargo build --release
```

Run any example from this book:

```bash
bl run example.bl
```

Or use the interactive REPL:

```bash
bl repl
```

Every code block in this book is valid BioLang. Copy, paste, run.

### Sample Data

Many examples in this book reference files like `contigs.fa`, `reads.fq`, `calls.vcf`,
and `gene_counts.csv`. BioLang ships with sample data for all of these in the
`examples/sample-data/` directory:

```
examples/sample-data/
  contigs.fa          4 assembled contigs (FASTA)
  reads.fq            8 sequencing reads with quality scores (FASTQ)
  calls.vcf           12 variants across 5 chromosomes (VCF)
  promoters.bed       10 promoter regions (BED)
  chip_peaks.bed      7 ChIP-seq peaks (BED)
  annotations.gff     16 gene features for 6 genes (GFF3)
  samples.csv         6-sample sheet with conditions and batches (CSV)
  gene_counts.csv     15 cancer genes × 6 samples expression matrix (CSV)
  counts.tsv          8 genomic region read counts (TSV)
  data.sam            6 SAM alignment records with headers (SAM)
```

To run the quickstart script that exercises all sample data:

```bash
bl run examples/quickstart.bl
```

When a book example references a bare filename like `read_fasta("contigs.fa")`,
you can substitute the sample data path:

```
let sequences = read_fasta("examples/sample-data/contigs.fa")
```
