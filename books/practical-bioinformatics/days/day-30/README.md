# Day 30: Capstone --- Multi-Species Gene Family Analysis

| | |
|---|---|
| **Difficulty** | Advanced |
| **Biology knowledge** | Advanced (molecular evolution, protein domains, phylogenetics, comparative genomics) |
| **Coding knowledge** | Advanced (all prior topics: pipes, tables, statistics, visualization, APIs) |
| **Time** | ~5--6 hours |
| **Prerequisites** | Days 1--29 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (synthetic ortholog sequences for 8 species) |

## Quick Start

```bash
cd days/day-30
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Gathering orthologous gene sequences from multiple species
- Pairwise sequence comparison (dotplots, k-mer similarity)
- Conservation scoring across species
- Domain architecture comparison
- Distance matrix construction and phylogenetic tree visualization
- Evolutionary rate analysis across protein domains
- Integrating data from NCBI, UniProt, Reactome, STRING, and PDB

## Output Files

| File | Description |
|---|---|
| `data/output/sequence_summary.tsv` | Ortholog lengths and divergence times |
| `data/output/similarity_table.tsv` | K-mer similarity to human TP53 |
| `data/output/distance_matrix.tsv` | Pairwise k-mer distance matrix |
| `data/output/domain_conservation.tsv` | Per-domain conservation scores |
| `data/output/domain_architecture.tsv` | Domain architecture per species |
| `data/output/evolutionary_rates.tsv` | DBD vs TAD evolutionary rates |
| `data/output/dotplot_human_mouse.svg` | Human-mouse dotplot |
| `data/output/dotplot_human_fly.svg` | Human-fly dotplot |
| `data/output/conservation_profile.svg` | Sliding window conservation plot |
| `data/output/phylo_tree.svg` | Neighbor-joining phylogenetic tree |
| `data/output/rate_dbd.svg` | DBD substitution rate vs divergence |
| `data/output/rate_tad.svg` | TAD substitution rate vs divergence |
| `data/output/summary.txt` | Pipeline summary statistics |
