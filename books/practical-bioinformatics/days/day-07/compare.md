# Day 7: Line-of-Code Comparison

## Task: Multi-format bioinformatics file reading, analysis, conversion, and output

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 112 | 168 | 172 |
| Import/setup | 0 | 6 | 5 |
| FASTA reading | 8 | 10 | 10 |
| VCF reading + analysis | 18 | 28 | 26 |
| BED reading | 10 | 16 | 12 |
| GFF reading | 12 | 30 | 30 |
| BAM reading | 10 | 20 | 22 |
| Format conversions | 8 | 10 | 6 |
| Writing output | 12 | 16 | 14 |
| Dependencies | 0 (built-in) | 3 (biopython, pysam, pandas) | 4 (Biostrings, VariantAnnotation, GenomicRanges, rtracklayer) |

## Key Differences

### Reading FASTA
```
BioLang:  let seqs = read_fasta("file.fasta")
Python:   seqs = list(SeqIO.parse("file.fasta", "fasta"))
R:        seqs <- readDNAStringSet("file.fasta")
```

### GC Content
```
BioLang:  gc_content(s.seq)
Python:   gc_fraction(s.seq)
R:        sum(letterFrequency(seqs[[i]], c("G","C"))) / width(seqs[[i]])
```

### Reading VCF
```
BioLang:  let variants = read_vcf("file.vcf")
Python:   # 15 lines of manual parsing (pysam requires bgzip+index)
R:        # 15 lines of manual parsing (VariantAnnotation requires index)
```

### Filtering Variants
```
BioLang:  variants |> filter(|v| v.filter == "PASS")
Python:   [v for v in variants if v["filter"] == "PASS"]
R:        variants[variants$filter == "PASS", ]
```

### Counting by Group
```
BioLang:  passed |> to_table() |> group_by("chrom") |> summarize(|chrom, rows| {chrom: chrom, count: len(rows)})
Python:   Counter(v["chrom"] for v in passed)
R:        table(passed$chrom)
```

### Reading BED
```
BioLang:  let regions = read_bed("file.bed")
Python:   # 10 lines of manual tab-parsing
R:        bed <- read.delim("file.bed", header=FALSE, ...)
```

### Reading GFF
```
BioLang:  let features = read_gff("file.gff")
Python:   # 20 lines of manual parsing + attribute extraction
R:        # 20 lines of manual parsing + attribute extraction
```

### Reading BAM
```
BioLang:  let alns = read_bam("file.bam")
Python:   import pysam; alns = list(pysam.AlignmentFile("file.bam"))
R:        # Manual SAM text parsing or Rsamtools::scanBam()
```

### VCF to BED Conversion
```
BioLang:  variants |> map(|v| {chrom: v.chrom, start: v.pos - 1, end: v.pos - 1 + len(v.ref)})
Python:   [{"chrom": v["chrom"], "start": v["pos"]-1, "end": v["pos"]-1+len(v["ref"])} for v in variants]
R:        data.frame(chrom=v$chrom, start=v$pos-1, end=v$pos-1+nchar(v$ref))
```

### Writing FASTA
```
BioLang:  write_fasta(seqs, "output.fasta")
Python:   SeqIO.write(seqs, open("output.fasta", "w"), "fasta")
R:        writeXStringSet(seqs, "output.fasta")
```

## Summary

BioLang provides uniform, one-line readers for every major bioinformatics format. Python and R require either manual text parsing or format-specific libraries with complex setup requirements (bgzip+indexing for pysam VCF, Bioconductor installation for R). The VCF and GFF parsing gap is particularly large: BioLang reads these with a single function call, while Python and R need 15-20 lines of manual field splitting and attribute parsing. The pipe-based filtering and conversion syntax in BioLang maps directly to the analyst's intent without boilerplate.
