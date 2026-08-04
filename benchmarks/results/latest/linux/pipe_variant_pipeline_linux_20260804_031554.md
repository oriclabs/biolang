# Variant Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Variant Analysis Pipeline | 0.083 | 0.094 | - | 1.1x | - |
| ClinVar Variant Pipeline | 0.119 | 0.084 | - | .7x | - |

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
  12: 1000 variants (971 SNPs, 29 indels)
  10: 1000 variants (889 SNPs, 111 indels)
  9: 1000 variants (966 SNPs, 34 indels)
  20: 1000 variants (948 SNPs, 52 indels)
  19: 1000 variants (997 SNPs, 3 indels)
```

**Python**:
```
Variant Pipeline (ClinVar Real Data):
  Chromosomes analyzed: 24
  1: 1000 variants (924 SNPs, 76 indels)
  2: 1000 variants (948 SNPs, 52 indels)
  3: 1000 variants (986 SNPs, 14 indels)
  4: 1000 variants (952 SNPs, 48 indels)
  5: 1000 variants (945 SNPs, 55 indels)
```

