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
      |> fastq_stats()
  }

  stage filter {
    read_fastq("sample_R1.fastq.gz")
      |> filter(|r| mean(r.quality) >= 30)
      |> write_fastq("sample_R1.filtered.fastq.gz")
  }

  stage filtered_stats {
    read_fastq("sample_R1.filtered.fastq.gz")
      |> fastq_stats()
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
    bwa_mem(ref, r1, r2, threads: 16)
  }

  stage sorted {
    # alignment is the output path from the previous stage
    samtools_sort(alignment, threads: 8)
  }

  stage indexed {
    samtools_index(sorted)
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
  let bam = bwa_mem("GRCh38.fa", sample.r1, sample.r2, threads: 4)
  let sorted = samtools_sort(bam, threads: 4)
  samtools_index(sorted)
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
    bwa_mem("GRCh38.fa", "sample_R1.fq.gz", "sample_R2.fq.gz")
      |> samtools_sort(threads: 8)
  }

  # These two stages depend only on aligned, not on each other.
  # BioLang runs them concurrently.
  stage depth {
    samtools_depth(aligned) |> mean()
  }

  stage flagstat {
    samtools_flagstat(aligned)
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
    let bam = bwa_mem("GRCh38.fa", trim.r1, trim.r2, threads: 16)
    let unsorted = bam
    let sorted = samtools_sort(bam, threads: 8)
    defer { remove(unsorted) }
    sorted
  }

  stage mark_dups {
    let deduped = gatk_mark_duplicates(align)
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
    let report = fastqc("rnaseq_R1.fq.gz", "rnaseq_R2.fq.gz",
                         out_dir: "qc/")
    let stats = read_fastq("rnaseq_R1.fq.gz") |> fastq_stats()
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
    star_align(
      index: "star_index/GRCh38",
      r1: "trimmed_R1.fq.gz",
      r2: "trimmed_R2.fq.gz",
      out_prefix: "aligned/rnaseq_",
      threads: 16,
      two_pass: true
    )
  }

  stage quantify {
    featurecounts(
      bam: align,
      annotation: "gencode.v44.gtf",
      paired: true,
      strand: "reverse",
      threads: 8
    )
  }

  stage normalize {
    let raw_counts = read_tsv(quantify)
    let size_factors = raw_counts
      |> columns(exclude: ["gene_id", "gene_name"])
      |> each(|col| median(col))
      |> |medians| median(medians) / medians

    raw_counts
      |> map_columns(exclude: ["gene_id", "gene_name"],
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
    bwa_mem(ref, "sample_R1.fq.gz", "sample_R2.fq.gz",
            threads: 16, read_group: "@RG\\tID:S1\\tSM:sample")
      |> samtools_sort(threads: 8)
      |> samtools_index()
  }

  stage mark_dups {
    gatk_mark_duplicates(align)
  }

  stage bqsr {
    let known_sites = [
      "dbsnp_156.vcf.gz",
      "Mills_and_1000G.indels.vcf.gz",
    ]
    gatk_base_recalibrator(mark_dups, ref: "GRCh38.fa",
                           known_sites: known_sites)
      |> gatk_apply_bqsr(mark_dups, ref: "GRCh38.fa")
  }

  stage call {
    gatk_haplotype_caller(bqsr, ref: "GRCh38.fa",
                          emit_ref_confidence: "GVCF",
                          intervals: "exome_targets.bed")
  }

  stage genotype {
    gatk_genotype_gvcfs(call, ref: "GRCh38.fa")
  }

  stage filter {
    let snps = gatk_select_variants(genotype, type: "SNP")
      |> gatk_filter(filters: [
           {name: "QD", expr: "QD < 2.0"},
           {name: "FS", expr: "FS > 60.0"},
           {name: "MQ", expr: "MQ < 40.0"},
         ])

    let indels = gatk_select_variants(genotype, type: "INDEL")
      |> gatk_filter(filters: [
           {name: "QD", expr: "QD < 2.0"},
           {name: "FS", expr: "FS > 200.0"},
         ])

    bcftools_concat(snps, indels) |> bcftools_sort()
  }

  stage annotate {
    ensembl_vep(filter,
                species: "human",
                assembly: "GRCh38",
                cache_dir: "vep_cache/",
                plugins: ["CADD", "SpliceAI"])
      |> write("sample.annotated.vcf.gz")
  }
}
```

## Example: Multi-Sample Parallel Processing with Merge

Processing a cohort in parallel, then merging results in a final stage.

```
let cohort = read_tsv("sample_manifest.tsv")
  |> map(|row| {
       id: row.sample_id,
       r1: row.fastq_r1,
       r2: row.fastq_r2,
       type: row.sample_type,
     })

pipeline cohort_analysis {
  stage per_sample {
    parallel for sample in cohort {
      let bam = bwa_mem("GRCh38.fa", sample.r1, sample.r2,
                        threads: 4,
                        read_group: "@RG\\tID:" + sample.id + "\\tSM:" + sample.id)
        |> samtools_sort(threads: 4)

      samtools_index(bam)

      let gvcf = gatk_haplotype_caller(bam, ref: "GRCh38.fa",
                                        emit_ref_confidence: "GVCF")

      let stats = samtools_flagstat(bam)
      let depth = samtools_depth(bam) |> mean()

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

    if length(failed) > 0 then
      error("QC failed for: " + join(map(failed, |s| s.id), ", "))

    per_sample
  }

  stage joint_genotype {
    let gvcfs = qc_gate |> map(|s| s.gvcf)
    gatk_combine_gvcfs(gvcfs, ref: "GRCh38.fa")
      |> gatk_genotype_gvcfs(ref: "GRCh38.fa")
  }

  stage filter_and_annotate {
    joint_genotype
      |> gatk_vqsr(ref: "GRCh38.fa",
                   resources: ["hapmap.vcf.gz", "1000G_omni.vcf.gz"])
      |> ensembl_vep(species: "human", assembly: "GRCh38")
      |> write("cohort.annotated.vcf.gz")
  }

  stage summary {
    let variant_counts = read_vcf("cohort.annotated.vcf.gz")
      |> group_by(|v| v.info.consequence)
      |> map_values(|vs| length(vs))

    let sample_stats = qc_gate
      |> map(|s| {id: s.id, depth: s.mean_depth, mapped: s.mapped_pct})

    {
      n_samples: length(cohort),
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
    bwa_mem("GRCh38.fa", sample.r1, sample.r2, threads: 8)
      |> samtools_sort(threads: 4)
  }
  stage index {
    samtools_index(bam)
    bam
  }
}

pipeline somatic_pair(tumor, normal) {
  stage tumor_bam { align_one(tumor) }
  stage normal_bam { align_one(normal) }

  stage call {
    mutect2(tumor: tumor_bam, normal: normal_bam,
            ref: "GRCh38.fa",
            pon: "pon.vcf.gz",
            germline: "af-only-gnomad.vcf.gz")
  }

  stage filter {
    gatk_filter_mutect(call, ref: "GRCh38.fa")
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
