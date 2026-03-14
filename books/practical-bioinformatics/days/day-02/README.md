# Day 2: Your First Language — BioLang

| | |
|---|---|
| **Difficulty** | Beginner |
| **Biology knowledge** | None required |
| **Coding knowledge** | Beginner (no prior experience needed) |
| **Time** | ~2.5 hours |
| **Prerequisites** | BioLang installed (see Appendix A) |
| **Data needed** | None (all sequences inline) |

## What You'll Learn

- How to use the BioLang REPL for interactive exploration
- Variables, types, and BioLang's type system (including bio types)
- The pipe operator `|>` — the core concept of BioLang
- Lists, records, and basic collections
- Functions and lambdas
- Control flow: if/else, for, match
- Higher-order functions: map, filter, reduce, each
- How BioLang compares to Python and R for biology tasks

## Files

| File | Description |
|------|-------------|
| `init.bl` | Minimal setup script (verifies BioLang is working) |
| `scripts/analysis.bl` | Clean BioLang script — all Day 2 analyses |
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

The full Day 2 chapter is in the book at `book/src/day-02.md`.
