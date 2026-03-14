# Day 11: Language Comparison --- Sequence Comparison

## Line Counts

| Operation | BioLang | Python (Biopython) | R (Biostrings) |
|-----------|---------|-------------------|----------------|
| GC content | 1 | 2 | 3 |
| Base counts | 1 | 1 | 2 |
| K-mer extraction | 1 | 1 | 1 |
| K-mer frequency | 1 | 1 | 2 |
| Find motif | 1 | 7 | 2 |
| Reverse complement | 1 | 2 | 2 |
| Codon usage | 1 | 2 | 2 |
| Jaccard similarity | 5 | 5 | 5 |
| **Total script** | **~55** | **~90** | **~85** |

## Key Differences

### GC Content

```
# BioLang — built-in function on dna literal
gc_content(dna"ATCGATCG")

# Python — Biopython utility
from Bio.SeqUtils import gc_fraction
gc_fraction(Seq("ATCGATCG"))

# R — manual calculation from Biostrings
freq <- alphabetFrequency(DNAString("ATCGATCG"))
(freq["G"] + freq["C"]) / 8
```

### K-mer Extraction

```
# BioLang — built-in
kmers(dna"ATCGATCG", 3)

# Python — list comprehension
[s[i:i+3] for i in range(len(s) - 2)]

# R — sapply
sapply(1:(nchar(s) - 2), function(i) substr(s, i, i + 2))
```

### Motif Finding

```
# BioLang — returns all positions
find_motif(seq, "ATG")

# Python — manual loop with str.find
positions = []
start = 0
while True:
    pos = seq.find("ATG", start)
    if pos == -1: break
    positions.append(pos)
    start = pos + 1

# R — Biostrings matchPattern
matchPattern("ATG", DNAString(seq))
```

### Reverse Complement

```
# BioLang — built-in on dna literal
reverse_complement(dna"ATCG")

# Python — Biopython method
Seq("ATCG").reverse_complement()

# R — Biostrings function
reverseComplement(DNAString("ATCG"))
```

### Set Operations for K-mer Comparison

```
# BioLang — built-in set functions
let shared = intersection(set(k1), set(k2))

# Python — native set operators
shared = k1 & k2

# R — base R
shared <- intersect(k1, k2)
```

## Observations

- BioLang's `dna"..."` literals integrate sequence operations into the type system. Python and R treat sequences as strings or special objects that must be constructed.
- BioLang has built-in `find_motif()` that returns all positions in one call. Python requires a manual loop; R's `matchPattern()` is concise but returns a complex object.
- BioLang's `kmers()` and `kmer_count()` are purpose-built. Python and R require list comprehensions or sapply.
- All three languages handle set operations naturally, but BioLang's named functions (`intersection`, `union`) are more explicit than Python's operators (`&`, `|`).
- BioLang requires no imports. Biopython must be installed and imported; Biostrings requires Bioconductor.
