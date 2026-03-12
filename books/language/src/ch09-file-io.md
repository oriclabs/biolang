# File I/O

Bioinformatics begins and ends with files. Sequencers emit FASTQ, aligners
produce BAM, callers output VCF, annotators read GFF. BioLang provides
first-class readers and writers for every major format, with consistent
interfaces and stream support for files that exceed available memory.

> **Sample data:** BioLang ships with sample files in `examples/sample-data/`
> covering FASTA, FASTQ, VCF, BED, GFF, CSV, TSV, and SAM formats. You can
> substitute these paths wherever the examples below use bare filenames.
> Run `bl run examples/quickstart.bl` to verify all files are present.

## Reading FASTA

`read_fasta()` returns a list of records, each with `header` and `seq` fields.

```biolang
let contigs = read_fasta("assembly.fasta")

for c in contigs {
    print(c.header + ": " + str(len(c.seq)) + " bp")
}

# Find the longest contig
contigs |> sort(|a, b| len(b.seq) - len(a.seq)) |> first() |> into longest
print("Longest: " + longest.header + " (" + str(len(longest.seq)) + " bp)")
```

FASTA records carry the raw sequence as a string. Use bio literals for
compile-time validated sequences, and `read_fasta` for runtime file data.

## Reading FASTQ

`read_fastq()` returns records with `id`, `seq`, `quality`, and `comment`
fields. Quality scores are integer Phred values.

```biolang
let reads = read_fastq("sample_R1.fastq.gz")

reads |> map(|r| gc_content(r.seq)) |> mean() |> into gc_mean
let summary = {
    total: len(reads),
    mean_length: reads |> map(|r| len(r.seq)) |> mean(),
    mean_qual: reads |> map(|r| mean(r.quality)) |> mean(),
    gc_pct: gc_mean * 100
}

print("Reads: " + str(summary.total))
print("Mean length: " + str(round(summary.mean_length, 1)))
print("Mean quality: " + str(round(summary.mean_qual, 1)))
print("GC%: " + str(round(summary.gc_pct, 1)))
```

Compressed files (`.gz`) are decompressed transparently.

## Reading VCF

`read_vcf()` returns variant records with fields `chrom`, `pos`, `id`, `ref`,
`alt`, `qual`, `filter`, `info_str`, and `genotypes`.

```biolang
let variants = read_vcf("gatk_output.vcf")

let passing = variants |> filter(|v| v.filter == "PASS")
let snps = passing |> filter(|v| is_snp(v))
let indels = passing |> filter(|v| is_indel(v))

print("PASS variants: " + str(len(passing)))
print("  SNPs: " + str(len(snps)))
print("  Indels: " + str(len(indels)))
```

## Reading BED

`read_bed()` returns interval records with `chrom`, `start`, `end`, and
optional `name`, `score`, `strand` fields depending on the number of columns.

```biolang
let targets = read_bed("exome_targets.bed")

let total_bp = targets |> map(|t| t.end - t.start) |> reduce(0, |a, b| a + b)
print("Total target region: " + str(total_bp) + " bp")
print("Number of targets: " + str(len(targets)))

# Group by chromosome
let by_chrom = targets |> group_by("chrom")
for (chrom, regions) in by_chrom {
    let bp = regions |> map(|r| r.end - r.start) |> reduce(0, |a, b| a + b)
    print(chrom + ": " + str(len(regions)) + " targets, " + str(bp) + " bp")
}
```

## Reading GFF/GTF

`read_gff()` parses GFF3 and GTF formats. Records have `chrom`, `source`,
`type`, `start`, `end`, `score`, `strand`, `phase`, and `attributes` (a map).

```biolang
let features = read_gff("gencode.v44.annotation.gff3")

let genes = features |> filter(|f| f.type == "gene")
let protein_coding = genes |> filter(|g| g.attributes.gene_type == "protein_coding")

print("Total genes: " + str(len(genes)))
print("Protein-coding: " + str(len(protein_coding)))

# Gene length distribution
let lengths = protein_coding |> map(|g| g.end - g.start)
print("Median gene length: " + str(median(lengths)) + " bp")
print("Mean gene length: " + str(round(mean(lengths), 0)) + " bp")
```

## Reading CSV and TSV

`csv()` and `tsv()` return tables where column headers become field names.

```biolang
let metadata = csv("sample_metadata.csv")

# Group samples by condition
let groups = metadata |> group_by("condition")

for (condition, samples) in groups {
    let ids = samples |> map(|s| s.sample_id) |> join(", ")
    print(condition + ": " + ids)
}

# DESeq2-style count matrix
let counts = tsv("gene_counts.tsv")
let gene_names = counts |> map(|row| row.gene_id)
print("Genes in count matrix: " + str(len(gene_names)))
```

## Writing Files

Writer functions take a list of records and an output path. The format is
determined by the function name.

```biolang
# Write FASTA
let filtered = contigs |> filter(|c| len(c.seq) >= 500)
write_fasta(filtered, "assembly_filtered.fasta")

# Write FASTQ
let trimmed = reads |> map(|r| trim_quality(r, 20, 0))
write_fastq(trimmed, "trimmed_R1.fastq.gz")

# Write CSV
let stats_table = samples |> map(|s| {
    sample: s.id,
    total_reads: s.read_count,
    pct_mapped: round(s.mapped / s.read_count * 100, 2),
    mean_depth: round(s.depth, 1)
})
write_tsv(stats_table, "qc_summary.csv")
```

The `.gz` extension triggers gzip compression automatically.

## Stream-Based Reading

For files too large to fit in memory -- a 300 GB whole-genome BAM, a
multi-sample VCF with millions of records -- use streaming. Stream readers
return a lazy sequence that is consumed once, record by record.

```biolang
# Stream a large FASTQ without loading it all
let stream = read_fastq("wgs_sample.fastq.gz")

# Streams work with all HOFs -- processing is lazy
let high_qual_count = stream
    |> filter(|r| mean(r.quality) >= 30)
    |> filter(|r| len(r.seq) >= 100)
    |> len()

print("High-quality long reads: " + str(high_qual_count))
```

Streams are consumed once. If you need multiple passes, collect into a list
or read the file again:

```biolang
# Two-pass: first pass counts, second pass filters
let total = read_fastq("reads.fastq.gz") |> len()
let passing = read_fastq("reads.fastq.gz")
    |> filter(|r| mean(r.quality) >= 20)
    |> len()

print("Pass rate: " + str(round(passing / total * 100, 2)) + "%")
```

## Example: Convert FASTQ to FASTA

Strip quality scores from a FASTQ file to produce a FASTA for downstream
assembly or BLAST.

```biolang
let reads = read_fastq("nanopore_reads.fastq.gz")

let fasta_records = reads |> map(|r| {
    header: r.id + " " + (r.description ?? ""),
    seq: r.seq
})

write_fasta(fasta_records, "nanopore_reads.fasta")
print("Converted " + str(len(fasta_records)) + " reads to FASTA")
```

For large files, this streams through without holding everything in memory
because `map` on a stream produces a stream.

## Example: Extract Coding Sequences from GFF + FASTA

Combine a genome FASTA with GFF annotation to extract CDS sequences for each
protein-coding gene.

```biolang
let genome = read_fasta("GRCh38.fasta")
let features = read_gff("gencode.v44.annotation.gff3")

# Build a lookup from chromosome name to sequence
let chrom_seqs = {}
for contig in genome {
    let name = split(contig.header, " ") |> first()
    chrom_seqs = {...chrom_seqs, [name]: contig.seq}
}

# Collect CDS exons grouped by transcript
let cds_features = features
    |> filter(|f| f.type == "CDS")
    |> group_by("transcript_id")

let cds_records = []

for (tx_id, exons) in cds_features {
    let sorted_exons = exons |> sort(|a, b| a.start - b.start)
    let chrom = sorted_exons[0].chrom
    let strand = sorted_exons[0].strand

    guard chrom_seqs[chrom] != nil else { continue }

    let seq = sorted_exons
        |> map(|e| slice(chrom_seqs[chrom], e.start - 1, e.end))
        |> join("")

    let final_seq = if strand == "-" { reverse_complement(seq) } else { seq }

    cds_records = cds_records + [{
        header: tx_id + " " + chrom + " " + strand,
        seq: final_seq
    }]
}

write_fasta(cds_records, "coding_sequences.fasta")
print("Extracted " + str(len(cds_records)) + " coding sequences")
```

## Example: Filter VCF by Quality and Write Passing Variants

Apply multiple quality filters to a VCF and write both passing and failing
records to separate files.

```biolang
let variants = read_vcf("raw_calls.vcf")

let results = variants |> map(|v| {
    let info = parse_info(v.info_str)
    let dp = info?.DP ?? 0
    let mq = info?.MQ ?? 0.0
    let af = info?.AF ?? 0.0

    let pass = v.qual >= 30.0
        && dp >= 10
        && mq >= 40.0
        && v.filter != "LowQual"

    {...v, qc_pass: pass, depth: dp, map_qual: mq, allele_freq: af}
})

let passing = results |> filter(|v| v.qc_pass)
let failing = results |> filter(|v| !v.qc_pass)

write_vcf(passing, "filtered_PASS.vcf")
write_vcf(failing, "filtered_FAIL.vcf")

print("Total: " + str(len(results)))
print("PASS: " + str(len(passing)))
print("FAIL: " + str(len(failing)))

# Summarise failure reasons
let low_qual = failing |> filter(|v| v.qual < 30.0) |> len()
let low_depth = failing |> filter(|v| v.depth < 10) |> len()
let low_mq = failing |> filter(|v| v.map_qual < 40.0) |> len()

print("Reasons (not mutually exclusive):")
print("  Low QUAL (<30): " + str(low_qual))
print("  Low depth (<10): " + str(low_depth))
print("  Low MQ (<40): " + str(low_mq))
```

## Example: Merge BED Files and Compute Coverage

Load target regions from multiple BED files, merge overlapping intervals, and
compute total coverage.

```biolang
let bed_files = [
    "targets_panel_v1.bed",
    "targets_panel_v2.bed",
    "custom_hotspots.bed"
]

# Read and concatenate all regions
let all_regions = bed_files
    |> flat_map(|path| {
        let regions = read_bed(path)
        print("Loaded " + str(len(regions)) + " regions from " + path)
        regions
    })

print("Total regions before merge: " + str(len(all_regions)))

# Sort by chromosome and start position
let sorted = all_regions
    |> sort(|a, b| {
        if a.chrom != b.chrom {
            compare(a.chrom, b.chrom)
        } else {
            a.start - b.start
        }
    })

# Merge overlapping intervals
fn merge_intervals(intervals) {
    guard len(intervals) > 0 else { return [] }

    let merged = [intervals[0]]

    for region in intervals |> drop(1) {
        let last_region = merged |> last()
        if region.chrom == last_region.chrom && region.start <= last_region.end {
            # Overlapping -- extend the current interval
            let new_end = max(last_region.end, region.end)
            merged = merged |> take(len(merged) - 1)
            merged = merged + [{...last_region, end: new_end}]
        } else {
            merged = merged + [region]
        }
    }
    merged
}

let merged = merge_intervals(sorted)
print("Regions after merge: " + str(len(merged)))

let total_bp = merged |> map(|r| r.end - r.start) |> reduce(0, |a, b| a + b)
print("Total coverage: " + str(total_bp) + " bp (" + str(round(total_bp / 1e6, 2)) + " Mb)")

# Per-chromosome summary
let by_chrom = merged |> group_by("chrom")
for (chrom, regions) in by_chrom |> sort(|a, b| compare(a.0, b.0)) {
    let bp = regions |> map(|r| r.end - r.start) |> reduce(0, |a, b| a + b)
    print("  " + chrom + ": " + str(len(regions)) + " regions, " + str(bp) + " bp")
}

# Write merged BED
write_bed(merged, "merged_targets.bed")
```

## Format Reference

| Function | Format | Returns |
|---|---|---|
| `read_fasta(path)` | FASTA | `[{header, seq}]` |
| `read_fastq(path)` | FASTQ | `[{id, seq, quality, comment}]` |
| `read_vcf(path)` | VCF | `[{chrom, pos, id, ref, alt, qual, filter, info_str, genotypes}]` |
| `read_bed(path)` | BED | `[{chrom, start, end, name?, score?, strand?}]` |
| `read_gff(path)` | GFF3/GTF | `[{chrom, source, type, start, end, score, strand, phase, attributes}]` |
| `csv(path)` | CSV | `[{col1: val, col2: val, ...}]` |
| `tsv(path)` | TSV | `[{col1: val, col2: val, ...}]` |
| `write_fasta(records, path)` | FASTA | writes file |
| `write_fastq(records, path)` | FASTQ | writes file |
| `write_vcf(records, path)` | VCF | writes file |
| `write_bed(records, path)` | BED | writes file |
| `write_tsv(records, path)` | CSV | writes file |

All readers accept `.gz` paths and decompress automatically. All writers
compress when the output path ends in `.gz`.
