# Day 28: Capstone --- Clinical Variant Report

| | |
|---|---|
| **Difficulty** | Advanced |
| **Biology knowledge** | Advanced (genomic variants, clinical genetics, gene-disease associations) |
| **Coding knowledge** | Advanced (all prior topics: pipes, tables, APIs, error handling, modules) |
| **Time** | ~4--5 hours |
| **Prerequisites** | Days 1--27 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (simulated VCF, gene panels, reference files) |

> **CLINICAL DISCLAIMER**: This is an educational exercise. The pipeline must
> NOT be used for actual clinical decision-making. Real clinical variant
> interpretation requires CAP/CLIA-accredited pipelines and board-certified review.

## Quick Start

```bash
cd days/day-28
bl init.bl
bl scripts/analysis.bl
```

## What You'll Learn

- Loading and parsing VCF files for variant analysis
- Multi-stage quality and frequency filtering
- Variant annotation with gene and ClinVar databases
- ACMG-inspired variant classification logic
- Structured clinical report generation
- Clinical-grade error handling at every pipeline stage
- Integrating skills from all 27 prior days into a capstone project

## Output Files

| File | Description |
|---|---|
| `data/output/clinical_report.txt` | Full clinical variant analysis report |
| `data/output/classified_variants.tsv` | Classified variants in tabular format |
