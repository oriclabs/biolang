# Day 5: Data Structures for Biology

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Light (gene names, genomic coordinates) |
| **Coding knowledge** | Intermediate (builds on Day 3-4 foundations) |
| **Time** | ~2.5 hours |
| **Prerequisites** | Days 3-4 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (all data inline) |

## What You'll Learn

- Lists for ordered collections: expression values, sample queues, ranked results
- Records for structured metadata: gene annotations, sample info, variant data
- Tables as the primary analysis structure: create, filter, mutate, group, summarize
- Sets for unique membership and Venn-diagram comparisons between experiments
- Genomic intervals and interval trees for coordinate overlap queries
- How to choose the right structure for any bioinformatics task
- Combining structures in a real analysis pattern

## Files

| File | Description |
|------|-------------|
| `init.bl` | Minimal setup script (verifies BioLang is working) |
| `scripts/analysis.bl` | Clean BioLang script covering all data structures |
| `scripts/analysis.py` | Python equivalent (pandas, sets, pybedtools-style) |
| `scripts/analysis.R` | R equivalent (data.frames, vectors, GenomicRanges-style) |
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

The full Day 5 chapter is in the book at `book/src/day-05.md`.
