# Day 22: Reproducible Pipelines

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Basic (FASTQ quality, GC content, sequence filtering) |
| **Coding knowledge** | Intermediate (functions, records, file I/O, checksums, JSON) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-21 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` |

## What You'll Learn

- How to design pipelines with external config files instead of hardcoded parameters
- How to use SHA-256 checksums to verify data integrity
- How to build automatic provenance logging for every pipeline run
- How to structure modular, shareable analyses

## Files

| File | Description |
|------|-------------|
| `init.bl` | Creates project structure, generates test FASTQ data and config |
| `scripts/analysis.bl` | Complete BioLang reproducible QC pipeline |
| `scripts/analysis.py` | Python equivalent with logging and hashlib |
| `scripts/analysis.R` | R equivalent with logging and digest |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected console output |

## Quick Start

```bash
# Create project structure and test data
bl run init.bl

# Run the BioLang version
bl run scripts/analysis.bl

# Run the Python version
pip install -r python/requirements.txt
python scripts/analysis.py

# Run the R version
Rscript r/install.R
Rscript scripts/analysis.R
```

## Output Files

After running:

| File | Description |
|------|-------------|
| `results/qc_summary.csv` | Per-sample QC summary table |
| `logs/provenance_*.json` | Timestamped provenance log with checksums |
