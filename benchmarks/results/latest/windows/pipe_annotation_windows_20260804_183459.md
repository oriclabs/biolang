# Annotation Pipeline Benchmark Report

**Category**: pipelines / annotation
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Annotation | 0.108 | 0.095 | - | 0.9x | - |
| ClinVar + Ensembl Annotation | 0.126 | 0.085 | - | 0.7x | - |

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
  ClinVar variants: 23678
  After filter: 23678
  Chromosomes with variants: 24
  Ensembl genes (chr22): 505
  Pathways:
    protein_folding: 52 genes
    immune_response: 39 genes
    transcription: 37 genes
    cell_adhesion: 36 genes
    signal_transduction: 36 genes
```

**Python**:
```
Annotation Pipeline (Real Data):
  ClinVar variants: 23678
  After filter: 23678
  Chromosomes with variants: 24
  Ensembl genes (chr22): 505
  Pathways:
    protein_folding: 52 genes
    immune_response: 39 genes
    transcription: 37 genes
    cell_adhesion: 36 genes
    signal_transduction: 36 genes
```

