# Day 17: Protein Analysis

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (amino acids, protein structure, domains) |
| **Coding knowledge** | Intermediate (records, pipes, lambda functions, maps) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-16 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (all examples use API calls or inline sequences) |
| **Requirements** | Internet connection for API sections (UniProt, PDB, Ensembl) |

## What You'll Learn

- How to work with protein sequences and understand amino acid properties
- How to query UniProt for protein information, features, domains, and GO terms
- How to access 3D structure data from the PDB
- How to analyze amino acid composition and k-mer profiles
- How to compare orthologs across species and assess mutation impact

## Files

| File | Description |
|------|-------------|
| `scripts/protein_analysis.bl` | Complete protein analysis in BioLang |
| `scripts/protein_analysis.py` | Python equivalent (requests + Biopython) |
| `scripts/protein_analysis.R` | R equivalent (UniProt.ws + bio3d) |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected output from `protein_analysis.bl` |

## Quick Start

```bash
cd days/day-17
bl scripts/protein_analysis.bl
```

## Notes

This day does not require an `init.bl` script. All examples either use inline protein sequences or query remote APIs (UniProt, PDB, Ensembl). An internet connection is required for the API sections.
