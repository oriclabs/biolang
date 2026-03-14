# Day 25: Error Handling in Production

| | |
|---|---|
| **Difficulty** | Intermediate--Advanced |
| **Biology knowledge** | Intermediate (FASTQ quality scores, FASTA format, sequence data) |
| **Coding knowledge** | Intermediate (functions, records, pipes, tables, file I/O) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1--24 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (includes intentionally corrupted files) |

## Quick Start

```bash
cd days/day-25
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Using try/catch for graceful error recovery in pipelines
- Classifying errors (transient vs permanent) for appropriate handling
- Implementing retry logic with exponential backoff
- Validating inputs before processing to fail fast
- Handling partial failures in batch processing (accumulator pattern)
- Logging structured errors for post-mortem debugging
- Checkpointing long-running pipelines for crash recovery

## Files

| File | Description |
|------|-------------|
| `init.bl` | Generates test FASTQ files including intentionally corrupted data |
| `scripts/analysis.bl` | BioLang resilient pipeline (clean, no comments) |
| `scripts/analysis.py` | Python equivalent (try/except, logging, tenacity) |
| `scripts/analysis.R` | R equivalent (tryCatch, futile.logger) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Line-count and feature comparison across languages |
| `expected/output.txt` | Expected output from the analysis |

## Test Data

The `init.bl` script generates these intentionally varied files:

| File | Purpose |
|------|---------|
| `good_001.fastq` -- `good_005.fastq` | Well-formed FASTQ, passes all checks |
| `truncated.fastq` | Cut off mid-record (triggers parse error) |
| `empty.fastq` | Zero content (triggers empty file check) |
| `bad_quality.fastq` | Valid format, all low-quality bases (triggers QC filter) |
| `mismatched.fastq` | Sequence/quality length mismatch (triggers validation) |
