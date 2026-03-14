# Day 24: Programmatic Database Access

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (gene names, protein accessions, pathway concepts, variant notation) |
| **Coding knowledge** | Intermediate (functions, records, error handling, pipes, tables) |
| **Time** | ~3--4 hours |
| **Prerequisites** | Days 1--23 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (gene list) |
| **Requirements** | Internet access for all API examples |

## Quick Start

```bash
cd days/day-24
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Querying NCBI, Ensembl, UniProt, KEGG, Reactome, STRING, GO, and PDB programmatically
- Building multi-database annotation pipelines
- Rate limiting and error handling for API calls
- Caching results to avoid redundant queries
- Cross-database integration for comprehensive gene annotation

## Files

| File | Description |
|------|-------------|
| `init.bl` | Generates `data/gene_list.csv` with 20 differentially expressed genes |
| `scripts/analysis.bl` | BioLang annotation pipeline (clean, no comments) |
| `scripts/analysis.py` | Python equivalent (requests, Biopython) |
| `scripts/analysis.R` | R equivalent (httr2, biomaRt, rentrez) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Line-count and feature comparison across languages |
| `expected/output.txt` | Expected output from the analysis |

## Requirements

All scripts in this day require **internet access** to query public bioinformatics APIs. API calls may fail if you are behind a restrictive firewall or if a database server is temporarily unavailable.

Optional environment variables:
- `NCBI_API_KEY` --- increases NCBI rate limit from 3 to 10 requests/second
