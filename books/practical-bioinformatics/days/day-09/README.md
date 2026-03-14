# Day 9: Biological Databases and APIs

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (gene identifiers, protein accessions, pathway concepts) |
| **Coding knowledge** | Intermediate (API calls, records, iteration) |
| **Time** | ~2.5 hours |
| **Prerequisites** | Days 1-8 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (all data fetched from live APIs) |
| **Requirements** | Internet connection required for all exercises |

## What You'll Learn

- How to query NCBI for genes, sequences, and literature
- How to use Ensembl for gene models, coordinates, and variant effect prediction
- How to look up protein function and features in UniProt
- How to find pathway memberships with KEGG
- How to search for 3D structures in PDB
- How to map protein interaction networks with STRING
- How to use Gene Ontology and Reactome for functional annotations
- How to combine multiple databases into a single gene profile
- Rate limiting best practices and batch query patterns

## Files

| File | Description |
|------|-------------|
| `init.bl` | Checks internet connectivity and API key configuration |
| `scripts/analysis.bl` | Complete multi-database query pipeline in BioLang |
| `scripts/analysis.py` | Python equivalent (requests, Biopython Entrez, etc.) |
| `scripts/analysis.R` | R equivalent (httr, rentrez, biomaRt) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Line-of-code comparison across languages |
| `expected/output.txt` | Expected console output (approximate --- API responses vary) |

## Quick Start

```bash
# Step 1: Check connectivity and API keys
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

## Important Notes

- All scripts in this day require an active internet connection.
- Output is approximate --- biological databases are updated regularly, so exact counts and descriptions may change.
- NCBI rate limits: 3 requests/second without key, 10/second with `NCBI_API_KEY`. Set the env var for best results.
- If a database is temporarily unavailable (maintenance windows, etc.), individual queries may fail. The scripts use `sleep()` between requests to be respectful.

## Chapter

The full Day 9 chapter is in the book at `book/src/day-09.md`.
