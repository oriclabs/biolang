# Variant Analysis Benchmark Report

**Category**: language / variants
**Platform**: Microsoft Windows 10.0.26300 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-03-10 23:52:35

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| VCF Filtering | 1.02 | 1.029 | - | 1x | - |
| ClinVar Variants | 1.009 | 1.027 | - | 1x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| VCF Filtering | 11 | 28 | - |
| ClinVar Variants | 20 | 28 | - |

## Output Comparison

### VCF Filtering

**BioLang**:
```
Total variants: 50000
After filtering: 1088
SNPs: 888
Indels: 200
Ti/Tv ratio: computed from filtered set
```

**Python**:
```
Total variants: 50000
After filtering: 1088
SNPs: 888
Indels: 200
Ti/Tv ratio: computed from filtered set
```

### ClinVar Variants

**BioLang**:
```
Total variants: 20000
Pathogenic/Likely pathogenic: 637
Pathogenic on chr1: 637
Pathogenic SNPs: 341
Pathogenic Indels: 296
```

**Python**:
```
Total variants: 20000
Pathogenic/Likely pathogenic: 637
Pathogenic on chr1: 637
Pathogenic SNPs: 341
Pathogenic Indels: 296
```

