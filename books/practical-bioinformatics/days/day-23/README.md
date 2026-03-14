# Day 23: Batch Processing and Automation

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Basic (FASTQ quality, sample sheets, sequencing runs) |
| **Coding knowledge** | Intermediate (functions, records, file I/O, parallel execution, error handling) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-22 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` |

## What You'll Learn

- How to parse sample sheets and discover files by directory traversal
- How to design per-sample processing functions for batch workflows
- How to use parallel execution for hundreds of samples
- How to handle errors per-sample so one failure does not stop the batch
- How to aggregate results into cohort-level summaries with outlier detection

## Files

| File | Description |
|------|-------------|
| `init.bl` | Creates project structure, generates 24 sample FASTQs and sample sheet |
| `scripts/analysis.bl` | Complete BioLang batch processing pipeline |
| `scripts/analysis.py` | Python equivalent with multiprocessing |
| `scripts/analysis.R` | R equivalent with parallel/mclapply |
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
| `results/batch_summary.csv` | Per-sample QC summary table (24 rows) |
| `results/group_summary.csv` | Per-group aggregated statistics |
| `logs/batch_report.json` | Batch report with timing, outliers, and errors |
