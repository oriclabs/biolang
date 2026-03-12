# Annotation Pipeline Benchmark Report

**Category**: pipelines / annotation
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Annotation | 1.013 | 1.019 | - | 1x | - |
| ClinVar + Ensembl Annotation | 1.012 | 1.04 | - | 1x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| Variant Annotation | 12 | 29 | - |
| ClinVar + Ensembl Annotation | 12 | 31 | - |

## Output Comparison

### Variant Annotation

**BioLang**:
```
Annotation Pipeline:
  Total variants: 50000
  After quality filter: 27585
  Chromosomes with variants: 24
  Annotated genes: 500
  Pathways:
    immune_response: 60 genes
    cell_cycle: 59 genes
    apoptosis: 57 genes
    translation: 54 genes
    transcription: 52 genes
```

**Python**:
```
Annotation Pipeline:
  Total variants: 50000
  After quality filter: 27585
  Chromosomes with variants: 24
  Annotated genes: 500
  Pathways:
    immune_response: 60 genes
    cell_cycle: 59 genes
    apoptosis: 57 genes
    translation: 54 genes
    transcription: 52 genes
```

### ClinVar + Ensembl Annotation

**BioLang**:
```
Annotation Pipeline (Real Data):
  ClinVar variants: 23675
  After filter: 23675
  Chromosomes with variants: 24
  Ensembl genes (chr22): 505
  Pathways:
    metabolism: 42 genes
    transcription: 37 genes
    translation: 36 genes
    chromatin_remodeling: 36 genes
    rna_processing: 36 genes
```

**Python**:
```
Annotation Pipeline (Real Data):
  ClinVar variants: 23675
  After filter: 23675
  Chromosomes with variants: 24
  Ensembl genes (chr22): 505
  Pathways:
    metabolism: 42 genes
    transcription: 37 genes
    rna_processing: 36 genes
    chromatin_remodeling: 36 genes
    translation: 36 genes
```

