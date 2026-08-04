# RNA-seq Mini Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-08-04 03:15:54

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| RNA-seq DE Analysis | 0.005 | 0.013 | - | 2.6x | - |

## Output Comparison

### RNA-seq DE Analysis

**BioLang**:
```
RNA-seq Mini Pipeline:
  Genes: 100
  Samples: 20
  DE genes (|log2FC| >= 0.585): 2
  Upregulated: 2
  Downregulated: 0
```

**Python**:
```
RNA-seq Mini Pipeline:
  Genes: 100
  Samples: 20
  DE genes (|log2FC| >= 0.585): 2
  Upregulated: 2
  Downregulated: 0
```

