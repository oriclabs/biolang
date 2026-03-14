# Day 11: Sequence Comparison

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (DNA composition, codons, restriction enzymes) |
| **Coding knowledge** | Intermediate (loops, records, functions, pipes) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-10 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (sequences defined inline) |
| **Requirements** | None (offline); internet optional for API examples |

## What You'll Learn

- How to compare sequences by base composition and GC content
- How k-mer decomposition enables alignment-free similarity
- How dotplots visually reveal similarity, repeats, and rearrangements
- How to find exact motifs including restriction enzyme recognition sites
- Why reverse complement matters for double-stranded DNA
- How to analyze codon usage bias across genes
- How to compare genes across species using Ensembl APIs

## Files

| File | Description |
|------|-------------|
| `init.bl` | Prints confirmation (no data files needed for this day) |
| `scripts/analysis.bl` | Complete sequence comparison pipeline in BioLang |
| `scripts/analysis.py` | Python equivalent (Biopython) |
| `scripts/analysis.R` | R equivalent (Biostrings, seqinr) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Line-of-code comparison across languages |
| `expected/output.txt` | Expected console output |

## Quick Start

```bash
# Step 1: Verify setup
cd days/day-11
bl run init.bl

# Step 2: Run the BioLang analysis
bl run scripts/analysis.bl

# Run the Python version
pip install -r python/requirements.txt
python scripts/analysis.py

# Run the R version
Rscript r/install.R
Rscript scripts/analysis.R
```

## Chapter

The full Day 11 chapter is in the book at `book/src/day-11.md`.
