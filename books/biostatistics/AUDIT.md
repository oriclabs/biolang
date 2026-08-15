# Biostatistics learning-book audit

This audit covers all 30 `day-*.md` chapters. It is intended to keep the book
practical without turning descriptive clues into automatic scientific
decisions.

## Reproducible checks

Run from the repository root after building `bl`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-biostatistics-book.ps1
```

The audit requires exactly 30 day chapters and parses every fenced `bio` or
`biolang` block independently with `bl check --no-gpu`. At the time of this
audit it checks 318 code blocks. Parsing verifies BioLang syntax; it does not
claim that examples requiring external files have been executed or that an
illustrative dataset is real.

The external numerical oracle is separate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File packages/statistics/validation/run.ps1 `
  -RscriptPath "C:\Program Files\R\R-4.5.2\bin\Rscript.exe" -RequireR
```

It currently compares 147 scale-sensitive metrics with R, including GLM,
random-intercept, and Cox diagnostics plus real
fixtures generated from `airquality`, `Nile`, `ChickWeight`, and
`survival::lung`. Generated fixtures and outputs remain ignored.

## Concepts reviewed

- centre is paired with a compatible spread and scientific estimand;
- variance is SD squared, not the full left-to-right width;
- ±1/2/3 SD coverage is a normal-model reference, not a universal outlier rule;
- shape tests and plots do not automatically choose a hypothesis test;
- rank, permutation, and bootstrap methods still have design assumptions;
- log transforms change additive questions into ratio/multiplicative questions;
- count data, zeros, censoring, dependence, and repeated measurements require
  sampling-process-aware methods;
- p-values do not give the probability that a hypothesis is true or prove that
  an effect exists;
- hazard ratios are conditional event-rate ratios, not individual risk or
  survival-time reductions;
- exact output is calculated by runnable examples or clearly labelled as an
  illustrative scenario, rather than presented as an unmeasured BioLang run.

## Interpretation boundary

This is a maintained teaching audit, not a claim that prose can be proved
correct by a linter. New examples should be checked against primary statistical
references, executable BioLang behaviour, and an independent oracle where a
numerical equivalence claim is made.
