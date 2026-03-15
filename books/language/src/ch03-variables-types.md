# Chapter 3: Variables and Types

BioLang has a rich type system designed around bioinformatics data. This chapter
covers variable bindings, the full set of types, records, type checking and coercion,
and nil handling.

## `let` Bindings

Variables are introduced with `let` and are immutable by default:

```biolang
let sample_id = "TCGA-A1-A0SP"
let min_depth = 30
let reference = dna"ATCGATCGATCG"
```

Attempting to rebind a `let` variable in the same scope is an error:

```biolang
let threshold = 0.05
let threshold = 0.01   # Error: 'threshold' is already bound in this scope
```

## Reassignment

Use `=` without `let` to reassign an existing variable:

```biolang
let total_reads = 0
total_reads = total_reads + 1847293

let status = "pending"
status = "complete"
```

## Primitive Types

| Type | Examples | Description |
|------|----------|-------------|
| `Int` | `42`, `-1`, `1_000_000` | 64-bit integer |
| `Float` | `3.14`, `1e-10`, `0.05` | 64-bit float |
| `Str` | `"hello"`, `f"GC: {gc}"`, `r"C:\data"` | UTF-8 string |
| `Bool` | `true`, `false` | Boolean |
| `Nil` | `nil` | Absence of value |

```biolang
let num_samples = 48
let p_value = 3.2e-8
let experiment = "RNA-seq time course"
let is_paired_end = true
let adapter_seq = nil   # not yet determined

# String variants
let name = "sample_01"                  # regular string (\n = newline, \t = tab)
let msg = f"Found {num_samples} reads"  # f-string with interpolation
let path = r"C:\Users\data\reads.fq"    # raw string (backslashes literal)
```

## Bio Types

These types carry domain semantics beyond raw data:

| Type | Literal | Description |
|------|---------|-------------|
| `DNA` | `dna"ATCG"` | DNA sequence (IUPAC) |
| `RNA` | `rna"AUGC"` | RNA sequence |
| `Protein` | `protein"MVLK"` | Amino acid sequence |
| `Quality` | `qual"IIIHH"` | Phred+33 quality scores |
| `Interval` | `interval("chr1", 1000, 2000)` | Genomic interval |
| `Variant` | `variant("chr17", 7674220, "C", "T")` | Sequence variant |

```biolang
let primer_fwd = dna"TGTAAAACGACGGCCAGT"
let stop_codon = rna"UAG"
let signal_peptide = protein"MKWVTFISLLFLFSSAYS"
let read_quals = qual"IIIIIIIIHHHHHGGGGFFFF"

# Genomic coordinates
let tp53_exon1 = interval("chr17", 7668421, 7669690)
let brca1_snp = variant("chr17", 43094464, "G", "A")
```

### Interval Type

Intervals represent genomic regions with chromosome, start, and end:

```biolang
let region = interval("chr1", 1000000, 2000000)
print(region.chrom)   # => "chr1"
print(region.start)   # => 1000000
print(region.end)     # => 2000000
print(region.length)  # => 1000000

# Interval arithmetic
let exon1 = interval("chr1", 1000, 1500)
let exon2 = interval("chr1", 1400, 2000)
print(exon1.end > exon2.start && exon2.end > exon1.start)  # => true
print(intersect(exon1, exon2))      # => interval("chr1", 1400, 1500)
```

### Variant Type

Variants represent sequence changes at specific positions:

```biolang
let snv = variant("chr7", 55249071, "C", "T")
print(snv.chrom)    # => "chr7"
print(snv.pos)      # => 55249071
print(snv.ref_allele)  # => "C"
print(snv.alt_allele)  # => "T"

# Classify variant type
print(variant_type(snv))   # => "SNV"

let indel = variant("chr7", 55249071, "CT", "C")
print(variant_type(indel)) # => "DEL"
```

## Records

Records are BioLang's primary structured data type. They map naturally to the
tabular, metadata-rich world of bioinformatics:

```biolang
let sample = {
  sample_id: "S001",
  patient: "P-4821",
  tissue: "tumor",
  reads: 45_000_000,
  mean_coverage: 32.5,
  contamination: 0.02
}

print(sample.sample_id)       # => "S001"
print(sample.mean_coverage)   # => 32.5
```

### String Keys

Record keys can be identifiers or strings:

```biolang
let annotation = {
  "gene_name": "EGFR",
  "Gene Ontology": "GO:0005524",
  chrom: "chr7"
}

print(annotation."Gene Ontology")   # => "GO:0005524"
```

### Record Spread

The spread operator `...` merges one record into another. Later keys override
earlier ones:

```biolang
let defaults = {
  aligner: "bwa-mem2",
  min_mapq: 30,
  mark_duplicates: true,
  reference: "GRCh38"
}

let sample_config = {
  ...defaults,
  sample_id: "S001",
  min_mapq: 20   # override the default
}

print(sample_config.aligner)   # => "bwa-mem2"
print(sample_config.min_mapq)  # => 20
```

This pattern is essential for bioinformatics pipelines where you have standard
parameters with per-sample overrides.

### Nested Records

Records can nest to arbitrary depth:

```biolang
let variant_annotation = {
  variant: variant("chr17", 7674220, "C", "T"),
  gene: {
    name: "TP53",
    transcript: "NM_000546.6",
    exon: 7,
    consequence: "missense_variant"
  },
  population: {
    gnomad_af: 0.00003,
    clinvar: "Pathogenic",
    cosmic_count: 4521
  },
  prediction: {
    sift: {score: 0.001, label: "deleterious"},
    polyphen: {score: 0.998, label: "probably_damaging"}
  }
}

print(variant_annotation.gene.consequence)
print(variant_annotation.prediction.sift.score)
```

## Type Checking

BioLang provides introspection functions for runtime type checking:

```biolang
let seq = dna"ATCG"
print(type(seq))       # => "DNA"
print(is_dna(seq))     # => true
print(is_rna(seq))     # => false

let rec = {name: "BRCA1", chrom: "chr17"}
print(type(rec))       # => "Record"
print(is_record(rec))  # => true
print(is_str(rec))     # => false
```

Available type check functions:

```biolang
is_int(val)
is_float(val)
is_str(val)
is_bool(val)
is_nil(val)
is_dna(val)
is_rna(val)
is_protein(val)
is_quality(val)
is_list(val)
is_set(val)
is_record(val)
is_table(val)
is_interval(val)
is_variant(val)
```

## Type Coercion with `into`

`into(value, "TargetType")` converts between compatible types:

```biolang
# String to DNA
let seq = into("ATCGATCG", "DNA")

# DNA to RNA (transcription)
let mrna = into(dna"ATCGATCG", "RNA")

# Int to Float
let ratio = into(42, "Float")

# String to Int
let depth = into("30", "Int")

# List of records to Table
let rows = [{gene: "TP53", pval: 0.001}, {gene: "BRCA1", pval: 0.05}]
let tbl = into(rows, "Table")

# Variant to Interval (single-base or span)
let region = into(variant("chr1", 1000, "A", "T"), "Interval")
```

## Type Aliases

Define aliases to make code self-documenting:

```biolang
type Locus = Record
type SampleMeta = Record
type QCMetrics = Record

let build_locus = |chrom, start, end, gene| {
  chrom: chrom, start: start, end: end, gene: gene
}

let tp53 = build_locus("chr17", 7668421, 7687490, "TP53")
```

## Nil Handling

Bioinformatics data is full of missing values -- absent annotations, failed QC
metrics, unmapped reads. BioLang has two operators for nil handling.

### `??` Null Coalesce

Returns the left side if non-nil, otherwise the right:

```biolang
let depth = sample.coverage ?? 0
let gene_name = annotation.symbol ?? annotation.id ?? "unknown"

# Useful for setting defaults in pipeline parameters
let min_qual = args.min_quality ?? 30
let threads = args.threads ?? 4
```

### `?.` Optional Chain

Safely accesses nested fields, returning nil if any intermediate value is nil:

```biolang
let clinvar_status = variant_record?.annotation?.clinvar?.significance
# Returns nil if annotation, clinvar, or significance is missing

# Combine with ??
let pathogenicity = variant_record?.annotation?.clinvar?.significance ?? "Unknown"
```

## Example: Building a Sample Metadata Registry

```biolang
# sample_registry.bl
# Parse a sample sheet and build a typed metadata registry.

let defaults = {
  platform: "Illumina",
  chemistry: "NovaSeq 6000",
  read_length: 150,
  paired: true,
  reference: "GRCh38"
}

let raw_sheet = csv("sample_sheet.csv")

let registry = raw_sheet |> map(|row| {
  ...defaults,
  sample_id: row.Sample_ID,
  patient_id: row.Patient ?? "unknown",
  tissue: row.Tissue,
  condition: row.Condition ?? "normal",
  fastq_r1: row.FASTQ_R1,
  fastq_r2: row.FASTQ_R2 ?? nil,
  paired: !is_nil(row.FASTQ_R2),
  library_prep: row.Library ?? "WGS",
  date: row.Date
})

# Validate: every sample must have R1
let invalid = registry |> filter(|s| is_nil(s.fastq_r1))
if len(invalid) > 0 then {
  print(f"ERROR: {len(invalid)} samples missing FASTQ R1:")
  invalid |> each(|s| print(f"  {s.sample_id}"))
  exit(1)
}

# Group by condition for downstream analysis
let by_condition = registry |> group_by("condition")
by_condition |> each(|group| {
  print(f"{group.key}: {len(group.values)} samples")
  group.values |> each(|s| print(f"  {s.sample_id} ({s.tissue})"))
})

# Summary
let tumor_count = registry |> filter(|s| s.condition == "tumor") |> len()
let normal_count = registry |> filter(|s| s.condition == "normal") |> len()
print(f"\nRegistry: {len(registry)} samples ({tumor_count} tumor, {normal_count} normal)")
```

## Example: Parsing and Validating Variant Records

```biolang
# validate_variants.bl
# Read a VCF file, parse variants into typed records, validate, classify.

let raw_variants = read_vcf("data/variants.vcf")

# Build typed variant records with full annotation
let typed_variants = raw_variants |> map(|v| {
  var: variant(v.chrom, v.pos, v.ref, v.alt),
  qual: into(v.qual ?? "0", "Float"),
  depth: into(v.info?.DP ?? "0", "Int"),
  allele_freq: into(v.info?.AF ?? "0.0", "Float"),
  genotype: v.samples?.GT ?? "./.",
  filter_status: v.filter
})

# Validate variant quality
let validate = |v| {
  let issues = []
  if v.qual < 30.0 then issues = issues ++ ["low_qual"]
  if v.depth < 10 then issues = issues ++ ["low_depth"]
  if v.allele_freq < 0.05 then issues = issues ++ ["low_af"]
  if v.genotype == "./." then issues = issues ++ ["missing_gt"]
  {...v, issues: issues, is_valid: len(issues) == 0}
}

let validated = typed_variants |> map(validate)

# Classify variants
let classify = |v| {
  let vtype = variant_type(v.var)
  let size = if vtype == "SNV" then 1
    else abs(len(v.var.alt_allele) - len(v.var.ref_allele))
  {...v, variant_type: vtype, size: size}
}

let classified = validated |> map(classify)

# Report summary
let total = len(classified)
let valid = classified |> filter(|v| v.is_valid) |> len()
let by_type = classified |> group_by("variant_type")

print(f"Total variants: {total}")
print(f"Passing filters: {valid} ({valid / total * 100.0:.1f}%)")
print(f"\nBy type:")
by_type |> each(|g| print(f"  {g.key}: {len(g.values)}"))

# Flag high-impact variants
let high_impact = classified
  |> filter(|v| v.is_valid && v.variant_type != "SNV" && v.size > 50)
  |> sort("size", descending: true)

print(f"\nHigh-impact structural variants (>50bp): {len(high_impact)}")
high_impact |> take(10) |> each(|v| {
  print(f"  {v.var.chrom}:{v.var.pos} {v.variant_type} size={v.size} AF={v.allele_freq:.3f}")
})
```

## Summary

BioLang's type system combines standard programming types with biological data types
and record-based data modeling. Records with spread make it natural to build layered
configurations and annotated data structures. Nil handling with `??` and `?.` keeps
pipelines resilient to the missing data that pervades genomics workflows.

The next chapter covers collections and tables -- the workhorses for processing the
large tabular datasets that dominate bioinformatics.
