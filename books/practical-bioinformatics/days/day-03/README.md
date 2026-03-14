# Day 3: Biology Crash Course for Developers

| | |
|---|---|
| **Difficulty** | Beginner |
| **Biology knowledge** | Core content (this IS the biology lesson) |
| **Coding knowledge** | Minimal (just reading BioLang examples) |
| **Time** | ~2 hours |
| **Prerequisites** | BioLang installed (see Appendix A) |
| **Data needed** | None (all sequences inline); internet for NCBI exercises |

## What You'll Learn

- The central dogma: DNA, RNA, and Protein — framed as a computing system
- DNA structure: bases, base pairing, complementarity, strand direction
- Gene structure: exons, introns, splicing
- The genetic code: codons, amino acids, start/stop signals
- Mutation types: SNP, missense, nonsense, frameshift — and why they matter
- Gene expression: which genes are active and how we measure it
- Reference genomes and genomic coordinates (BED vs VCF conventions)
- The -omics landscape: genomics, transcriptomics, proteomics, and more
- The TP53 story: a real gene, real mutations, real clinical impact

## Files

| File | Description |
|------|-------------|
| `init.bl` | Minimal setup script (verifies BioLang is working) |
| `scripts/analysis.bl` | Clean BioLang script — all Day 3 biology explorations |
| `scripts/analysis.py` | Python equivalent using Biopython |
| `scripts/analysis.R` | R equivalent using Biostrings |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Line-of-code comparison across languages |
| `expected/output.txt` | Expected console output |

## Quick Start

```bash
# Run the BioLang version
bl run scripts/analysis.bl

# Run the Python version
pip install -r python/requirements.txt
python scripts/analysis.py

# Run the R version
Rscript r/install.R
Rscript scripts/analysis.R
```

## Chapter

The full Day 3 chapter is in the book at `book/src/day-03.md`.
