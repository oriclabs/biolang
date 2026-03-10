# Variant Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Analysis Pipeline | 0.268 | 0.177 | - | .6x | - |
| ClinVar Variant Pipeline | 0.591 | 0.261 | - | .4x | - |

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
  10: 1000 variants (894 SNPs, 106 indels)
  7: 1000 variants (941 SNPs, 59 indels)
  18: 1000 variants (972 SNPs, 28 indels)
  15: 1000 variants (958 SNPs, 42 indels)
  17: 1000 variants (953 SNPs, 47 indels)
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

