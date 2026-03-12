# Chapter 14: Streams and Scale

Genomics data is large. A single whole-genome sequencing run produces hundreds
of gigabytes of FASTQ. A population-scale VCF can hold billions of variant
records. Loading everything into memory is not an option. BioLang addresses
this with streams: lazy, single-pass iterators that process data record by
record without materializing the entire dataset.

## What Is a Stream?

A stream is a `StreamValue` -- a lazy sequence of items produced on demand.
File readers in BioLang return streams by default. Streams are consumed once:
after you iterate through a stream, it is exhausted.

```
# read_fastq returns a stream, not a list
let reads = read_fastq("whole_genome_R1.fastq.gz")

# This processes one record at a time -- constant memory
let high_quality = reads
  |> filter(|r| mean_phred(r.quality) >= 30)
  |> map(|r| {id: r.id, length: r.length, gc: gc_content(r.seq)})
  |> write_tsv("read_stats.tsv")
```

No matter how large the FASTQ file is, this script uses the same amount of
memory. Each record flows through `filter`, then `map`, then is written out
before the next record is read.

## Stream Operations

Streams support the same functional operations as lists, but they execute
lazily.

### map

Transform each element.

```
read_vcf("variants.vcf.gz")
  |> map(|v| {chrom: v.chrom, pos: v.pos, qual: v.qual, af: v.info.AF})
  |> write_tsv("variant_summary.tsv")
```

### filter

Keep only elements matching a predicate.

```
read_bam("aligned.bam")
  |> filter(|r| r.mapping_quality >= 30 and not r.is_duplicate)
  |> count()
  |> |n| print(str(n) + " high-quality unique alignments")
```

### take_while

Consume from a stream while a condition holds, then stop.

```
# Read the first million reads from a FASTQ for quick QC
let sample = read_fastq("deep_sequencing.fastq.gz")
  |> take_while(|r, i| i < 1000000)

let quals = sample |> map(|r| mean_phred(r.quality))
print("Sampled stats: " + str(mean(quals)) + " mean Q")
```

### each

Execute a side effect for every element (the stream analogue of a for loop).

```
let bad_count = 0
read_fastq("sample.fastq.gz")
  |> each(|r| {
       if mean_phred(r.quality) < 20 then
         bad_count = bad_count + 1
     })
print(str(bad_count) + " low-quality reads")
```

### fold / reduce

Accumulate a value across the entire stream.

```
let total_bases = read_fastq("sample.fastq.gz")
  |> map(|r| r.length)
  |> sum()
print("Total bases: " + str(total_bases))
```

## Chunked Processing

When you need batch operations on a stream, `chunk` groups elements into
fixed-size lists. This is useful for writing output in blocks or batching API
calls.

```
# Process a FASTQ in chunks of 10,000 reads
read_fastq("large_sample.fastq.gz")
  |> chunk(10000)
  |> each(|batch| {
       let mean_q = batch |> map(|r| mean_phred(r.quality)) |> mean()
       print("Batch: " + str(len(batch)) + " reads, "
             + "mean Q=" + str(mean_q))
     })
```

Chunking is also useful for writing to multiple output files.

```
# Split a FASTQ into files of 1 million reads each
let file_num = 0
read_fastq("huge_sample.fastq.gz")
  |> chunk(1000000)
  |> each(|batch| {
       let path = "split/chunk_" + str(file_num) + ".fastq.gz"
       batch |> write_fastq(path)
       file_num = file_num + 1
     })
print("Wrote " + str(file_num) + " chunk files")
```

## Window Operations

`window` creates a sliding window over a list (or materialized portion of a
stream). This is essential for computing positional statistics across a genome.

```
# Sliding window GC content
read_fasta("chr1.fa") |> first() |> |r| r.seq |> into sequence
let win_size = 1000
let step = 500

let gc_track = window(sequence, win_size)
  |> enumerate()
  |> map(|pair| {
       pos: pair[0] * step,
       gc: gc_content(pair[1]),
     })

gc_track |> write_tsv("chr1_gc_content.tsv")
```

For windowed operations on streams (where you cannot look back), use
`window` which maintains a buffer.

```
# Compute rolling average base quality across a BAM
read_bam("sample.bam")
  |> filter(|r| r.chrom == "chr17")
  |> sort_by(|r| r.pos)
  |> window(100)
  |> map(|win| {
       pos: win[50].pos,
       mean_mapq: mean(win |> map(|r| r.mapping_quality)),
     })
  |> write_tsv("chr17_mapq_rolling.tsv")
```

## Parallel Operations

### par_map

Apply a function to each element of a list in parallel, distributing work
across available CPU cores.

```
let chromosomes = ["chr1", "chr2", "chr3", "chr4", "chr5", "chr6",
                   "chr7", "chr8", "chr9", "chr10", "chr11", "chr12",
                   "chr13", "chr14", "chr15", "chr16", "chr17", "chr18",
                   "chr19", "chr20", "chr21", "chr22", "chrX", "chrY"]

let per_chrom_stats = par_map(chromosomes, |chrom| {
  let reads = read_bam("sample.bam") |> filter(|r| r.chrom == chrom)
  let count = reads |> count()
  let depth_table = tool("samtools", "depth -r " + chrom + " sample.bam")
  let depth = depth_table["depth"] |> mean()
  {chrom: chrom, reads: count, mean_depth: depth}
})

per_chrom_stats |> sort_by(|s| s.chrom) |> write_tsv("chrom_stats.tsv")
```

### par_filter

Filter elements in parallel. Useful when the predicate itself is expensive.

```
# requires: internet connection
let variants = tsv("all_variants.tsv")

let pathogenic = par_filter(variants, |v| {
  let vep = ensembl_vep(v.chrom + ":" + str(v.pos) + ":"
                        + v.ref + ":" + v.alt)
  let worst = vep.consequences |> first()
  worst.impact == "HIGH"
})

print(str(len(pathogenic)) + " high-impact variants")
```

## Unit Helpers

BioLang provides genomic unit helpers that make size comparisons readable.

```
let region_size = mb(3)          # 3,000,000
let read_length = bp(150)        # 150
let window = kb(50)              # 50,000
let genome_size = gb(3)          # 3,000,000,000

# Use in calculations
let coverage = 1800000000 / genome_size  # ~0.6x
print("Coverage: " + str(coverage) + "x")

# Filter regions by size
let large_svs = read_vcf("structural_variants.vcf.gz")
  |> filter(|v| v.info.SVLEN != nil and abs(v.info.SVLEN) > mb(1))

print(str(large_svs |> count()) + " SVs larger than 1 Mb")
```

These helpers are simple multipliers but they prevent off-by-three-zeros bugs
that plague genomics scripts.

## Example: Process a 100GB FASTQ

Stream 4 billion reads, filter by quality, and compute statistics without
loading the file into memory.

```
let input = "novaseq_R1.fastq.gz"  # 100 GB compressed

let stats = {
  total: 0,
  passed: 0,
  total_bases: 0,
  gc_bases: 0,
  qual_sum: 0,
}

read_fastq(input)
  |> each(|r| {
       stats.total = stats.total + 1
       let q = mean_phred(r.quality)
       if q >= 30 then {
         stats.passed = stats.passed + 1
         stats.total_bases = stats.total_bases + r.length
         let gc = gc_content(dna(r.seq)) * r.length
         stats.gc_bases = stats.gc_bases + gc
         stats.qual_sum = stats.qual_sum + q
       }
     })

let pct_pass = stats.passed / stats.total * 100
let gc = stats.gc_bases / stats.total_bases * 100
let mean_q = stats.qual_sum / stats.passed

print("Total reads:  " + str(stats.total))
print("Passed Q>=30: " + str(stats.passed) + " (" + str(pct_pass) + "%)")
print("Mean quality: " + str(mean_q))
print("GC content:   " + str(gc) + "%")
```

This script processes the entire file in a single pass. Memory usage stays
constant regardless of file size.

## Example: Parallel Variant Annotation Across Chromosomes

Split variant annotation work by chromosome to use all cores.

```
# requires: internet connection
let vcf_path = "cohort.vcf.gz"
let chromosomes = range(1, 23) |> map(|n| "chr" + str(n))
let chromosomes = concat(chromosomes, ["chrX", "chrY"])

let annotated = par_map(chromosomes, |chrom| {
  let variants = read_vcf(vcf_path)
    |> filter(|v| v.chrom == chrom)
    |> collect()

  let results = variants |> map(|v| {
    let vep = ensembl_vep(v.chrom + ":" + str(v.pos) + ":" + v.ref + ":" + v.alt)
    let worst = vep.consequences
      |> sort_by(|c| c.impact_rank)
      |> first()
    {
      ...v,
      consequence: worst.consequence,
      impact: worst.impact,
      gene: worst.gene_symbol,
    }
  })

  print(chrom + ": " + str(len(results)) + " variants annotated")
  results
})
  |> flatten()

annotated |> write_tsv("annotated_variants.tsv")
print("Annotated " + str(len(annotated)) + " variants across "
      + str(len(chromosomes)) + " chromosomes")
```

## Example: Sliding Window GC Content

Compute GC content in 1 kb windows across an entire chromosome.

```
let chr_seq = read_fasta("GRCh38_chr22.fa") |> first() |> |r| r.seq
let chr_len = len(chr_seq)
let win = kb(1)
let step = bp(500)

print("Chromosome 22: " + str(chr_len) + " bp")
print("Computing GC in " + str(win) + " bp windows, step " + str(step))

let gc_profile = window(chr_seq, win)
  |> enumerate()
  |> map(|pair| {
       start: pair[0] * step,
       end: pair[0] * step + win,
       gc: gc_content(pair[1]),
     })

# Identify GC-rich and GC-poor regions
let gc_rich = gc_profile |> filter(|w| w.gc > 0.60)
let gc_poor = gc_profile |> filter(|w| w.gc < 0.35)

print("GC-rich windows (>60%): " + str(len(gc_rich)))
print("GC-poor windows (<35%): " + str(len(gc_poor)))

gc_profile |> write_tsv("chr22_gc_profile.tsv")
```

## Example: Batch Processing Thousands of Samples

Process a large sample cohort using chunked parallelism to avoid overwhelming
system resources.

```
let manifest = tsv("sample_manifest.tsv")  # 2,000 samples
print("Processing " + str(len(manifest)) + " samples")

let batch_size = 16  # process 16 samples at a time

let all_results = manifest
  |> chunk(batch_size)
  |> enumerate()
  |> map(|pair| {
       let batch_idx = pair[0]
       let batch = pair[1]
       print("Batch " + str(batch_idx + 1) + "/"
             + str(ceil(len(manifest) / batch_size)))

       par_map(batch, |sample| {
         # Align
         let sam = tool("bwa-mem2", "mem -t 2 GRCh38.fa " + sample.r1 + " " + sample.r2)
         let bam = tool("samtools", "sort -@ 2 -o " + sample.sample_id + ".sorted.bam " + sam)
         tool("samtools", "index " + bam)

         # QC metrics
         let flagstat = tool("samtools", "flagstat " + bam)
         let depth_vals = tool("samtools", "depth " + bam)
         let mean_depth = depth_vals["depth"] |> mean()

         {
           id: sample.sample_id,
           bam: bam,
           mean_depth: mean_depth,
           status: if mean_depth > 20
                   then "PASS" else "FAIL",
         }
       })
     })
  |> flatten()

let passed = all_results |> filter(|r| r.status == "PASS")
let failed = all_results |> filter(|r| r.status == "FAIL")

print("Results: " + str(len(passed)) + " PASS, "
      + str(len(failed)) + " FAIL")

if len(failed) > 0 then {
  print("Failed samples:")
  failed |> each(|r| print("  " + r.id + " depth=" + str(r.mean_depth)
                           + " mapped=" + str(r.mapped_pct) + "%"))
}

all_results |> write_tsv("cohort_qc_results.tsv")
```

## Memory Patterns to Know

| Pattern | Memory | Use When |
|---|---|---|
| `read_fastq(f) \|> filter(...) \|> count()` | O(1) | Counting, simple stats |
| `read_vcf(f) \|> collect()` | O(n) | Need random access |
| `read_bam(f) \|> chunk(k) \|> each(...)` | O(k) | Batch writing, API calls |
| `par_map(list, f)` | O(n) | List already in memory |
| `stream \|> fold(init, f)` | O(1) | Aggregation |

The rule of thumb: stay in stream mode as long as possible. Call `collect()`
only when you genuinely need the full dataset in memory (for sorting, random
access, or passing to a function that requires a list).

## Summary

Streams let BioLang handle files of any size in constant memory. Combine them
with `chunk` for batch processing, `window` for positional analysis, and
`par_map`/`par_filter` for multi-core parallelism. The unit helpers `bp()`,
`kb()`, `mb()`, and `gb()` keep genomic arithmetic readable. Together, these
tools let you write scripts that scale from a test FASTQ to a
population-scale dataset without changing the code.
