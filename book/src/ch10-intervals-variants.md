# Genomic Intervals and Variants

Two data types sit at the heart of genomic analysis: intervals (regions on a
chromosome) and variants (differences from a reference). BioLang provides
built-in types and operations for both, including interval trees for fast
overlap queries and variant classification functions that handle the full
spectrum from SNPs to complex structural events.

## Genomic Intervals

An interval represents a contiguous region on a chromosome. Create one with
the `interval()` constructor.

```biolang
let promoter = interval("chr1", 11869, 14409)
let exon = interval("chr1", 11869, 12227, strand: "+")
```

### Interval Fields

Every interval has `chrom`, `start`, and `end`. Optional fields include
`strand`, `name`, and `score`.

```biolang
let region = interval("chr7", 55181000, 55182000, strand: "+", name: "EGFR_exon20")

print(region.chrom)   # chr7
print(region.start)   # 55181000
print(region.end)     # 55182000
print(region.strand)  # +
print(region.name)    # EGFR_exon20

let width = region.end - region.start
print("Width: " + str(width) + " bp")
```

BioLang uses 0-based, half-open coordinates (BED convention) throughout.

### Interval Arithmetic

Test relationships between intervals directly.

```biolang
let a = interval("chr1", 100, 200)
let b = interval("chr1", 150, 250)
let c = interval("chr1", 300, 400)

# Overlap test
print(overlaps(a, b))   # true
print(overlaps(a, c))   # false

# Containment
let outer = interval("chr1", 100, 500)
print(contains(outer, c))  # true

# Distance between non-overlapping intervals
print(distance(a, c))   # 100

# Merge overlapping intervals into one
let merged = merge_interval(a, b)
# interval("chr1", 100, 250)

# Intersection
let common = intersect(a, b)
# interval("chr1", 150, 200)
```

## Interval Trees

When you have thousands of intervals and need to query overlaps repeatedly,
build an interval tree. Construction is O(n log n); each query is O(log n + k)
where k is the number of hits.

```biolang
let exons = read_bed("gencode_exons.bed")
    |> map(|e| interval(e.chrom, e.start, e.end, name: e.name))

let tree = interval_tree(exons)
```

### query_overlaps

Find all intervals in the tree that overlap a query region.

```biolang
let query = interval("chr17", 43044294, 43044295)  # single position in BRCA1
let hits = query_overlaps(tree, query)

for hit in hits {
    print("Overlaps: " + hit.name + " " + str(hit.start) + "-" + str(hit.end))
}
```

### query_nearest

Find the closest interval to a query, even if there is no overlap.

```biolang
let snp_pos = interval("chr7", 55181378, 55181379)
let nearest = query_nearest(tree, snp_pos)

print("Nearest feature: " + nearest.name
    + " at distance " + str(distance(snp_pos, nearest)) + " bp")
```

### coverage

Compute per-base coverage across a set of intervals -- how many intervals
cover each position.

```biolang
let reads_as_intervals = aligned_reads
    |> map(|r| interval(r.chrom, r.start, r.end))

let tree = interval_tree(reads_as_intervals)
let target = interval("chr17", 43044000, 43045000)

let cov = coverage(tree, target)
print("Mean depth over target: " + str(mean(cov)))
print("Bases >= 30x: " + str(cov |> filter(|d| d >= 30) |> len()))
```

## Variants

A variant represents a difference from the reference genome. Create one with
the `variant()` constructor.

```biolang
let snp = variant("chr7", 55181378, "T", "A")           # EGFR T790M
let del = variant("chr17", 43045684, "TCAA", "T")       # BRCA1 deletion
let ins = variant("chr2", 29416089, "A", "AGCTGCTG")    # ALK insertion
```

### Variant Classification

BioLang provides functions to classify variants by their molecular type.

```biolang
let v = variant("chr7", 55181378, "T", "A")

print(is_snp(v))           # true
print(is_indel(v))         # false
print(is_transition(v))    # false  (T>A is transversion)
print(is_transversion(v))  # true
print(variant_type(v))     # "Snp"
```

The full classification set:

```biolang
let examples = [
    variant("chr1", 100, "A", "G"),      # transition (purine to purine)
    variant("chr1", 200, "A", "T"),      # transversion
    variant("chr1", 300, "ACG", "A"),    # deletion
    variant("chr1", 400, "A", "ATCG"),   # insertion
    variant("chr1", 500, "ACG", "TGA"),  # MNP (multi-nucleotide polymorphism)
]

for v in examples {
    let vtype = variant_type(v)
    let detail = given {
        is_snp(v) && is_transition(v)   => "transition"
        is_snp(v) && is_transversion(v) => "transversion"
        is_indel(v)                     => "indel (" + str(abs(len(v.alt) - len(v.ref))) + "bp)"
        otherwise                       => vtype
    }
    print(v.chrom + ":" + str(v.pos) + " " + v.ref + ">" + v.alt + " => " + detail)
}
```

### Genotype Queries

When a variant carries genotype information (from a VCF), query zygosity.

```biolang
let variants = read_vcf("sample.vcf")

let het_snps = variants
    |> filter(|v| is_snp(v) && is_het(v))

let hom_alt = variants
    |> filter(|v| is_hom_alt(v))

print("Heterozygous SNPs: " + str(len(het_snps)))
print("Homozygous alt calls: " + str(len(hom_alt)))

# Allele balance for heterozygous calls
let allele_balances = het_snps |> map(|v| {
    let info = parse_info(v.info_str)
    let ad = info?.AD ?? [0, 0]
    let total = ad |> reduce(0, |a, b| a + b)
    if total > 0 { ad[1] / total } else { 0.0 }
})

print("Median allele balance (het): " + str(round(median(allele_balances), 3)))
```

### parse_vcf_info

The VCF INFO column packs key-value pairs into a semicolon-delimited string.
`parse_vcf_info()` turns it into a record.

```biolang
let info_str = "DP=42;MQ=60.0;AF=0.48;CLNSIG=Pathogenic;GENEINFO=BRCA1:672"
let info = parse_info(info_str)

print(info.DP)       # 42
print(info.AF)       # 0.48
print(info.CLNSIG)   # "Pathogenic"
print(info.GENEINFO) # "BRCA1:672"
```

## Example: Find Genes Overlapping with ChIP-seq Peaks

Identify which genes have promoter regions that overlap with transcription
factor binding peaks from a ChIP-seq experiment.

```biolang
# Load gene annotations
let genes = read_gff("gencode.v44.annotation.gff3")
    |> filter(|f| f.type == "gene" && f.attributes.gene_type == "protein_coding")

# Define promoter regions: 2kb upstream of TSS
let promoters = genes |> map(|g| {
    let tss = if g.strand == "+" { g.start } else { g.end }
    let prom_start = if g.strand == "+" { tss - 2000 } else { tss }
    let prom_end = if g.strand == "+" { tss } else { tss + 2000 }
    interval(g.chrom, max(0, prom_start), prom_end,
             name: g.attributes.gene_name)
})

# Load ChIP-seq peaks
let peaks = read_bed("H3K27ac_peaks.bed")
    |> map(|p| interval(p.chrom, p.start, p.end, name: p.name ?? "peak"))

# Build interval tree from promoters for fast lookup
let promoter_tree = interval_tree(promoters)

# Query each peak against promoters
let genes_with_peaks = []

for peak in peaks {
    let hits = query_overlaps(promoter_tree, peak)
    for hit in hits {
        genes_with_peaks = genes_with_peaks + [{
            gene: hit.name,
            peak_chrom: peak.chrom,
            peak_start: peak.start,
            peak_end: peak.end,
            overlap_bp: intersect(peak, hit) |> width()
        }]
    }
}

# Deduplicate by gene name
let unique_genes = genes_with_peaks
    |> map(|g| g.gene)
    |> unique()

print("ChIP-seq peaks: " + str(len(peaks)))
print("Genes with H3K27ac at promoter: " + str(len(unique_genes)))

# Top genes by peak overlap size
let top = genes_with_peaks
    |> sort(|a, b| b.overlap_bp - a.overlap_bp)
    |> take(20)

for entry in top {
    print(entry.gene + ": " + str(entry.overlap_bp) + " bp overlap at "
        + entry.peak_chrom + ":" + str(entry.peak_start) + "-" + str(entry.peak_end))
}

write_csv(genes_with_peaks, "genes_with_h3k27ac_peaks.csv")
```

## Example: Classify Variants and Generate a Ti/Tv Report

Transition/transversion ratio (Ti/Tv) is a key quality metric for SNP calling.
Whole-genome sequencing typically yields Ti/Tv around 2.0-2.1; exome around
2.8-3.0. Deviations suggest systematic errors.

```biolang
let variants = read_vcf("deepvariant_output.vcf")

# Keep only PASS SNPs
let pass_snps = variants
    |> filter(|v| v.filter == "PASS" && is_snp(v))

# Classify each SNP
let classified = pass_snps |> map(|v| {
    let cls = if is_transition(v) { "Ti" } else { "Tv" }
    let change = v.ref + ">" + v.alt
    {chrom: v.chrom, pos: v.pos, change: change, class: cls, qual: v.qual}
})

let ti_count = classified |> filter(|v| v.class == "Ti") |> len()
let tv_count = classified |> filter(|v| v.class == "Tv") |> len()
let ratio = if tv_count > 0 { ti_count / tv_count } else { 0.0 }

print("Transitions: " + str(ti_count))
print("Transversions: " + str(tv_count))
print("Ti/Tv ratio: " + str(round(ratio, 3)))

# Per-chromosome Ti/Tv
let by_chrom = classified |> group_by(|v| v.chrom)

let chrom_report = by_chrom |> map(|group| {
    let chrom = group.0
    let vars = group.1
    let ti = vars |> filter(|v| v.class == "Ti") |> len()
    let tv = vars |> filter(|v| v.class == "Tv") |> len()
    {
        chrom: chrom,
        ti: ti,
        tv: tv,
        ratio: if tv > 0 { round(ti / tv, 3) } else { 0.0 },
        total: ti + tv
    }
})

let sorted_report = chrom_report |> sort(|a, b| compare(a.chrom, b.chrom))

print("\nPer-chromosome Ti/Tv:")
print("Chrom\tTi\tTv\tRatio\tTotal")
for row in sorted_report {
    print(row.chrom + "\t" + str(row.ti) + "\t" + str(row.tv)
        + "\t" + str(row.ratio) + "\t" + str(row.total))
}

# Substitution spectrum: count each of the 12 possible changes
let spectrum = classified
    |> group_by(|v| v.change)
    |> map(|g| {change: g.0, count: len(g.1)})
    |> sort(|a, b| b.count - a.count)

print("\nSubstitution spectrum:")
for entry in spectrum {
    let bar = str_repeat("*", entry.count / 100)
    print(entry.change + "\t" + str(entry.count) + "\t" + bar)
}

write_csv(sorted_report, "titv_per_chromosome.csv")
write_csv(spectrum, "substitution_spectrum.csv")
```

## Example: Annotate Variants with Overlapping Regulatory Regions

Given a set of variants and a regulatory region BED file (e.g., ENCODE cCREs),
annotate each variant with the regulatory elements it falls within.

```biolang
# Load regulatory regions from ENCODE
let regulatory = read_bed("ENCODE_cCREs.bed")
    |> map(|r| interval(r.chrom, r.start, r.end, name: r.name ?? "cCRE"))

let reg_tree = interval_tree(regulatory)

# Load variants
let variants = read_vcf("gwas_hits.vcf")

# Annotate each variant with overlapping regulatory regions
let annotated = variants |> map(|v| {
    let pos_interval = interval(v.chrom, v.pos - 1, v.pos + len(v.ref) - 1)
    let overlapping = query_overlaps(reg_tree, pos_interval)
    let nearest = query_nearest(reg_tree, pos_interval)

    let reg_names = overlapping |> map(|r| r.name) |> join(";")
    let nearest_name = nearest?.name ?? "none"
    let nearest_dist = if len(overlapping) > 0 {
        0
    } else {
        distance(pos_interval, nearest)
    }

    {
        chrom: v.chrom,
        pos: v.pos,
        ref: v.ref,
        alt: v.alt,
        in_regulatory: len(overlapping) > 0,
        regulatory_elements: if len(reg_names) > 0 { reg_names } else { "none" },
        num_overlapping: len(overlapping),
        nearest_element: nearest_name,
        distance_to_nearest: nearest_dist
    }
})

# Summary statistics
let in_reg = annotated |> filter(|v| v.in_regulatory) |> len()
let total = len(annotated)
print("Variants in regulatory regions: " + str(in_reg) + "/" + str(total)
    + " (" + str(round(in_reg / total * 100, 1)) + "%)")

# Distribution of distance to nearest regulatory element
let distances = annotated
    |> filter(|v| !v.in_regulatory)
    |> map(|v| v.distance_to_nearest)

print("Non-regulatory variants:")
print("  Median distance to nearest cCRE: " + str(median(distances)) + " bp")
print("  Within 1kb of cCRE: " + str(distances |> filter(|d| d <= 1000) |> len()))
print("  Within 10kb of cCRE: " + str(distances |> filter(|d| d <= 10000) |> len()))

# Variants overlapping multiple regulatory elements
let multi = annotated |> filter(|v| v.num_overlapping > 1)
print("\nVariants in multiple regulatory regions: " + str(len(multi)))
for v in multi |> take(10) {
    print("  " + v.chrom + ":" + str(v.pos) + " overlaps " + str(v.num_overlapping)
        + " elements: " + v.regulatory_elements)
}

write_csv(annotated, "variants_regulatory_annotation.csv")
```

## Summary

### Interval Operations

| Function | Description |
|---|---|
| `interval(chrom, start, end)` | Create an interval |
| `overlaps(a, b)` | Test for overlap |
| `contains(a, b)` | Test containment |
| `distance(a, b)` | Gap between non-overlapping intervals |
| `intersect(a, b)` | Overlapping portion |
| `merge_interval(a, b)` | Union of overlapping pair |
| `interval_tree(list)` | Build tree for fast queries |
| `query_overlaps(tree, q)` | All intervals overlapping q |
| `query_nearest(tree, q)` | Closest interval to q |
| `coverage(tree, region)` | Per-base depth array |

### Variant Operations

| Function | Description |
|---|---|
| `variant(chrom, pos, ref, alt)` | Create a variant |
| `variant_type(v)` | "Snp", "Indel", "Mnp", "Other" |
| `is_snp(v)` | Single nucleotide change |
| `is_indel(v)` | Insertion or deletion |
| `is_transition(v)` | Purine-purine or pyrimidine-pyrimidine |
| `is_transversion(v)` | Purine-pyrimidine or vice versa |
| `is_het(v)` | Heterozygous genotype |
| `is_hom_ref(v)` | Homozygous reference |
| `is_hom_alt(v)` | Homozygous alternate |
| `parse_vcf_info(str)` | Parse INFO column to record |

Intervals and variants are the coordinate system of genomics. Master these
types and you can express peak-calling, variant annotation, coverage analysis,
and regulatory overlap queries concisely and efficiently.
