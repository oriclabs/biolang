# Day 1: What Is Bioinformatics?

| | |
|---|---|
| **Difficulty** | Beginner |
| **Time** | ~2 hours |
| **Prerequisites** | None |
| **Data needed** | None (all sequences inline) |

## What You'll Learn

- What bioinformatics is and why it matters
- The central dogma: DNA, RNA, and Protein
- Why biology needs computation (the scale problem)
- Your first BioLang code: sequence literals, transcription, translation
- GC content, base composition, and motif searching

## Files

| File | Description |
|------|-------------|
| `init.bl` | Minimal setup script (verifies BioLang is working) |
| `scripts/analysis.bl` | Clean BioLang script — all Day 1 analyses |
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

The full Day 1 chapter is in the book at `book/src/day-01.md`.
