# Variant Analysis Benchmark Report

**Category**: language / variants
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| VCF Filtering | 0.083 | 0.107 | 6.048 | 1.3x | 72.9x |
| ClinVar Variants | 0.18 | 0.103 | 0.841 | 0.6x | 4.7x |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| VCF Filtering | 11 | 28 | 19 |
| ClinVar Variants | 20 | 28 | 36 |

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

**R**:
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
Pathogenic/Likely pathogenic: 635
Pathogenic on chr1: 635
Pathogenic SNPs: 336
Pathogenic Indels: 299
```

**Python**:
```
Total variants: 20000
Pathogenic/Likely pathogenic: 635
Pathogenic on chr1: 635
Pathogenic SNPs: 336
Pathogenic Indels: 299
```

**R**:
```
Total variants: 20000
Pathogenic/Likely pathogenic: 635
Pathogenic on chr1: 635
Pathogenic SNPs: 336
Pathogenic Indels: 299
```

