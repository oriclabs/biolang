# Variant Analysis Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| VCF Filtering | 0.349 | 0.166 | 6.312 | .4x | 18.0x |
| ClinVar Variants | 0.661 | 0.265 | 1.208 | .4x | 1.8x |

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

**R**:
```
Total variants: 20000
Pathogenic/Likely pathogenic: 637
Pathogenic on chr1: 637
Pathogenic SNPs: 341
Pathogenic Indels: 296
```

