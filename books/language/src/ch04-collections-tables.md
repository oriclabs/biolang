# Chapter 4: Collections and Tables

Bioinformatics data is inherently tabular and set-oriented. BioLang provides
Lists, Sets, Tables, and Ranges as first-class collection types, with rich
operations for filtering, grouping, and summarizing biological datasets.

## Lists

Lists are ordered, heterogeneous sequences:

```
let chromosomes = ["chr1", "chr2", "chr3", "chrX", "chrY"]
let read_lengths = [150, 151, 149, 150, 148, 150]
let mixed = ["TP53", 42, true, dna"ATCG"]
```

Lists support standard access and manipulation:

```
let genes = ["BRCA1", "TP53", "EGFR", "KRAS", "BRAF"]

print(genes[0])        # => "BRCA1"
print(genes[-1])       # => "BRAF"
print(len(genes))      # => 5

# Concatenation
let more = genes ++ ["PIK3CA", "PTEN"]

# Nested lists (e.g., paired-end read pairs)
let pairs = [
  ["sample1_R1.fq.gz", "sample1_R2.fq.gz"],
  ["sample2_R1.fq.gz", "sample2_R2.fq.gz"]
]
print(pairs[0][1])   # => "sample1_R2.fq.gz"
```

## Sets

Sets are unordered collections of unique elements. They are the right tool for
gene lists, sample IDs, and any membership-testing workload:

```
let panel_genes = set(["BRCA1", "BRCA2", "TP53", "EGFR", "KRAS", "BRAF"])
let mutated_genes = set(["TP53", "KRAS", "APC", "PIK3CA"])

# Set operations
let overlap = intersection(panel_genes, mutated_genes)
print(overlap)   # => set(["TP53", "KRAS"])

let all_genes = union(panel_genes, mutated_genes)
let panel_only = difference(panel_genes, mutated_genes)
let mutated_only = difference(mutated_genes, panel_genes)

print(f"On panel and mutated: {len(overlap)}")
print(f"Mutated, not on panel: {len(mutated_only)}")

# Membership
print(contains(panel_genes, "TP53"))   # => true
```

A real use case -- finding shared variants between tumor and normal:

```
let tumor_variants = read_vcf("tumor.vcf.gz")
  |> map(|v| f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}")
  |> set()

let normal_variants = read_vcf("normal.vcf.gz")
  |> map(|v| f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}")
  |> set()

let somatic = difference(tumor_variants, normal_variants)
let germline = intersection(tumor_variants, normal_variants)
print(f"Somatic: {len(somatic)}, Germline: {len(germline)}")
```

## Tables

Tables are the central data structure for tabular bioinformatics data. They are
created from lists of records:

```
let samples = table([
  {sample_id: "S001", tissue: "tumor",  reads: 42_000_000, coverage: 35.2},
  {sample_id: "S002", tissue: "normal", reads: 38_000_000, coverage: 31.4},
  {sample_id: "S003", tissue: "tumor",  reads: 51_000_000, coverage: 42.1},
  {sample_id: "S004", tissue: "normal", reads: 44_000_000, coverage: 36.8}
])
```

### Column Access

```
let coverages = samples |> select("coverage")
print(coverages)   # => [35.2, 31.4, 42.1, 36.8]

# Multiple columns
let subset = samples |> select("sample_id", "coverage")
```

### Row Iteration

```
samples |> each(|row| {
  print(f"{row.sample_id}: {row.coverage}x ({row.tissue})")
})
```

## Table Operations

### `select` -- Choose Columns

```
let qc_summary = full_table |> select("sample_id", "total_reads", "pct_mapped", "mean_coverage")
```

### `mutate` -- Add or Transform Columns

```
let enriched = samples
  |> mutate("reads_millions", |row| row.reads / 1_000_000.0)
  |> mutate("is_high_coverage", |row| row.coverage >= 30.0)
  |> mutate("label", |row| f"{row.sample_id}_{row.tissue}")
```

### `summarize` -- Aggregate Statistics

```
let coverages = samples |> select("coverage")
let all_reads = samples |> select("reads")

let stats = {
  n_samples: len(samples),
  total_reads: sum(all_reads),
  mean_coverage: mean(coverages),
  min_coverage: min(coverages),
  max_coverage: max(coverages),
}

print(f"Samples: {stats.n_samples}, Mean coverage: {stats.mean_coverage:.1f}x")
```

### `group_by` -- Split-Apply-Combine

```
let by_tissue = samples
  |> group_by("tissue")
  |> summarize(|tissue, group| {
    tissue: tissue,
    n: len(group),
    mean_cov: mean(group |> select("coverage")),
    mean_reads: mean(group |> select("reads")),
  })

by_tissue |> each(|row|
  print(f"{row.tissue}: n={row.n}, cov={row.mean_cov:.1f}x, reads={row.mean_reads:.0f}")
)
```

### `sort` -- Order Rows

```
# Sort by coverage descending
let ranked = samples |> sort_by(|row| -row.coverage)

# Sort by a field ascending
let ordered = samples |> sort_by(|row| row.tissue)
```

### `filter` -- Select Rows by Condition

```
let high_cov = samples |> filter(|row| row.coverage >= 30.0)
let tumor_only = samples |> filter(|row| row.tissue == "tumor")

# Chained filters
let good_tumor = samples
  |> filter(|row| row.tissue == "tumor")
  |> filter(|row| row.coverage >= 30.0)
  |> filter(|row| row.reads >= 40_000_000)
```

### Joins (Conceptual)

Table joins are planned for a future release. Currently, use `map` with lookups:

```
let annotations = table([
  {sample_id: "S001", patient: "P101", stage: "III"},
  {sample_id: "S002", patient: "P101", stage: "III"},
  {sample_id: "S003", patient: "P202", stage: "II"}
])

# Manual left join via lookup
let ann_map = annotations |> group_by("sample_id")
let joined = samples |> map(|row| {
  let ann_rows = ann_map[row.sample_id]
  let ann = if !is_nil(ann_rows) then first(ann_rows) else nil
  {...row, patient: ann?.patient ?? "unknown", stage: ann?.stage ?? "unknown"}
})
```

## Ranges

Ranges generate sequences of integers, useful for genomic coordinate windows:

```
let indices = 0..10          # [0, 1, 2, ..., 9]
let inclusive = 0..=10       # [0, 1, 2, ..., 10]
let stepped = 0..100..10    # [0, 10, 20, ..., 90]

# Chromosome positions in 1 Mb windows
let chrom_length = 248_956_422   # chr1
let windows = 0..chrom_length..1_000_000
  |> map(|start| {
    chrom: "chr1",
    start: start,
    end: min(start + 1_000_000, chrom_length)
  })

print(f"Generated {len(windows)} windows for chr1")
```

## Example: Sample QC Summary Table from FASTQ Stats

Read multiple FASTQ files, compute QC metrics, and build a summary table.

```
# fastq_qc_summary.bl
# Build a QC summary table from raw FASTQ files.

let sample_sheet = csv("samples.csv")

let read_stats = |reads| {
  let quals = reads |> map(|r| mean(r.quality))
  let lengths = reads |> map(|r| seq_len(r.seq))
  let gc_vals = reads |> map(|r| gc_content(r.seq))
  {
    total_reads: len(reads),
    total_bases: sum(lengths),
    mean_length: mean(lengths),
    mean_quality: mean(quals),
    q30_pct: reads |> filter(|r| mean(r.quality) >= 30) |> len() / len(reads) * 100.0,
    gc_pct: mean(gc_vals) * 100.0
  }
}

let qc_results = sample_sheet |> map(|row| {
  let r1 = read_fastq(row.fastq_r1)
  let r1_stats = read_stats(r1)
  let r2_stats = if !is_nil(row.fastq_r2) then
    read_stats(read_fastq(row.fastq_r2))
  else nil

  {
    sample_id: row.sample_id,
    total_reads: r1_stats.total_reads + (r2_stats?.total_reads ?? 0),
    total_bases: r1_stats.total_bases + (r2_stats?.total_bases ?? 0),
    mean_length: r1_stats.mean_length,
    mean_quality: r1_stats.mean_quality,
    q30_pct: r1_stats.q30_pct,
    gc_pct: r1_stats.gc_pct
  }
})

let qc_table = table(qc_results)

# Add pass/fail column
let qc_flagged = qc_table |> mutate("qc_pass", |row| row.mean_quality >= 30.0 && row.q30_pct >= 80.0 && row.total_reads >= 10_000_000)

# Summary by pass/fail
let summary = qc_flagged |> group_by("qc_pass") |> summarize(|pass, group| {
  qc_pass: pass,
  count: len(group),
  mean_reads: mean(group |> select("total_reads")),
  mean_q: mean(group |> select("mean_quality")),
})

print("QC Summary:")
print("=" * 60)
qc_flagged |> each(|row| {
  let flag = if row.qc_pass then "PASS" else "FAIL"
  print(f"  [{flag}] {row.sample_id}: {row.total_reads / 1e6:.1f}M reads, Q={row.mean_quality:.1f}, Q30={row.q30_pct:.1f}%")
})

let pass_count = qc_flagged |> filter(|r| r.qc_pass) |> len()
let fail_count = qc_flagged |> filter(|r| !r.qc_pass) |> len()
print(f"\n{pass_count} passed, {fail_count} failed out of {len(qc_flagged)} samples")

# Write results
qc_flagged |> write_tsv("qc_summary.csv")
```

## Example: Gene Expression Matrix -- Filter and Normalize

Load a counts matrix, filter low-count genes, and apply TPM normalization.

```
# expression_normalize.bl
# Filter low-count genes and compute TPM from a raw counts matrix.

let counts = csv("gene_counts.csv")   # rows = genes, columns = samples
let gene_lengths = csv("gene_lengths.csv")  # gene_id, length

# Build a length lookup
let length_map = gene_lengths |> group_by("gene_id")

# Convert to table with gene metadata
let expr_table = table(counts)
let sample_cols = columns(expr_table) |> filter(|c| c != "gene_id")

# Filter: keep genes with >= 10 counts in at least 3 samples
let filtered = expr_table |> filter(|row| {
  let expressed = sample_cols |> filter(|col| row[col] >= 10) |> len()
  expressed >= 3
})

print(f"Genes before filter: {len(expr_table)}")
print(f"Genes after filter: {len(filtered)}")

# RPK: reads per kilobase — transform each row
let rpk_table = filtered |> map(|gene_row| {
  let gene_group = length_map[gene_row.gene_id]
  let gene_len = if !is_nil(gene_group) then first(gene_group) else nil
  let len_kb = (gene_len?.length ?? 1000) / 1000.0
  let updated = sample_cols |> reduce({...gene_row}, |acc, col| {
    ...acc, [col]: gene_row[col] / len_kb
  })
  updated
}) |> table()

# TPM: normalize RPK to sum to 1 million per sample
let rpk_sums = sample_cols |> map(|col| {
  col: col,
  total: rpk_table |> select(col) |> sum()
})

let tpm_table = rpk_table |> map(|row| {
  sample_cols |> reduce({...row}, |acc, col| {
    let col_sum = rpk_sums |> find(|s| s.col == col)
    {...acc, [col]: (row[col] / col_sum.total) * 1e6}
  })
}) |> table()

# Summary statistics per gene
let gene_stats = tpm_table
  |> mutate("mean_tpm", |row| sample_cols |> map(|c| row[c]) |> mean())
  |> mutate("max_tpm", |row| sample_cols |> map(|c| row[c]) |> max())
  |> mutate("cv", |row| {
    let vals = sample_cols |> map(|c| row[c])
    stdev(vals) / (mean(vals) + 1e-10)
  })

# Top variable genes
let top_variable = gene_stats
  |> sort_by(|g| -g.cv)
  |> take(20)

print("\nTop 20 most variable genes (by CV):")
top_variable |> each(|g|
  print(f"  {g.gene_id}: mean TPM={g.mean_tpm:.1f}, CV={g.cv:.2f}")
)

tpm_table |> write_tsv("tpm_normalized.csv")
```

## Example: Merge Variant Calls from Multiple Samples

Combine VCF calls from multiple samples into a unified genotype matrix.

```
# merge_variants.bl
# Merge variant calls from multiple samples into a unified table.

let sample_vcfs = [
  {sample: "tumor_A",  vcf: "tumor_A.vcf.gz"},
  {sample: "tumor_B",  vcf: "tumor_B.vcf.gz"},
  {sample: "normal_A", vcf: "normal_A.vcf.gz"},
  {sample: "normal_B", vcf: "normal_B.vcf.gz"}
]

# Read all variants with sample tag
let all_calls = sample_vcfs |> flat_map(|s| {
  read_vcf(s.vcf) |> map(|v| {
    key: f"{v.chrom}:{v.pos}:{v.ref}>{v.alt}",
    chrom: v.chrom,
    pos: v.pos,
    ref_allele: v.ref,
    alt_allele: v.alt,
    sample: s.sample,
    qual: into(v.qual ?? "0", "Float"),
    depth: into(v.info?.DP ?? "0", "Int"),
    af: into(v.info?.AF ?? "0.0", "Float"),
    genotype: v.samples?.GT ?? "./."
  })
})

# Collect unique variant sites
let unique_sites = all_calls
  |> map(|v| v.key)
  |> unique()
  |> sort()

print(f"Total calls: {len(all_calls)}")
print(f"Unique variant sites: {len(unique_sites)}")

# Build genotype matrix: one row per variant site, one column per sample
let sample_names = sample_vcfs |> map(|s| s.sample)
let calls_by_key = all_calls |> group_by("key")

let merged = unique_sites |> map(|site_key| {
  let site_vals = calls_by_key[site_key]
  let first_call = first(site_vals)

  let base = {
    chrom: first_call.chrom,
    pos: first_call.pos,
    ref_allele: first_call.ref_allele,
    alt_allele: first_call.alt_allele,
    n_samples: len(site_vals)
  }

  # Add per-sample genotype columns
  let gt_fields = sample_names |> reduce({}, |acc, sname| {
    let call = site_vals |> find(|v| v.sample == sname)
    {...acc, [f"{sname}_GT"]: call?.genotype ?? "./.", [f"{sname}_AF"]: call?.af ?? 0.0}
  })

  {...base, ...gt_fields}
})

let merged_table = table(merged)

# Find variants present in tumor but absent in normal (somatic candidates)
let somatic = merged_table |> filter(|row| {
  let tumor_present = row.tumor_A_GT != "./." || row.tumor_B_GT != "./."
  let normal_absent = row.normal_A_GT == "./." && row.normal_B_GT == "./."
  tumor_present && normal_absent
})

let shared_tumor = merged_table |> filter(|row|
  row.tumor_A_GT != "./." && row.tumor_B_GT != "./."
)

print(f"\nSomatic candidates: {len(somatic)}")
print(f"Shared across tumors: {len(shared_tumor)}")
print(f"\nTop somatic candidates:")
somatic
  |> sort_by(|v| -v.n_samples)
  |> take(10)
  |> each(|v| print(f"  {v.chrom}:{v.pos} {v.ref_allele}>{v.alt_allele} (in {v.n_samples} sample(s))"))

merged_table |> write_tsv("merged_genotypes.csv")
```

## Summary

Collections and tables give you the tools to handle the data shapes that
dominate bioinformatics: sample sheets, QC matrices, expression counts,
and multi-sample variant calls. Combined with `group_by`, `summarize`,
and `mutate`, you can express complex data wrangling pipelines concisely.

The next chapter introduces pipes and transforms -- the core of BioLang's
workflow composition model.
