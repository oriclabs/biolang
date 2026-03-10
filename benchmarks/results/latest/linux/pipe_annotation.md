# Annotation Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Annotation | 0.309 | 0.122 | - | .3x | - |
| ClinVar + Ensembl Annotation | 0.409 | 0.183 | - | .4x | - |

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
    chromatin_remodeling: 36 genes
    translation: 36 genes
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

