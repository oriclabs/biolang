# Day 3: Line-of-Code Comparison

## Task: Biology explorations (complement, translate, mutations, intervals)

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 76 | 96 | 101 |
| Import/setup | 0 | 4 | 4 |
| Sequence operations | 8 | 10 | 12 |
| Translation | 3 | 3 | 5 |
| Mutation comparison | 7 | 7 | 7 |
| Wobble experiment | 6 | 8 | 9 |
| Intervals | 10 | 14 | 14 |
| Dependencies | 0 (built-in) | 1 (biopython) | 1 (Biostrings) |

## Key Differences

### DNA Literals
```
BioLang:  let seq = dna"ATGCGA"
Python:   seq = Seq("ATGCGA")
R:        seq <- DNAString("ATGCGA")
```

### Translation
```
BioLang:  translate(seq)                           # stops at stop codon
Python:   seq.translate(to_stop=True)              # must pass flag
R:        sub("\\*$", "", as.character(translate(seq)))  # must strip stop manually
```

### Complement
```
BioLang:  complement(seq)
Python:   seq.complement()
R:        complement(seq)
```

### Genomic Intervals
```
BioLang:  let iv = interval("chr17", 100, 200)    # first-class type with .chrom, .start, .end
Python:   iv = ("chr17", 100, 200)                 # just a tuple — no methods
R:        iv <- list(chrom="chr17", start=100, end=200)  # named list — no type safety
```

### Pipe Syntax for Chaining
```
BioLang:  normal |> translate()
Python:   normal.translate(to_stop=True)           # method chaining (different pattern)
R:        translate(normal)                         # nested calls or magrittr pipe
```

## Summary

BioLang has the most concise syntax for biological operations. DNA literals, first-class intervals, and translate-to-stop as default behavior reduce boilerplate. Python requires explicit `to_stop=True`. R requires manual stop-codon stripping. For interval work, BioLang provides typed intervals with field access; Python and R require custom data structures or external packages (pybedtools, GenomicRanges).
