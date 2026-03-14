# Day 4: Coding Crash Course for Biologists

| | |
|---|---|
| **Difficulty** | Beginner |
| **Biology knowledge** | Minimal (lab analogies used throughout) |
| **Coding knowledge** | Core content (this IS the coding lesson) |
| **Time** | ~2.5 hours |
| **Prerequisites** | BioLang installed (see Appendix A) |
| **Data needed** | None (all data inline) |

## What You'll Learn

- Why code beats spreadsheets for reproducibility, scale, and sharing
- Variables, types, and how they map to lab concepts (labeled tubes)
- Lists (sample racks) and records (notebook entries)
- Loops for batch processing every sample the same way
- Conditions for QC decision-making in code
- Functions as reusable protocols (SOPs)
- Pipes (`|>`) to chain processing steps like a lab workflow
- Error handling: what goes wrong and how to fix it
- A complete gene expression fold-change analysis

## Files

| File | Description |
|------|-------------|
| `init.bl` | Minimal setup script (verifies BioLang is working) |
| `scripts/analysis.bl` | Clean BioLang script — complete Day 4 analysis |
| `scripts/analysis.py` | Python equivalent using standard libraries |
| `scripts/analysis.R` | R equivalent using base R and dplyr |
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

The full Day 4 chapter is in the book at `book/src/day-04.md`.
