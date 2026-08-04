# Variant Analysis Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| VCF Filtering | 0.066 | 0.061 | 5.414 | .9x | 82.0x |
| ClinVar Variants | 0.135 | 0.064 | 0.705 | .4x | 5.2x |

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

