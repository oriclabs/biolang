# Control Flow

BioLang provides a rich set of control-flow constructs designed for the messy
realities of biological data. Variants need classification, samples need
multi-criteria gating, and searches over sequences must handle both the
found and not-found cases gracefully.

## if/else Expressions

`if/else` in BioLang is an expression -- it returns a value. This lets you
embed conditional logic directly in assignments and pipes.

```biolang
let label = if gc_content(seq) > 0.6 {
    "GC-rich"
} else if gc_content(seq) < 0.4 {
    "AT-rich"
} else {
    "balanced"
}
```

Because it is an expression, you can use it inline:

```biolang
let allele_freq = if total_depth > 0 { alt_count / total_depth } else { 0.0 }
```

## match Expressions

`match` compares a value against a series of patterns. Like `if`, it is an
expression and returns the result of the matching arm.

```biolang
let codon = "ATG"

let amino_acid = match codon {
    "ATG" => "Met"
    "TGG" => "Trp"
    "TAA" | "TAG" | "TGA" => "Stop"
    _ => codon_usage(codon)
}
```

Patterns can destructure records:

```biolang
fn describe_alignment(aln) {
    match aln {
        {mapped: true, mapq: q} if q >= 30 => "high-confidence"
        {mapped: true, mapq: q} if q >= 10 => "low-confidence"
        {mapped: true}                     => "very-low-quality"
        {mapped: false}                    => "unmapped"
    }
}
```

## for Loops

The basic `for` loop iterates over any collection or stream.

```biolang
let reads = read_fastq("sample.fastq.gz")
let total_bases = 0

for read in reads {
    total_bases = total_bases + len(read.seq)
}

print("Total bases: " + str(total_bases))
```

### Tuple Destructuring

When iterating over paired data, destructure directly in the loop header.

```biolang
let sample_ids = ["S1", "S2", "S3"]
let fastq_paths = [
    "data/S1_R1.fastq.gz",
    "data/S2_R1.fastq.gz",
    "data/S3_R1.fastq.gz"
]

for (sample, path) in zip(sample_ids, fastq_paths) {
    let stats = read_fastq(path) |> read_stats()
    print(sample + ": " + str(stats.total_reads) + " reads")
}
```

Destructuring also works with `enumerate`:

```biolang
let exons = read_bed("BRCA1_exons.bed")

for (i, exon) in enumerate(exons) {
    print("Exon " + str(i + 1) + ": " + exon.chrom + ":" + str(exon.start) + "-" + str(exon.end))
}
```

### for/else

The `else` block executes only when the loop completes without hitting a
`break`. This is perfect for search-and-report patterns.

```biolang
# requires: internet connection
let target = dna"TATAAA"
let promoters = read_bed("promoter_regions.bed")

for region in promoters {
    let seq = ucsc_sequence("hg38", region.chrom, region.start, region.end)
    if contains(seq, target) {
        print("TATA box found in " + region.name)
        break
    }
} else {
    print("No TATA box found in any promoter region")
}
```

## while Loops

Use `while` when the number of iterations is unknown ahead of time.

```biolang
fn find_orfs_manual(seq) {
    let i = 0
    let in_orf = false
    let orf_start = 0
    let orfs = []

    while i + 2 < len(seq) {
        let codon = slice(seq, i, i + 3)
        if codon == "ATG" && !in_orf {
            in_orf = true
            orf_start = i
        }
        if (codon == "TAA" || codon == "TAG" || codon == "TGA") && in_orf {
            orfs = orfs + [{start: orf_start, end: i + 3, length: i + 3 - orf_start}]
            in_orf = false
        }
        i = i + 3
    }
    orfs
}

let orfs = find_orfs_manual("ATGAAACCCTAGATGTTTGAATAA")
# [{start: 0, end: 12, length: 12}, {start: 12, end: 24, length: 12}]
```

## break and continue

`break` exits the innermost loop. `continue` skips to the next iteration.

```biolang
let variants = read_vcf("somatic.vcf")
let first_pathogenic = nil

for v in variants {
    # Skip low-quality calls
    if v.qual < 30.0 {
        continue
    }

    let info = parse_info(v.info_str)
    if info?.CLNSIG == "Pathogenic" {
        first_pathogenic = v
        break
    }
}
```

## given/otherwise

`given/otherwise` is a declarative chain of conditions. It reads top to bottom;
the first true condition wins. `otherwise` is the fallback.

```biolang
fn classify_read_pair(r1_len, r2_len, insert_size) {
    given {
        insert_size < 0         => "invalid_pair"
        insert_size > 1000      => "structural_variant_candidate"
        r1_len < 36 || r2_len < 36 => "short_read"
        insert_size < 150       => "short_insert"
        otherwise               => "normal"
    }
}
```

`given` is an expression, so you can assign its result:

```biolang
let risk = given {
    allele_freq >= 0.5 && coverage >= 30 => "high_confidence_somatic"
    allele_freq >= 0.2                   => "moderate_evidence"
    allele_freq >= 0.05                  => "low_frequency"
    otherwise                            => "below_detection"
}
```

## guard Clauses

`guard` asserts that a condition is true. If it is not, the `else` block
executes -- typically an early return or error.

```biolang
fn calculate_tmb(variants, exome_size_mb) {
    guard exome_size_mb > 0 else {
        return {error: "Exome size must be positive"}
    }
    guard len(variants) > 0 else {
        return {tmb: 0.0, classification: "low"}
    }

    let somatic = variants |> filter(|v| v.filter == "PASS")
    let tmb = len(somatic) / exome_size_mb

    let classification = given {
        tmb >= 20.0  => "high"
        tmb >= 10.0  => "intermediate"
        otherwise    => "low"
    }

    {tmb: tmb, classification: classification}
}
```

Guards keep the main logic at the top indentation level by pushing error
handling to the margin. Use them liberally for input validation.

## unless

`unless` is syntactic sugar for `if !condition`. It reads naturally for
negative checks.

```biolang
fn process_bam(path) {
    let header = sam_header(path)

    unless contains(header.sort_order, "coordinate") {
        print("WARNING: BAM is not coordinate-sorted, sorting first")
        shell("samtools sort -o " + path + " " + path)
    }

    unless file_exists(path + ".bai") {
        print("Indexing BAM")
        shell("samtools index " + path)
    }

    # Proceed with analysis
    let stats = flagstat(path)
    stats
}
```

## Example: Classify Variants by Type

Read a VCF and produce a summary table of variant types using `match`.

```biolang
let variants = read_vcf("sample.vcf")

let classified = variants |> map(|v| {
    let vtype = match true {
        is_snp(v) && is_transition(v)   => "transition"
        is_snp(v) && is_transversion(v) => "transversion"
        is_snp(v)                       => "snp_other"
        is_indel(v) && len(v.alt) > len(v.ref) => "insertion"
        is_indel(v)                     => "deletion"
        _                               => "complex"
    }
    {...v, classification: vtype}
})

let summary = classified
    |> group_by("classification")
    |> map(|group| {type: group.0, count: len(group.1)})
    |> sort(|a, b| b.count - a.count)

for row in summary {
    print(row.type + ": " + str(row.count))
}
# transition: 45231
# transversion: 22890
# deletion: 3412
# insertion: 2876
# complex: 134
```

## Example: Search for Motif with for/else

Scan upstream regions for a transcription-factor binding motif. Report the
first hit or state that none was found.

```biolang
# requires: internet connection
let motif = dna"CANNTG"  # E-box motif (N = any base)
let genes = read_gff("annotations.gff")
    |> filter(|f| f.type == "gene" && f.biotype == "protein_coding")

for gene in genes {
    let upstream = interval(gene.chrom, gene.start - 2000, gene.start)
    let seq = ucsc_sequence("hg38", gene.chrom, gene.start - 2000, gene.start)
    let hits = find_motif(seq, motif)

    if len(hits) > 0 {
        print("E-box found upstream of " + gene.name + " at offset " + str(hits[0].position))
        break
    }
} else {
    print("No E-box motif found in any upstream region scanned")
}
```

## Example: Multi-Criteria Sample QC with given/otherwise

Gate samples through a series of quality thresholds and assign a disposition.

```biolang
fn qc_disposition(stats) {
    given {
        stats.total_reads < 1_000_000 =>
            {pass: false, reason: "insufficient_reads", reads: stats.total_reads}

        stats.pct_mapped < 70.0 =>
            {pass: false, reason: "low_mapping_rate", pct: stats.pct_mapped}

        stats.mean_depth < 10.0 =>
            {pass: false, reason: "low_depth", depth: stats.mean_depth}

        stats.pct_duplicate > 40.0 =>
            {pass: false, reason: "high_duplication", pct: stats.pct_duplicate}

        stats.contamination > 0.03 =>
            {pass: false, reason: "contamination", frac: stats.contamination}

        otherwise =>
            {pass: true, reason: "all_checks_passed"}
    }
}

let samples = ["S001", "S002", "S003", "S004"]
let bam_dir = "data/aligned"

for sample in samples {
    let stats = flagstat(bam_dir + "/" + sample + ".bam")
    let result = qc_disposition(stats)

    if result.pass {
        print(sample + ": PASS")
    } else {
        print(sample + ": FAIL (" + result.reason + ")")
    }
}
```

## Example: Input Validation with guard

A variant-calling wrapper that validates every precondition before running
the expensive computation.

```biolang
fn call_variants(bam_path, ref_path, bed_path, min_depth: 10) {
    guard file_exists(bam_path) else {
        return {error: "BAM file not found: " + bam_path}
    }
    guard file_exists(ref_path) else {
        return {error: "Reference FASTA not found: " + ref_path}
    }
    guard file_exists(bed_path) else {
        return {error: "Target BED not found: " + bed_path}
    }

    let header = sam_header(bam_path)
    guard header.sort_order == "coordinate" else {
        return {error: "BAM must be coordinate-sorted"}
    }
    guard file_exists(bam_path + ".bai") else {
        return {error: "BAM index (.bai) not found"}
    }

    # All preconditions met -- run the caller
    let regions = read_bed(bed_path)
    let vcf_out = "calls.vcf"
    shell("bcftools mpileup -f " + ref_path + " -R " + bed_path + " " + bam_path
        + " | bcftools call -mv -Ov -o " + vcf_out)

    let raw_calls = read_vcf(vcf_out)
    let filtered = raw_calls
        |> filter(|v| v.qual >= 30.0)

    {
        total_calls: len(raw_calls),
        passing_calls: len(filtered),
        variants: filtered
    }
}
```

## Summary

| Construct | Returns Value? | Best For |
|---|---|---|
| `if/else` | Yes | Binary or ternary decisions |
| `match` | Yes | Multi-arm pattern dispatch |
| `for` | No | Iteration over collections |
| `for/else` | No | Search with not-found fallback |
| `while` | No | Indeterminate iteration |
| `given/otherwise` | Yes | Declarative condition chains |
| `guard ... else` | No | Early-exit preconditions |
| `unless` | No | Negative-condition readability |

Choose the construct that best communicates intent. Use `guard` for validation
at function boundaries, `given` for multi-criteria classification, `match` for
type-driven dispatch, and `for/else` when a search must report failure.
