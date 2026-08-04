# Interval Operations Benchmark Report

**Category**: language / intervals
**Platform**: Microsoft Windows 10.0.26200 , 12th Gen Intel(R) Core(TM) i9-12900K, 31.7 GB
**Date**: 2026-08-04 18:34:56

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| BED Interval Overlap | 0.063 | 0.065 | 1.155 | 1x | 18.3x |
| ENCODE Peak Overlap | 0.193 | 3.133 | - | 16.2x | - |

## Lines of Code

| Task | BioLang | Python | R |
|---|---|---|---|
| BED Interval Overlap | 7 | 27 | 11 |
| ENCODE Peak Overlap | 7 | 34 | - |

## Output Comparison

### BED Interval Overlap

**BioLang**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

**Python**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

**R**:
```
Regions: 10000
Queries: 1000
Total overlaps: 13
```

### ENCODE Peak Overlap

**BioLang**:
```
H3K27ac peaks (regions): 52455
CTCF peaks (queries): 41952
Total overlaps: 7301
```

**Python**:
```
H3K27ac peaks (regions): 52455
CTCF peaks (queries): 41952
Total overlaps: 7301
```

