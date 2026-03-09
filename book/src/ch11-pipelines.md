# Chapter 11: Pipelines

Bioinformatics workflows are rarely a single step. A typical analysis chains
together quality control, alignment, variant calling, filtering, and annotation
into a directed acyclic graph of stages. BioLang provides first-class pipeline
blocks that make these multi-stage workflows explicit, reproducible, and
parallel-aware.

## Pipeline Blocks

A `pipeline` block declares a named workflow composed of `stage` blocks. Stages
execute sequentially by default, and each stage can reference the output of the
previous stage.

```
pipeline fastq_qc {
  stage raw_stats {
    read_fastq("sample_R1.fastq.gz")
      |> read_stats()
  }

  stage filter {
    read_fastq("sample_R1.fastq.gz")
      |> filter(|r| mean(r.quality) >= 30)
      |> write_fastq("sample_R1.filtered.fastq.gz")
  }

  stage filtered_stats {
    read_fastq("sample_R1.filtered.fastq.gz")
      |> read_stats()
  }
}
```

When you invoke `fastq_qc`, BioLang runs `raw_stats`, then `filter`, then
`filtered_stats` in order. The return value of the pipeline is the result of
the last stage.

## Passing Data Between Stages

Stages can capture the result of a previous stage by name. The stage name
becomes a binding available to all subsequent stages.

```
pipeline align_and_sort {
  stage alignment {
    let ref = "GRCh38.fa"
    let r1 = "tumor_R1.fastq.gz"
    let r2 = "tumor_R2.fastq.gz"
    tool("bwa-mem2", "mem -t 16 " + ref + " " + r1 + " " + r2 + " -o aligned.bam")
  }

  stage sorted {
    # alignment is the output path from the previous stage
    tool("samtools", "sort -@ 8 -o sorted.bam " + alignment)
  }

  stage indexed {
    tool("samtools", "index " + sorted)
    sorted  # return the sorted BAM path
  }
}
```

Each stage name resolves to whatever its block evaluated to. This creates an
implicit dependency chain without manual wiring.

## Parallel Execution

When you need to process multiple samples, `parallel for` fans out work across
available cores. BioLang schedules iterations concurrently, respecting system
resources.

```
let samples = [
  {id: "tumor_01", r1: "t01_R1.fq.gz", r2: "t01_R2.fq.gz"},
  {id: "tumor_02", r1: "t02_R1.fq.gz", r2: "t02_R2.fq.gz"},
  {id: "tumor_03", r1: "t03_R1.fq.gz", r2: "t03_R2.fq.gz"},
  {id: "normal_01", r1: "n01_R1.fq.gz", r2: "n01_R2.fq.gz"},
]

parallel for sample in samples {
  let bam = tool("bwa-mem2", "mem -t 4 GRCh38.fa " + sample.r1 + " " + sample.r2 + " -o " + sample.id + ".bam")
  let sorted = tool("samtools", "sort -@ 4 -o " + sample.id + ".sorted.bam " + bam)
  tool("samtools", "index " + sorted)
  {id: sample.id, bam: sorted}
}
```

The result is a list of records, one per sample, collected in the order the
iterations complete.

## Stage Dependencies and DAG Execution

BioLang infers a dependency graph from which stage names appear in each block.
Independent stages run concurrently when the runtime detects no data
dependencies.

```
pipeline variant_qc {
  stage aligned {
    let bam = tool("bwa-mem2", "mem -t 16 GRCh38.fa sample_R1.fq.gz sample_R2.fq.gz -o aligned.bam")
    tool("samtools", "sort -@ 8 -o aligned.sorted.bam " + bam)
  }

  # These two stages depend only on aligned, not on each other.
  # BioLang runs them concurrently.
  stage depth {
    tool("samtools", "depth " + aligned) |> mean()
  }

  stage flagstat {
    tool("samtools", "flagstat " + aligned)
  }

  # This stage depends on both depth and flagstat
  stage report {
    {
      mean_depth: depth,
      mapping_rate: flagstat.mapped_pct,
      sample: "sample_01",
    }
  }
}
```

The runtime builds a DAG: `aligned` has no dependencies, `depth` and `flagstat`
both depend on `aligned` (and thus run in parallel once it finishes), and
`report` depends on both.

## Defer for Cleanup

Intermediate files pile up fast. `defer` registers a cleanup action that runs
when the pipeline completes, whether it succeeds or fails. Multiple `defer`
blocks execute in reverse order (LIFO).

```
pipeline trimmed_alignment {
  stage trim {
    let trimmed_r1 = "tmp/trimmed_R1.fq.gz"
    let trimmed_r2 = "tmp/trimmed_R2.fq.gz"
    trim_adapters("sample_R1.fq.gz", "sample_R2.fq.gz",
                  out_r1: trimmed_r1, out_r2: trimmed_r2,
                  adapter: "AGATCGGAAGAG")
    defer { remove(trimmed_r1); remove(trimmed_r2) }
    {r1: trimmed_r1, r2: trimmed_r2}
  }

  stage align {
    let bam = tool("bwa-mem2", "mem -t 16 GRCh38.fa " + trim.r1 + " " + trim.r2 + " -o aligned.bam")
    let unsorted = bam
    let sorted = tool("samtools", "sort -@ 8 -o sorted.bam " + bam)
    defer { remove(unsorted) }
    sorted
  }

  stage mark_dups {
    let deduped = tool("gatk", "MarkDuplicates -I " + align + " -O deduped.bam -M dup_metrics.txt")
    let metrics = align + ".dup_metrics"
    defer { remove(metrics) }
    deduped
  }
}
```

When `trimmed_alignment` finishes, the deferred blocks fire: the duplicate
metrics file is removed first, then the unsorted BAM, then the trimmed FASTQs.

## Example: RNA-seq Processing Pipeline

A complete RNA-seq workflow from raw reads to normalized expression counts.

```
pipeline rnaseq {
  stage qc {
    let report = tool("fastqc", "rnaseq_R1.fq.gz rnaseq_R2.fq.gz -o qc/")
    let stats = read_fastq("rnaseq_R1.fq.gz") |> read_stats()
    defer { log("QC complete: " + str(stats.total_reads) + " reads") }
    {report: report, stats: stats}
  }

  stage trim {
    let trimmed = trim_adapters(
      "rnaseq_R1.fq.gz", "rnaseq_R2.fq.gz",
      out_r1: "trimmed_R1.fq.gz",
      out_r2: "trimmed_R2.fq.gz",
      quality: 20,
      min_length: 36
    )
    defer { remove("trimmed_R1.fq.gz"); remove("trimmed_R2.fq.gz") }
    trimmed
  }

  stage align {
    tool("STAR", "--genomeDir star_index/GRCh38 --readFilesIn trimmed_R1.fq.gz trimmed_R2.fq.gz --outFileNamePrefix aligned/rnaseq_ --runThreadN 16 --twopassMode Basic")
  }

  stage quantify {
    tool("featureCounts", "-a gencode.v44.gtf -o counts.txt -p -s 2 -T 8 " + align)
  }

  stage normalize {
    let raw_counts = tsv(quantify)
    let count_cols = raw_counts
      |> select(exclude: ["gene_id", "gene_name"])
    let medians = count_cols |> each(|col| median(col))
    let size_factors = medians |> map(|m| median(medians) / m)

    raw_counts
      |> mutate(exclude: ["gene_id", "gene_name"],
                |col, i| col * size_factors[i])
      |> write_tsv("normalized_counts.tsv")
  }
}
```

## Example: Variant Calling Pipeline

Germline variant calling from FASTQ to annotated VCF.

```
pipeline germline_variants {
  stage align {
    let ref = "GRCh38.fa"
    let bam = tool("bwa-mem2", "mem -t 16 -R '@RG\\tID:S1\\tSM:sample' " + ref + " sample_R1.fq.gz sample_R2.fq.gz -o aligned.bam")
    let sorted = tool("samtools", "sort -@ 8 -o sorted.bam " + bam)
    tool("samtools", "index " + sorted)
    sorted
  }

  stage mark_dups {
    tool("gatk", "MarkDuplicates -I " + align + " -O deduped.bam -M dup_metrics.txt")
  }

  stage bqsr {
    let known_sites = [
      "dbsnp_156.vcf.gz",
      "Mills_and_1000G.indels.vcf.gz",
    ]
    let table = tool("gatk", "BaseRecalibrator -I " + mark_dups + " -R GRCh38.fa --known-sites " + join(known_sites, " --known-sites ") + " -O recal.table")
    tool("gatk", "ApplyBQSR -I " + mark_dups + " -R GRCh38.fa --bqsr-recal-file " + table + " -O recal.bam")
  }

  stage call {
    tool("gatk", "HaplotypeCaller -I " + bqsr + " -R GRCh38.fa -ERC GVCF -L exome_targets.bed -O sample.g.vcf.gz")
  }

  stage genotype {
    tool("gatk", "GenotypeGVCFs -V " + call + " -R GRCh38.fa -O genotyped.vcf.gz")
  }

  stage filter {
    let snps_raw = tool("gatk", "SelectVariants -V " + genotype + " --select-type-to-include SNP -O snps.vcf.gz")
    let snps = tool("gatk", "VariantFiltration -V " + snps_raw + " --filter-name QD --filter-expression 'QD < 2.0' --filter-name FS --filter-expression 'FS > 60.0' --filter-name MQ --filter-expression 'MQ < 40.0' -O snps.filtered.vcf.gz")

    let indels_raw = tool("gatk", "SelectVariants -V " + genotype + " --select-type-to-include INDEL -O indels.vcf.gz")
    let indels = tool("gatk", "VariantFiltration -V " + indels_raw + " --filter-name QD --filter-expression 'QD < 2.0' --filter-name FS --filter-expression 'FS > 200.0' -O indels.filtered.vcf.gz")

    let merged = tool("bcftools", "concat " + snps + " " + indels + " -o merged.vcf.gz")
    tool("bcftools", "sort " + merged + " -o sorted.vcf.gz")
  }

  stage annotate {
    tool("vep", "--input_file " + filter + " --output_file sample.annotated.vcf.gz --species human --assembly GRCh38 --dir_cache vep_cache/ --plugin CADD --plugin SpliceAI --vcf --compress_output bgzip")
  }
}
```

## Example: Multi-Sample Parallel Processing with Merge

Processing a cohort in parallel, then merging results in a final stage.

```
let cohort = tsv("sample_manifest.tsv")
  |> map(|row| {
       id: row.sample_id,
       r1: row.fastq_r1,
       r2: row.fastq_r2,
       type: row.sample_type,
     })

pipeline cohort_analysis {
  stage per_sample {
    parallel for sample in cohort {
      let raw_bam = tool("bwa-mem2", "mem -t 4 -R '@RG\\tID:" + sample.id + "\\tSM:" + sample.id + "' GRCh38.fa " + sample.r1 + " " + sample.r2 + " -o " + sample.id + ".bam")
      let bam = tool("samtools", "sort -@ 4 -o " + sample.id + ".sorted.bam " + raw_bam)

      tool("samtools", "index " + bam)

      let gvcf = tool("gatk", "HaplotypeCaller -I " + bam + " -R GRCh38.fa -ERC GVCF -O " + sample.id + ".g.vcf.gz")

      let stats = tool("samtools", "flagstat " + bam)
      let depth = tool("samtools", "depth " + bam) |> mean()

      {
        id: sample.id,
        bam: bam,
        gvcf: gvcf,
        mapped_pct: stats.mapped_pct,
        mean_depth: depth,
      }
    }
  }

  stage qc_gate {
    # Fail the pipeline if any sample has poor mapping
    let failed = per_sample
      |> filter(|s| s.mapped_pct < 90.0 or s.mean_depth < 20.0)

    if len(failed) > 0 then
      error("QC failed for: " + join(map(failed, |s| s.id), ", "))

    per_sample
  }

  stage joint_genotype {
    let gvcfs = qc_gate |> map(|s| s.gvcf)
    let gvcf_args = gvcfs |> map(|g| "-V " + g) |> join(" ")
    let combined = tool("gatk", "CombineGVCFs " + gvcf_args + " -R GRCh38.fa -O cohort.g.vcf.gz")
    tool("gatk", "GenotypeGVCFs -V " + combined + " -R GRCh38.fa -O cohort.vcf.gz")
  }

  stage filter_and_annotate {
    let recal = tool("gatk", "VariantRecalibrator -V " + joint_genotype + " -R GRCh38.fa --resource:hapmap hapmap.vcf.gz --resource:omni 1000G_omni.vcf.gz -O cohort.recal --tranches-file cohort.tranches")
    let filtered = tool("gatk", "ApplyVQSR -V " + joint_genotype + " --recal-file cohort.recal --tranches-file cohort.tranches -O cohort.filtered.vcf.gz")
    tool("vep", "--input_file " + filtered + " --output_file cohort.annotated.vcf.gz --species human --assembly GRCh38 --vcf --compress_output bgzip")
  }

  stage summary {
    let variant_counts = read_vcf("cohort.annotated.vcf.gz")
      |> group_by(|v| v.info.consequence)
      |> map(|g| {consequence: g.0, count: len(g.1)})

    let sample_stats = qc_gate
      |> map(|s| {id: s.id, depth: s.mean_depth, mapped: s.mapped_pct})

    {
      n_samples: len(cohort),
      variant_counts: variant_counts,
      sample_stats: sample_stats,
    }
      |> write_json("cohort_summary.json")
  }
}
```

## Parameterized Pipelines (Templates)

A pipeline can accept parameters, turning it into a reusable template. When you
declare `pipeline name(param1, param2) { ... }`, BioLang defines a callable
function instead of executing immediately. You invoke it like any other function.

```
# Define a reusable alignment template
pipeline align_sample(sample_id, r1, r2, reference) {
  stage aligned {
    shell("bwa-mem2 mem -t 16 " + reference + " " + r1 + " " + r2
          + " | samtools sort -@ 8 -o " + sample_id + ".sorted.bam")
    sample_id + ".sorted.bam"
  }

  stage indexed {
    shell("samtools index " + aligned)
    aligned
  }
}

# Call with different inputs — same pipeline, different data
let tumor = align_sample("tumor", "tumor_R1.fq.gz", "tumor_R2.fq.gz", "GRCh38.fa")
let normal = align_sample("normal", "normal_R1.fq.gz", "normal_R2.fq.gz", "GRCh38.fa")
print("Tumor BAM: " + tumor)
print("Normal BAM: " + normal)
```

You can loop over a sample list to process a whole cohort with the same template:

```
let samples = [
  {id: "S1", r1: "S1_R1.fq.gz", r2: "S1_R2.fq.gz"},
  {id: "S2", r1: "S2_R1.fq.gz", r2: "S2_R2.fq.gz"},
  {id: "S3", r1: "S3_R1.fq.gz", r2: "S3_R2.fq.gz"},
]

let bams = samples |> map(|s| align_sample(s.id, s.r1, s.r2, "GRCh38.fa"))
print("Aligned " + str(len(bams)) + " samples")
```

**How it works:**

- `pipeline name { ... }` (no params) — executes immediately, result bound to `name`
- `pipeline name(params) { ... }` — defines a callable function; nothing runs until you call it
- The parameter list uses the same syntax as function parameters

## Pipeline Composition

Pipelines are values. You can call one pipeline from within another or store
its result for downstream use.

```
pipeline align_one(sample) {
  stage bam {
    let raw = tool("bwa-mem2", "mem -t 8 GRCh38.fa " + sample.r1 + " " + sample.r2 + " -o aligned.bam")
    tool("samtools", "sort -@ 4 -o sorted.bam " + raw)
  }
  stage index {
    tool("samtools", "index " + bam)
    bam
  }
}

pipeline somatic_pair(tumor, normal) {
  stage tumor_bam { align_one(tumor) }
  stage normal_bam { align_one(normal) }

  stage call {
    tool("gatk", "Mutect2 -I " + tumor_bam + " -I " + normal_bam + " -R GRCh38.fa --panel-of-normals pon.vcf.gz --germline-resource af-only-gnomad.vcf.gz -O somatic.vcf.gz")
  }

  stage filter {
    tool("gatk", "FilterMutectCalls -V " + call + " -R GRCh38.fa -O somatic.filtered.vcf.gz")
  }
}

# Run the composed pipeline
let result = somatic_pair(
  {r1: "tumor_R1.fq.gz", r2: "tumor_R2.fq.gz"},
  {r1: "normal_R1.fq.gz", r2: "normal_R2.fq.gz"}
)
print("Somatic variants written to: " + result)
```

## Summary

Pipeline blocks give you named, composable workflows with automatic dependency
tracking. Stages pass data forward by name, `parallel for` fans out across
samples, and `defer` keeps your working directory clean. The runtime infers a
DAG from stage references, running independent stages concurrently while
respecting data dependencies.
