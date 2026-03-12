# Chapter 5: Pipes and Transforms

The pipe operator is the defining feature of BioLang. Every bioinformatics workflow
is a chain of transformations: read data, filter, transform, summarize, output. Pipes
make these chains explicit, readable, and composable.

## The Pipe Operator `|>`

The pipe `a |> f(b)` desugars to `f(a, b)`. The left-hand value becomes the first
argument of the right-hand function:

```
# These are equivalent:
gc_content(dna"ATCGATCGATCG")
dna"ATCGATCGATCG" |> gc_content()

# And these:
filter(reads, |r| mean(r.quality) >= 30)
reads |> filter(|r| mean(r.quality) >= 30)
```

### Why Pipe-First Matters for Bioinformatics

Without pipes, nested function calls read inside-out:

```
# Hard to follow -- you read from the innermost call outward
write_tsv(
  sort(
    summarize(
      group_by(
        filter(read_vcf("calls.vcf.gz"), |v| v.qual >= 30),
        "chrom"
      ),
      |chrom, rows| {n: len(rows), mean_qual: mean(col(rows, "qual"))}
    ),
    "n",
    descending: true
  ),
  "chrom_summary.csv"
)
```

With pipes, the same workflow reads top-to-bottom, matching how you think about
the analysis:

```
read_vcf("calls.vcf.gz")
  |> filter(|v| v.qual >= 30)
  |> group_by("chrom")
  |> summarize(|chrom, rows| {n: len(rows), mean_qual: mean(col(rows, "qual"))})
  |> sort("n", descending: true)
  |> write_tsv("chrom_summary.csv")
```

Each line is one transformation step. You can read the pipeline like a protocol.

## Chaining Transforms

The real power emerges when you chain multiple operations. Here is a complete
read-to-report workflow:

```
# Read a BAM file, compute per-chromosome alignment statistics
read_bam("sample.sorted.bam")
  |> filter(|r| r.mapq >= 30 && !r.is_duplicate && !r.is_unmapped)
  |> group_by("chrom")
  |> summarize(|chrom, rows| {
    mapped_reads: len(rows),
    mean_mapq: mean(col(rows, "mapq")),
    mean_insert: mean(col(rows, "insert_size"))
  })
  |> sort("mapped_reads", descending: true)
  |> mutate("pct", |row| row.mapped_reads / sum(col(rows, "mapped_reads")) * 100.0)
  |> write_tsv("alignment_stats.csv")
```

## The Tap Pipe `|>>`

Sometimes you need a side effect in the middle of a chain -- logging, writing
an intermediate file, printing a progress message -- without disrupting the data
flow. The tap pipe `|>>` evaluates the right side for its effect but passes the
left side through unchanged:

```
read_fastq("sample_R1.fastq.gz")
  |>> |reads| print(f"Raw reads: {len(reads)}")
  |> filter(|r| mean(r.quality) >= 25)
  |>> |reads| print(f"After quality filter: {len(reads)}")
  |> filter(|r| seq_len(r.seq) >= 50)
  |>> |reads| print(f"After length filter: {len(reads)}")
  |> write_fastq("filtered_R1.fastq.gz")
```

Output:

```
Raw reads: 2847291
After quality filter: 2541083
After length filter: 2539847
```

The tap pipe is invaluable for debugging pipelines:

```
read_vcf("raw_calls.vcf.gz")
  |>> |vs| print(f"Step 0 - Raw: {len(vs)} variants")
  |> filter(|v| v.filter == "PASS")
  |>> |vs| print(f"Step 1 - PASS only: {len(vs)}")
  |> filter(|v| v.qual >= 30)
  |>> |vs| print(f"Step 2 - Qual >= 30: {len(vs)}")
  |> filter(|v| into(v.info?.DP ?? "0", "Int") >= 10)
  |>> |vs| print(f"Step 3 - Depth >= 10: {len(vs)}")
  |> write_vcf("filtered_calls.vcf.gz")
```

You can also use tap to write intermediate checkpoints:

```
read_fasta("assembly.fa")
  |> filter(|s| seq_len(s.seq) >= 1000)
  |>> |seqs| write_fasta(seqs, "contigs_gt1kb.fa")
  |> filter(|s| gc_content(s.seq) >= 0.3 && gc_content(s.seq) <= 0.7)
  |>> |seqs| write_fasta(seqs, "contigs_normal_gc.fa")
  |> sort(|s| seq_len(s.seq), descending: true)
  |> take(100)
  |> write_fasta("top100_contigs.fa")
```

## Higher-Order Functions in Pipe Chains

### `map` -- Transform Each Element

```
# Extract GC content per contig
let gc_profile = read_fasta("contigs.fa")
  |> map(|seq| {id: seq.id, gc: gc_content(seq.seq), length: seq_len(seq.seq)})
```

### `filter` -- Select Elements by Predicate

```
# Keep only high-quality mapped reads
let good_reads = read_bam("aligned.bam")
  |> filter(|r| r.mapq >= 30)
  |> filter(|r| !r.is_duplicate)
  |> filter(|r| r.is_proper_pair)
```

### `reduce` -- Accumulate a Single Value

```
# Total bases across all chromosomes
let total_bases = read_fasta("reference.fa")
  |> map(|s| seq_len(s.seq))
  |> reduce(0, |acc, n| acc + n)

print(f"Reference genome size: {total_bases / 1e9:.2f} Gb")
```

### `sort` -- Order Elements

```
# Rank genes by expression level
let top_genes = csv("tpm.csv")
  |> sort("tpm", descending: true)
  |> take(50)
  |> map(|row| row.gene_name)
```

### `flat_map` -- Map and Flatten

Essential when each input produces multiple outputs:

```
# Extract all exons from a gene annotation
let all_exons = read_gff("genes.gff3")
  |> filter(|f| f.type == "gene")
  |> flat_map(|gene| gene.children |> filter(|c| c.type == "exon"))

# Count k-mers across multiple sequences (auto-spills to disk for large data)
let top_kmers = fasta("contigs.fa")
  |> kmer_count(21)        # Table sorted by count descending
  |> head(20)

# For bounded memory, use the top-N parameter:
# fasta("contigs.fa") |> kmer_count(21, 100)  # only top 100
```

### `take_while` -- Stream Until Condition Fails

```
# Read sorted variants until we pass a genomic position
let nearby_variants = read_vcf("sorted.vcf.gz")
  |> filter(|v| v.chrom == "chr17")
  |> take_while(|v| v.pos <= 7700000)
  |> filter(|v| v.pos >= 7660000)
```

### `scan` -- Running Accumulation

Like `reduce`, but emits every intermediate value:

```
# Cumulative read count across sorted BAM regions
let cumulative = read_bam("sorted.bam")
  |> group_by("chrom")
  |> map(|g| {chrom: g.key, count: len(g.values)})
  |> sort("chrom")
  |> scan(0, |acc, row| acc + row.count)
```

### `window` and `chunk` -- Sliding and Fixed Blocks

```
# Sliding window GC content
let gc_track = dna"ATCGATCGATCGATCGCCCCGGGG"
  |> window(size: 10, step: 5)
  |> map(|w| gc_content(w))

# Chunk reads into batches for parallel processing
let batches = read_fastq("sample.fastq.gz")
  |> chunk(100_000)
  |> map(|batch| {n_reads: len(batch), mean_gc: batch |> map(|r| gc_content(r.seq)) |> mean()})
```

### `zip` -- Pair Two Collections

```
# Pair forward and reverse reads
let r1 = read_fastq("sample_R1.fastq.gz")
let r2 = read_fastq("sample_R2.fastq.gz")

let paired = zip(r1, r2) |> map(|pair| {
  id: pair[0].id,
  r1_qual: mean(pair[0].quality),
  r2_qual: mean(pair[1].quality),
  combined_length: seq_len(pair[0].seq) + seq_len(pair[1].seq)
})
```

### `enumerate` -- Track Position

```
# Number contigs by size rank
read_fasta("assembly.fa")
  |> sort(|s| seq_len(s.seq), descending: true)
  |> enumerate()
  |> map(|i, seq| {rank: i + 1, id: seq.id, length: seq_len(seq.seq)})
  |> take(10)
  |> each(|row| print(f"#{row.rank}: {row.id} ({row.length} bp)"))
```

## Pipe Binding with `|> into`

When you want to capture an intermediate result from a pipe chain, the usual
approach is `let name = expr`. But this reads right-to-left, breaking the flow.
The `|> into` operator binds the result of a pipe chain to a name while
preserving left-to-right reading order:

```
# Traditional style — reads right-to-left:
let passed = variants |> filter(|v| v.quality >= 30)

# With |> into — reads left-to-right:
variants |> filter(|v| v.quality >= 30) |> into passed
```

The syntax `expr |> into name` is equivalent to `let name = expr`. It evaluates
the expression, binds the result to `name` (creating or shadowing), and returns
the value.

This is especially useful in multi-step workflows where you need to reference
intermediate results:

```
# Multi-step pipeline with into
read_fastq("reads.fq")
  |> filter(|r| mean_phred(r.quality) >= 30)
  |> into high_quality

high_quality |> map(|r| trim(r, 20)) |> into trimmed

print(f"Kept {len(high_quality)} reads, trimmed to {len(trimmed)}")
```

Each `|> into` step saves the result under a name so it can be reused later,
without wrapping the whole chain in `let x = (...)`. The pipeline reads
top-to-bottom, matching the conceptual flow of the analysis.

## Example: FASTQ Quality Filtering Pipeline

A complete quality filtering workflow with logging at each step.

```
# quality_filter.bl
# Multi-step FASTQ quality filtering pipeline.

let input_r1 = "raw_data/sample_R1.fastq.gz"
let input_r2 = "raw_data/sample_R2.fastq.gz"

# Process R1 and R2 in parallel-style
let process_reads = |input_path, label| {
  read_fastq(input_path)
    |>> |reads| print(f"[{label}] Input: {len(reads)} reads")

    # Step 1: Remove short reads
    |> filter(|r| seq_len(r.seq) >= 50)
    |>> |reads| print(f"[{label}] After length filter: {len(reads)}")

    # Step 2: Filter by mean quality
    |> filter(|r| mean(r.quality) >= 25)
    |>> |reads| print(f"[{label}] After quality filter: {len(reads)}")

    # Step 3: Remove low-complexity reads
    |> filter(|r| {
      let gc = gc_content(r.seq)
      gc > 0.1 && gc < 0.9
    })
    |>> |reads| print(f"[{label}] After complexity filter: {len(reads)}")
}

let filtered_r1 = process_reads(input_r1, "R1")
let filtered_r2 = process_reads(input_r2, "R2")

# Keep only concordant pairs (both mates survived)
let r1_ids = filtered_r1 |> map(|r| r.id) |> set()
let r2_ids = filtered_r2 |> map(|r| r.id) |> set()
let shared_ids = intersection(r1_ids, r2_ids)

let paired_r1 = filtered_r1 |> filter(|r| contains(shared_ids, r.id))
let paired_r2 = filtered_r2 |> filter(|r| contains(shared_ids, r.id))

print(f"\nFinal paired reads: {len(paired_r1)} pairs")

paired_r1 |> write_fastq("filtered/sample_R1.fastq.gz")
paired_r2 |> write_fastq("filtered/sample_R2.fastq.gz")

# Orphan reads (mate was filtered out)
let orphan_r1 = filtered_r1 |> filter(|r| !contains(shared_ids, r.id))
let orphan_r2 = filtered_r2 |> filter(|r| !contains(shared_ids, r.id))
let orphans = orphan_r1 ++ orphan_r2
print(f"Orphan reads: {len(orphans)}")
orphans |> write_fastq("filtered/sample_orphans.fastq.gz")
```

## Example: Variant Annotation Pipeline

Read VCF, classify variants, annotate against known databases, filter, and report.

```
# annotate_variants.bl
# Variant annotation and classification pipeline.

let known_pathogenic = csv("clinvar_pathogenic.csv")
  |> map(|r| f"{r.chrom}:{r.pos}:{r.ref}>{r.alt}")
  |> set()

let gnomad_af = csv("gnomad_af.csv")
  |> map(|r| {key: f"{r.chrom}:{r.pos}:{r.ref}>{r.alt}", af: into(r.af, "Float")})
  |> group_by("key")
  |> map(|g| {key: g.key, af: first(g.values).af})

read_vcf("somatic_calls.vcf.gz")

  # Step 1: Basic quality filter
  |> filter(|v| v.qual >= 30 && v.filter == "PASS")
  |>> |vs| print(f"After quality filter: {len(vs)}")

  # Step 2: Classify variant type
  |> map(|v| {
    ...v,
    key: f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}",
    vtype: variant_type(variant(v.chrom, v.pos, v.ref, v.alt))
  })

  # Step 3: Annotate with population frequency
  |> map(|v| {
    let gnomad = gnomad_af |> find(|g| g.key == v.key)
    {...v, gnomad_af: gnomad?.af ?? 0.0}
  })

  # Step 4: Annotate with ClinVar pathogenicity
  |> map(|v| {
    ...v,
    is_known_pathogenic: contains(known_pathogenic, v.key)
  })

  # Step 5: Assign priority tier
  |> map(|v| {
    let tier = if v.is_known_pathogenic then "Tier1_Known"
      else if v.gnomad_af < 0.001 && v.vtype != "SNV" then "Tier2_RareIndel"
      else if v.gnomad_af < 0.01 then "Tier3_Rare"
      else "Tier4_Common"
    {...v, tier: tier}
  })
  |>> |vs| {
    let tier_counts = vs |> group_by("tier") |> map(|g| f"{g.key}: {len(g.values)}")
    print(f"Tier distribution: {tier_counts}")
  }

  # Step 6: Filter to actionable variants
  |> filter(|v| v.tier == "Tier1_Known" || v.tier == "Tier2_RareIndel")
  |>> |vs| print(f"Actionable variants: {len(vs)}")

  # Step 7: Sort by priority and position
  |> sort(["tier", "chrom", "pos"])

  # Step 8: Generate report
  |> map(|v| {
    chrom: v.chrom,
    pos: v.pos,
    ref_allele: v.ref,
    alt_allele: v.alt,
    type: v.vtype,
    qual: v.qual,
    gnomad_af: v.gnomad_af,
    tier: v.tier,
    clinvar: if v.is_known_pathogenic then "Pathogenic" else "."
  })
  |> write_tsv("annotated_actionable.csv")
```

## Example: Multi-Sample Comparison with Zip and Reduce

Compare variant calls across matched tumor-normal pairs and compute concordance.

```
# multi_sample_compare.bl
# Compare variant calls across tumor-normal pairs.

let pairs = [
  {tumor: "patient1_tumor.vcf.gz", normal: "patient1_normal.vcf.gz", patient: "P001"},
  {tumor: "patient2_tumor.vcf.gz", normal: "patient2_normal.vcf.gz", patient: "P002"},
  {tumor: "patient3_tumor.vcf.gz", normal: "patient3_normal.vcf.gz", patient: "P003"}
]

# Process each pair
let pair_results = pairs |> map(|pair| {
  let tumor_vars = read_vcf(pair.tumor)
    |> filter(|v| v.qual >= 30)
    |> map(|v| f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}")
    |> set()

  let normal_vars = read_vcf(pair.normal)
    |> filter(|v| v.qual >= 30)
    |> map(|v| f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}")
    |> set()

  let somatic = difference(tumor_vars, normal_vars)
  let germline = intersection(tumor_vars, normal_vars)
  let normal_only = difference(normal_vars, tumor_vars)

  {
    patient: pair.patient,
    tumor_total: len(tumor_vars),
    normal_total: len(normal_vars),
    somatic: len(somatic),
    germline: len(germline),
    normal_only: len(normal_only),
    somatic_variants: somatic
  }
})

# Summary table
let summary = table(pair_results |> map(|r| {
  patient: r.patient,
  tumor_total: r.tumor_total,
  normal_total: r.normal_total,
  somatic: r.somatic,
  germline: r.germline,
  somatic_rate: r.somatic / r.tumor_total * 100.0
}))

print("Patient Variant Comparison")
print("=" * 70)
summary |> each(|row|
  print(f"  {row.patient}: tumor={row.tumor_total}, somatic={row.somatic} ({row.somatic_rate:.1f}%), germline={row.germline}")
)

# Find recurrent somatic variants (present in 2+ patients)
let all_somatic = pair_results
  |> flat_map(|r| r.somatic_variants |> collect() |> map(|v| {variant: v, patient: r.patient}))
  |> group_by("variant")
  |> filter(|g| len(g.values) >= 2)
  |> map(|g| {
    variant: g.key,
    n_patients: len(g.values),
    patients: g.values |> map(|v| v.patient)
  })
  |> sort("n_patients", descending: true)

print(f"\nRecurrent somatic variants (in >= 2 patients): {len(all_somatic)}")
all_somatic |> take(20) |> each(|v|
  print(f"  {v.variant} -- found in {v.n_patients} patients: {v.patients}")
)

# Cross-patient aggregates via reduce
let totals = pair_results |> reduce(
  {total_somatic: 0, total_germline: 0, patients: 0},
  |acc, r| {
    total_somatic: acc.total_somatic + r.somatic,
    total_germline: acc.total_germline + r.germline,
    patients: acc.patients + 1
  }
)

print(f"\nCohort totals across {totals.patients} patients:")
print(f"  Mean somatic variants: {totals.total_somatic / totals.patients:.0f}")
print(f"  Mean germline variants: {totals.total_germline / totals.patients:.0f}")

# Zip tumor-normal read depths for concordance check
let p1_tumor_depths = read_vcf(pairs[0].tumor) |> map(|v| into(v.info?.DP ?? "0", "Int"))
let p1_normal_depths = read_vcf(pairs[0].normal) |> map(|v| into(v.info?.DP ?? "0", "Int"))

let depth_comparison = zip(p1_tumor_depths, p1_normal_depths)
  |> map(|pair| {tumor_dp: pair[0], normal_dp: pair[1], ratio: pair[0] / (pair[1] + 1.0)})
  |> filter(|d| d.ratio > 3.0)   # flag positions with 3x tumor vs normal depth

print(f"\nPatient 1 depth outliers (tumor/normal > 3x): {len(depth_comparison)}")
```

## Summary

Pipes and transforms are the backbone of BioLang. The `|>` operator turns nested
function calls into linear, readable workflows. The tap pipe `|>>` lets you
observe and debug without disrupting the data flow. The `|> into` operator
captures intermediate results without breaking left-to-right reading order.
Higher-order functions like `map`, `filter`, `reduce`, `flat_map`, `zip`, and
`window` cover the full range of data transformation patterns you encounter in
bioinformatics pipelines.

Every example in this chapter represents a real analysis pattern: quality filtering,
variant annotation, multi-sample comparison. As your pipelines grow more complex,
pipes keep them readable and composable.
