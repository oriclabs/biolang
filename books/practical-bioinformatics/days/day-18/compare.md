# Day 18: Language Comparison --- Genomic Coordinates and Intervals

## Line Counts

| Operation | BioLang | Python (pyranges) | R (GenomicRanges) |
|-----------|---------|-------------------|-------------------|
| Create interval | 1 | 3 | 1 |
| Read BED file | 1 | 1 | 1 |
| Build interval tree | 1 | (automatic) | (automatic) |
| Query overlaps | 1 | 1 | 1 |
| Count overlaps | 1 | 2 | 2 |
| Bulk overlaps | 1 | 4 (loop) | 4 (loop) |
| Nearest query | 1 | 2 | 2 |
| Read VCF | 1 | 10+ | 5 |
| Coordinate conversion | 1 | 1 | 1 |
| Write BED | 1 | 2 | 1 |
| **Total script** | **~80** | **~120** | **~100** |

## Key Differences

### Creating Intervals

```
# BioLang — native type with literal syntax
let brca1 = interval("chr17", 43044295, 43125483)
print(brca1.chrom)   # field access

# Python — must construct DataFrame + PyRanges
brca1 = pr.PyRanges(pd.DataFrame({
    "Chromosome": ["chr17"],
    "Start": [43044295],
    "End": [43125483],
}))

# R — GRanges constructor
brca1 <- GRanges(seqnames = "chr17",
                 ranges = IRanges(start = 43044295, end = 43125483))
```

### Overlap Queries

```
# BioLang — explicit tree + query
let tree = interval_tree(regions)
let hits = query_overlaps(tree, query)
let n = count_overlaps(tree, query)

# Python — PyRanges has implicit overlap
hits = regions.overlap(query)

# R — GenomicRanges findOverlaps
hits <- findOverlaps(query, regions)
```

### Variant-in-Region Filtering

```
# BioLang — pipe chain with coordinate conversion
let exonic = variants |> filter(|v| {
    let vi = interval(v.chrom, v.pos - 1, v.pos)
    count_overlaps(tree, vi) > 0
})

# Python — construct GRanges, then overlap
variants_gr = pr.PyRanges(pd.DataFrame({...}))
exonic = variants_gr.overlap(exons)

# R — findOverlaps + queryHits
overlaps <- findOverlaps(variants_gr, exons)
exonic_idx <- unique(queryHits(overlaps))
```

### Coverage

```
# BioLang — built-in coverage visualization
coverage(reads, "chr1")

# Python — requires bedtools or manual computation
# pyranges does not have a direct coverage function
# pybedtools: a = pybedtools.BedTool(reads); cov = a.genomecov()

# R — built-in coverage in GenomicRanges
cov <- coverage(reads_gr)
```

## Dependencies

| Language | Packages Required |
|----------|-------------------|
| BioLang | None (built-in) |
| Python | pyranges, pandas, numpy |
| R | GenomicRanges, IRanges, rtracklayer (Bioconductor) |

## Summary

BioLang provides interval operations as built-in language primitives. Python requires pyranges (or pybedtools + pysam) and manual VCF parsing. R uses Bioconductor's GenomicRanges, which is powerful but requires Bioconductor installation. BioLang's explicit `interval_tree()` + `query_overlaps()` pattern makes the computational cost visible, while pyranges and GenomicRanges build index structures implicitly.

The biggest practical difference is VCF reading: BioLang's `read_vcf()` handles the format natively, while Python requires either manual parsing or the `cyvcf2`/`pysam` package. R has `VariantAnnotation::readVcf()` but it is heavy for simple use cases.
