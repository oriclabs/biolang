# Variant Pipeline Benchmark Report

**Category**: pipelines / variant_pipeline
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Analysis Pipeline | 1.017 | 1.028 | - | 1x | - |
| ClinVar Variant Pipeline | 1.024 | 1.024 | - | 1x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| Variant Analysis Pipeline | 8 | 29 | - |
| ClinVar Variant Pipeline | 16 | 31 | - |

## Output Comparison

### Variant Analysis Pipeline

**BioLang**:
```
Variant Pipeline Results:
  Chromosomes analyzed: 24
  chr11: 1117 variants (891 SNPs, 226 indels)
  chr7: 1115 variants (872 SNPs, 243 indels)
  chr15: 1090 variants (878 SNPs, 212 indels)
  chr1: 1088 variants (888 SNPs, 200 indels)
  chr13: 1081 variants (854 SNPs, 227 indels)
```

**Python**:
```
Variant Pipeline Results:
  Chromosomes analyzed: 24
  chr11: 1117 variants (891 SNPs, 226 indels, mean QUAL 45.6)
  chr7: 1115 variants (872 SNPs, 243 indels, mean QUAL 44.9)
  chr15: 1090 variants (878 SNPs, 212 indels, mean QUAL 45.0)
  chr1: 1088 variants (888 SNPs, 200 indels, mean QUAL 44.6)
  chr13: 1081 variants (854 SNPs, 227 indels, mean QUAL 45.1)
```

### ClinVar Variant Pipeline

**BioLang**:
```
Variant Pipeline (ClinVar Real Data):
  Chromosomes analyzed: 24
  4: 1000 variants (952 SNPs, 48 indels)
  18: 1000 variants (972 SNPs, 28 indels)
  1: 1000 variants (924 SNPs, 76 indels)
  8: 1000 variants (972 SNPs, 28 indels)
  5: 1000 variants (947 SNPs, 53 indels)
```

**Python**:
```
Variant Pipeline (ClinVar Real Data):
  Chromosomes analyzed: 24
  1: 1000 variants (924 SNPs, 76 indels)
  2: 1000 variants (950 SNPs, 50 indels)
  3: 1000 variants (986 SNPs, 14 indels)
  4: 1000 variants (952 SNPs, 48 indels)
  5: 1000 variants (947 SNPs, 53 indels)
```

