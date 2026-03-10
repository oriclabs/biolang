# Multi-Sample Pipeline Benchmark Report

**Platform**: Linux 6.6.87.2-microsoft-standard-WSL2 x86_64, 12th Gen Intel(R) Core(TM) i9-12900K, 15 GB
**Date**: 2026-03-10 23:36:43

## Execution Time (seconds, best of 3)

| Task | BioLang | Python | R | BL vs Py | BL vs R |
|---|---|---|---|---|---|
| Multi-Sample Aggregation | 0.245 | 0.090 | - | .3x | - |

## Output Comparison

### Multi-Sample Aggregation

**BioLang**:
```
Multi-Sample Pipeline Results:
  Total samples: 5000
  Cohorts: 4

  treatment_C:
    Samples: 1244
    Mean depth: 42.9
    Mean quality: 27.6
    Total reads: 6438309898
  treatment_B:
    Samples: 1233
    Mean depth: 42.8
    Mean quality: 27.4
    Total reads: 6187278991
  treatment_A:
    Samples: 1268
    Mean depth: 42.4
    Mean quality: 27.5
    Total reads: 6439472843
  control:
    Samples: 1255
    Mean depth: 42.0
    Mean quality: 27.5
    Total reads: 6467124256

  Overall mean depth: 42.5
  Overall mean quality: 27.5
  Depth std dev: 21.77
```

**Python**:
```
Multi-Sample Pipeline Results:
  Total samples: 5000
  Cohorts: 4

  control:
    Samples: 1255
    Mean depth: 42.0
    Mean quality: 27.5
    Total reads: 6467124256
  treatment_A:
    Samples: 1268
    Mean depth: 42.4
    Mean quality: 27.5
    Total reads: 6439472843
  treatment_B:
    Samples: 1233
    Mean depth: 42.8
    Mean quality: 27.4
    Total reads: 6187278991
  treatment_C:
    Samples: 1244
    Mean depth: 42.9
    Mean quality: 27.6
    Total reads: 6438309898

  Overall mean depth: 42.5
  Overall mean quality: 27.5
  Depth std dev: 21.77
```

