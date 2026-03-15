# Chapter 11: Pipelines

Bioinformatics workflows are rarely a single step. A typical analysis chains
quality control, alignment, variant calling, filtering, and annotation into a
multi-stage workflow. BioLang's pipe-first design with `|>` makes these
workflows natural to write. For larger workflows with named stages, BioLang
also provides `pipeline` blocks with `stage` declarations.

## Pipe Composition: The Primary Pipeline Mechanism

The pipe operator `|>` is BioLang's core tool for building pipelines. It
inserts the left-hand value as the first argument to the right-hand function,
creating readable chains of transformations.

```biolang
# Single-step pipe
let reads = read_fastq("data/reads.fastq")

# Multi-step pipeline via pipe chaining
let result = read_fastq("data/reads.fastq")
  |> filter(|r| mean_phred(r.quality) >= 30)
  |> map(|r| {seq: r.sequence, gc: gc_content(r.sequence)})
  |> sort_by(|a, b| b.gc - a.gc)

println("Top GC: " + str(result[0].gc))
```

Each `|>` feeds its result forward. You can chain as many steps as you need
with no special syntax. This is how most BioLang pipelines are written.

## Multi-Step Workflows with Variable Bindings

For workflows where intermediate results need names, use sequential `let`
bindings connected by pipes. Each step is explicit and inspectable.

```biolang
# FASTQ QC pipeline
let raw = read_fastq("data/reads.fastq")

let stats = raw
  |> map(|r| mean_phred(r.quality))
let avg_quality = mean(stats)
let total_reads = len(raw)

println("Reads: " + str(total_reads) + ", Mean Q: " + str(round(avg_quality, 1)))

# Filter and write
let passed = read_fastq("data/reads.fastq")
  |> filter(|r| mean_phred(r.quality) >= 25)
  |> collect()

write_fastq(passed, "sample_R1.filtered.fastq.gz")
println("Kept " + str(len(passed)) + " of " + str(total_reads) + " reads")
```

This pattern gives you full control: name any intermediate, inspect it, branch
on it, or feed it into multiple downstream steps.

## Group, Summarize, and Count

BioLang's table operations work naturally in pipe chains for aggregation
workflows.

```biolang
# Variant summary pipeline
let summary = read_vcf("data/variants.vcf")
  |> filter(|v| v.qual >= 30)
  |> classify_variants()
  |> group_by("type")
  |> count_by("type")

println(summary)

# Per-chromosome depth analysis
let depths = read_bed("data/regions.bed")
  |> group_by("chrom")
  |> summarize(|chrom, rows| {chrom: chrom, mean_depth: mean(col(rows, "score"))})
  |> arrange("chrom")

println(depths)
```

The `group_by`, `count_by`, `filter_by`, `summarize`, and `arrange` builtins
all accept pipe input, so they compose seamlessly.

## Pipeline Blocks

For larger workflows with named stages, BioLang provides `pipeline` blocks.
A `pipeline` declares a named workflow. Inside the block you can use `stage`
declarations to name intermediate results with the arrow (`->`) syntax.

```biolang
pipeline fastq_qc {
  # stage name -> expression
  stage raw_stats -> read_fastq("data/reads.fastq")
    |> map(|r| mean_phred(r.quality))
    |> mean()

  stage filtered -> read_fastq("data/reads.fastq")
    |> filter(|r| mean_phred(r.quality) >= 30)

  stage write_out -> write_fastq(filtered, "sample_R1.filtered.fastq.gz")

  stage report -> {
    mean_quality: raw_stats,
    kept: len(filtered),
  }
}

# fastq_qc is bound to the result of the last stage
println("Mean Q: " + str(fastq_qc.mean_quality))
```

When a `pipeline` block has no parameters, it executes immediately and binds
its result to the pipeline name. Each `stage name -> expr` evaluates the
expression and makes the result available by name to subsequent stages.

## Passing Data Between Stages

Stage names become bindings that later stages can reference. This creates an
implicit dependency chain.

```biolang
pipeline align_and_call {
  stage aligned -> shell("bwa-mem2 mem -t 16 GRCh38.fa sample_R1.fq.gz sample_R2.fq.gz | samtools sort -@ 8 -o aligned.sorted.bam")

  stage indexed -> shell("samtools index " + aligned)

  stage variants -> shell("bcftools mpileup -f GRCh38.fa " + aligned + " | bcftools call -mv -Oz -o variants.vcf.gz")

  stage stats -> read_vcf("data/variants.vcf")
    |> filter(|v| v.qual >= 30)
    |> classify_variants()
    |> count_by("type")
}

println(align_and_call)
```

Each stage name resolves to whatever its expression evaluated to. The `aligned`
stage returns a string (the shell output), which `indexed` and `variants`
reference directly.

## Parameterized Pipelines (Templates)

A pipeline can accept parameters, turning it into a reusable template. When you
declare `pipeline name(params) { ... }`, BioLang defines a callable function
instead of executing immediately.

```biolang
# Define a reusable alignment template
pipeline align_sample(sample_id, r1, r2, reference) {
  stage sorted -> shell("bwa-mem2 mem -t 16 " + reference + " " + r1 + " " + r2
        + " | samtools sort -@ 8 -o " + sample_id + ".sorted.bam")

  stage indexed -> shell("samtools index " + sorted)

  # Pipeline returns its last stage value
  sorted
}

# Call with different inputs
let tumor_bam = align_sample("tumor", "tumor_R1.fq.gz", "tumor_R2.fq.gz", "GRCh38.fa")
let normal_bam = align_sample("normal", "normal_R1.fq.gz", "normal_R2.fq.gz", "GRCh38.fa")
println("Tumor BAM: " + tumor_bam)
println("Normal BAM: " + normal_bam)
```

Parameterized pipelines behave exactly like functions. You can loop over a
sample list to process a whole cohort:

```biolang
let samples = [
  {id: "S1", r1: "S1_R1.fq.gz", r2: "S1_R2.fq.gz"},
  {id: "S2", r1: "S2_R1.fq.gz", r2: "S2_R2.fq.gz"},
  {id: "S3", r1: "S3_R1.fq.gz", r2: "S3_R2.fq.gz"},
]

let bams = samples |> map(|s| align_sample(s.id, s.r1, s.r2, "GRCh38.fa"))
println("Aligned " + str(len(bams)) + " samples")
```

## Parallel Processing

### par_map and par_filter

For data-parallel operations on lists, `par_map` and `par_filter` distribute
work across available cores.

```biolang
let sequences = read_fasta("data/sequences.fasta")

# Compute GC content in parallel
let gc_values = sequences
  |> par_map(|seq| {name: seq.name, gc: gc_content(seq.sequence)})

# Filter in parallel
let high_gc = gc_values
  |> par_filter(|s| s.gc > 0.6)

println("High GC sequences: " + str(len(high_gc)))
```

### parallel for

For workflows that need to run a block of statements per item, `parallel for`
fans out iterations. In the current tree-walking interpreter these run
sequentially; a future bytecode backend will parallelize them.

```biolang
let samples = [
  {id: "tumor_01", r1: "t01_R1.fq.gz", r2: "t01_R2.fq.gz"},
  {id: "tumor_02", r1: "t02_R1.fq.gz", r2: "t02_R2.fq.gz"},
  {id: "normal_01", r1: "n01_R1.fq.gz", r2: "n01_R2.fq.gz"},
]

parallel for sample in samples {
  let bam = shell("bwa-mem2 mem -t 4 GRCh38.fa " + sample.r1 + " " + sample.r2
                   + " | samtools sort -@ 4 -o " + sample.id + ".sorted.bam")
  shell("samtools index " + sample.id + ".sorted.bam")
  println("Done: " + sample.id)
}
```

The result of a `parallel for` is the value of the last iteration's block.

## Shell Integration

The `shell()` builtin executes external commands and returns their stdout as a
string. This is how BioLang integrates with existing bioinformatics tools.

```biolang
# Run a single command
let flagstat = shell("samtools flagstat aligned.bam")
println(flagstat)

# Chain shell commands in a pipeline
let bam = shell("bwa-mem2 mem -t 16 ref.fa R1.fq.gz R2.fq.gz | samtools sort -@ 8 -o out.bam")

# Use shell output in downstream BioLang processing
let depth_lines = shell("samtools depth aligned.bam")
let depths = depth_lines
  |> split("\n")
  |> filter(|line| len(line) > 0)
  |> map(|line| split(line, "\t"))
  |> map(|parts| float(parts[2]))

println("Mean depth: " + str(mean(depths)))
```

## Defer for Cleanup

`defer` registers an expression that runs when the enclosing scope exits,
whether it succeeds or fails. This keeps intermediate files from piling up.

```biolang
pipeline trimmed_alignment {
  stage trimmed -> shell("fastp -i sample_R1.fq.gz -I sample_R2.fq.gz -o trimmed_R1.fq.gz -O trimmed_R2.fq.gz")

  defer shell("rm -f trimmed_R1.fq.gz trimmed_R2.fq.gz")

  stage aligned -> shell("bwa-mem2 mem -t 16 GRCh38.fa trimmed_R1.fq.gz trimmed_R2.fq.gz | samtools sort -@ 8 -o aligned.bam")

  stage indexed -> shell("samtools index " + aligned)

  aligned
}
# When the pipeline finishes, the deferred rm command fires
```

## Error Handling in Pipelines

Wrap pipeline steps in `try`/`catch` to handle failures gracefully without
aborting the entire workflow.

```biolang
let samples = ["sample_A", "sample_B", "sample_C"]

let results = samples |> map(|name| {
  try {
    let bam = shell("bwa-mem2 mem -t 8 ref.fa " + name + "_R1.fq.gz " + name + "_R2.fq.gz | samtools sort -o " + name + ".bam")
    shell("samtools index " + name + ".bam")
    {id: name, status: "ok", bam: name + ".bam"}
  } catch e {
    println("WARN: " + name + " failed: " + str(e))
    {id: name, status: "failed", bam: nil}
  }
})

let passed = results |> filter(|r| r.status == "ok")
let failed = results |> filter(|r| r.status == "failed")
println(str(len(passed)) + " succeeded, " + str(len(failed)) + " failed")
```

You can also use `try`/`catch` around individual stages within a pipeline block
to continue past non-critical failures.

## Example: FASTQ QC Pipeline

A complete quality-control workflow using only pipe composition.

```biolang
# Read and analyze
let reads = read_fastq("data/reads.fastq")
let qualities = reads |> map(|r| mean_phred(r.quality))

let qc = {
  total_reads: len(reads),
  mean_quality: round(mean(qualities), 2),
  median_quality: round(median(qualities), 2),
  q30_pct: round(len(filter(qualities, |q| q >= 30)) / len(qualities) * 100, 1),
  gc_mean: round(mean(reads |> map(|r| gc_content(r.sequence))), 4),
}

println("=== FASTQ QC Report ===")
println("Total reads:    " + str(qc.total_reads))
println("Mean quality:   " + str(qc.mean_quality))
println("Median quality: " + str(qc.median_quality))
println("Q30%:           " + str(qc.q30_pct) + "%")
println("Mean GC:        " + str(qc.gc_mean))

# Filter and write passing reads
let passed = reads |> filter(|r| mean_phred(r.quality) >= 25)
write_fastq(passed, "sample_R1.qc_passed.fastq.gz")
println("Wrote " + str(len(passed)) + " passing reads")

# Notify on completion
notify("FASTQ QC complete: " + str(qc.total_reads) + " reads, Q30=" + str(qc.q30_pct) + "%")
```

## Example: Variant Calling Pipeline

Germline variant calling from FASTQ to filtered VCF using pipeline blocks.

```biolang
pipeline germline_variants {
  stage aligned -> shell(
    "bwa-mem2 mem -t 16 -R '@RG\\tID:S1\\tSM:sample' GRCh38.fa "
    + "sample_R1.fq.gz sample_R2.fq.gz "
    + "| samtools sort -@ 8 -o sample.sorted.bam"
  )

  stage indexed -> shell("samtools index " + aligned)

  stage called -> shell(
    "gatk HaplotypeCaller -I " + aligned
    + " -R GRCh38.fa -L exome_targets.bed -O sample.g.vcf.gz -ERC GVCF"
  )

  stage genotyped -> shell(
    "gatk GenotypeGVCFs -V " + called + " -R GRCh38.fa -O sample.vcf.gz"
  )

  stage filtered -> shell(
    "gatk VariantFiltration -V " + genotyped
    + " --filter-name 'LowQD' --filter-expression 'QD < 2.0'"
    + " --filter-name 'HighFS' --filter-expression 'FS > 60.0'"
    + " -O sample.filtered.vcf.gz"
  )

  # Analyze the results in BioLang
  stage summary -> read_vcf("data/variants.vcf")
    |> filter(|v| v.filter == "PASS")
    |> classify_variants()
    |> count_by("type")
}

println(germline_variants)
```

## Example: Multi-Sample Analysis

Processing a cohort with parameterized pipelines and aggregation.

```biolang
# Reusable per-sample pipeline
pipeline process_sample(sample_id, r1, r2) {
  stage bam -> shell(
    "bwa-mem2 mem -t 8 -R '@RG\\tID:" + sample_id + "\\tSM:" + sample_id
    + "' GRCh38.fa " + r1 + " " + r2
    + " | samtools sort -@ 4 -o " + sample_id + ".sorted.bam"
  )

  stage idx -> shell("samtools index " + bam)

  stage depth_info -> shell("samtools depth " + bam)
    |> split("\n")
    |> filter(|line| len(line) > 0)
    |> map(|line| float(split(line, "\t")[2]))
    |> mean()

  {id: sample_id, bam: bam, mean_depth: depth_info}
}

# Load sample manifest
let manifest = read_csv("data/sample_sheet.csv")

# Process all samples
let results = manifest
  |> map(|row| {
    try {
      process_sample(row.sample_id, row.fastq_r1, row.fastq_r2)
    } catch e {
      println("WARN: " + row.sample_id + " failed: " + str(e))
      {id: row.sample_id, bam: nil, mean_depth: 0.0}
    }
  })

# QC gate: check depth thresholds
let passed = results |> filter(|s| s.mean_depth >= 20.0)
let failed = results |> filter(|s| s.mean_depth < 20.0)

if len(failed) > 0 then
  println("WARNING: " + str(len(failed)) + " samples below 20x depth:")
each(failed, |s| println("  " + s.id + ": " + str(round(s.mean_depth, 1)) + "x"))

println(str(len(passed)) + " of " + str(len(results)) + " samples passed QC")

# Write summary table
let summary = results
  |> map(|s| {sample: s.id, depth: round(s.mean_depth, 1), pass: s.mean_depth >= 20.0})
write_tsv(summary, "cohort_qc_summary.csv")

# Notify the team
slack("Cohort processing complete: " + str(len(passed)) + "/" + str(len(results)) + " passed QC")
```

## Pipeline Composition

Pipelines are values. Parameterized pipelines are functions. You can call one
from another to build layered workflows.

```biolang
pipeline align_one(sample) {
  stage sorted -> shell(
    "bwa-mem2 mem -t 8 GRCh38.fa " + sample.r1 + " " + sample.r2
    + " | samtools sort -@ 4 -o " + sample.id + ".sorted.bam"
  )
  stage idx -> shell("samtools index " + sorted)
  sorted
}

pipeline somatic_pair(tumor, normal) {
  stage tumor_bam -> align_one(tumor)
  stage normal_bam -> align_one(normal)

  stage called -> shell(
    "gatk Mutect2 -I " + tumor_bam + " -I " + normal_bam
    + " -R GRCh38.fa -O somatic.vcf.gz"
  )

  stage filtered -> shell(
    "gatk FilterMutectCalls -V " + called + " -R GRCh38.fa -O somatic.filtered.vcf.gz"
  )

  let variants = read_vcf("data/variants.vcf")
    |> filter(|v| v.filter == "PASS")
  println("Found " + str(len(variants)) + " somatic variants")
  filtered
}

let result = somatic_pair(
  {id: "tumor", r1: "tumor_R1.fq.gz", r2: "tumor_R2.fq.gz"},
  {id: "normal", r1: "normal_R1.fq.gz", r2: "normal_R2.fq.gz"}
)
```

## Summary

BioLang pipelines come in two flavors:

- **Pipe chains** (`|>`): The everyday tool. Chain any sequence of
  transformations into a readable, left-to-right flow. No special syntax
  needed.

- **Pipeline blocks** (`pipeline name { stage x -> expr }`): For larger
  workflows where you want named stages, parameterized templates, and
  stage-to-stage data passing.

Both approaches compose freely. Use `par_map` and `par_filter` for
data-parallel operations, `parallel for` for per-item workflows, `shell()` to
call external tools, `try`/`catch` for error resilience, and `defer` for
cleanup. The result is a language where analysis pipelines are just ordinary
code, with no framework overhead.
