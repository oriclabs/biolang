# Day 20: Multi-Species Comparison

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (orthologs, conservation, phylogenetics, k-mers) |
| **Coding knowledge** | Intermediate (API calls, records, pipes, nested loops, try/catch) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-19 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (API-based); internet connection required |

## What You'll Learn

- How to fetch ortholog sequences across species using the Ensembl API
- How to compare sequence properties and compute alignment-free similarity
- How to visualize conservation patterns and phylogenetic relationships
- How to export sequences for external alignment and tree-building tools

## Files

| File | Description |
|------|-------------|
| `init.bl` | Creates output directories (no data generation; this day is API-based) |
| `scripts/multi_species.bl` | Complete BioLang multi-species comparison pipeline |
| `scripts/multi_species.py` | Python equivalent (Biopython, requests) |
| `scripts/multi_species.R` | R equivalent (biomaRt, Biostrings) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected console output |

## Quick Start

```bash
# Create output directories
bl run init.bl

# Run the BioLang version (requires internet)
bl run scripts/multi_species.bl

# Run the Python version
pip install -r python/requirements.txt
python scripts/multi_species.py

# Run the R version
Rscript r/install.R
Rscript scripts/multi_species.R
```

## Output Files

After running:

| File | Description |
|------|-------------|
| `results/species_comparison.csv` | Cross-species comparison table |
| `results/brca1_orthologs.fasta` | BRCA1 protein sequences for external alignment |
