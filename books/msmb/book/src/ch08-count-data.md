# 8. High-Throughput Count Data

Following Chapter 8 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch08/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch08)

---

## The idea in one paragraph

This is where the previous seven chapters meet. An RNA-seq analysis needs a count
model that admits overdispersion (Chapter 4), a test (Chapter 6), multiple-testing
control (Chapter 6), and a plot that does not mislead (Chapter 3). Getting any one
of them wrong invalidates the result, and each failure looks like a normal answer.

---

## 8.1 Differential expression, end to end

**Run:** `bl run 01-differential-expression.bl`

4000 genes, 8 vs 8 samples, 200 genuinely changed three-fold, counts drawn from a
negative binomial — the distribution Chapter 4 built.

### Library size comes first

```
  group A totals: [412051, 398772, 405119, ...]
```

Totals differ between samples for reasons that have nothing to do with biology:
how much was loaded, how the run went. Comparing raw counts finds "differences"
driven by sequencing depth, so counts are scaled to a common total before
anything else.

### Then the log scale

Counts are multiplicative — a gene going 100 → 300 is the same biological event as
1000 → 3000. Differences are meaningful in logs, the variance is far more stable
there, and `+1` keeps zeros finite.

### Results

| method | called | real | false | recovered |
|---|---|---|---|---|
| uncorrected p < 0.05 | 433 | 198 | 235 | 99.0% |
| FDR < 0.05 | 192 | 182 | **10** | 91.0% |
| FDR < 0.10 | 213 | 190 | 23 | 95.0% |

FDR at 5% delivered 10 false among 192 calls — **5.2%, its nominal rate** — while
recovering 91% of the truth. The uncorrected list finds one more real gene and
pays with 235 false ones.

### Power depends on expression level

```
  baseline counts      changed genes    detected at FDR 5%
       0 -      20             22             59.1%
      20 -      60             56             91.1%
      60 -     200             70             97.1%
     200 - 1000000             52             96.2%
```

The same three-fold change is caught six times in ten at the lowest expression
band and essentially always above a hundred counts. **The biology is identical
across those rows; only the measurement precision differs.**

Low-expression genes are absent from results not because nothing happened to them
but because the experiment could not have seen it. Filtering them before testing
costs nothing and *improves* the FDR correction, by removing tests that were never
going to be informative.

### The MA plot

```
0|    . ...................................... .... .     |
 |    . .......... ................... .. ..  . .    .    |
 |            .  . ...... .  ....  .  .                   |
 +--------------------------------------------------------+
  low expression                          high expression
```

Two things to read off it. The cloud **narrows to the right** — high-count genes
have more stable estimates, which is the power table restated. And the significant
genes sit top and bottom *right*: large changes in well-measured genes. Nothing is
called on the left however extreme its apparent fold change, and that is correct.

---

## What to take away

1. **Normalise for library size before anything else**, or you will find
   sequencing depth.
2. **Work in logs.** Count data is multiplicative and its variance is not
   constant on the raw scale.
3. **Use a count model that allows overdispersion.** Chapter 4 measured what
   ignoring this costs: a 5% test that rejects 69% of the time.
4. **A gene missing from your results may simply have been unmeasurable.** Absence
   of evidence, precisely.
5. **Filter low-count genes before testing**, not after — it improves the
   correction rather than weakening it.

## Notes on BioLang

`glm`, `lm`, `normalize_counts`, `diff_expr`, `log2_transform` and `ma_plot` all
exist; this chapter builds the pipeline from primitives instead, because the point
is what each step is for. The negative binomial sampler comes from
`msmbstats/src/distributions.bl` (Chapter 4).

There is no DESeq2-equivalent dispersion estimator — real tools shrink per-gene
dispersion estimates toward a fitted trend, which is most of what makes them work
on small sample sizes. This chapter assumes the dispersion is known, which is the
one place it is more optimistic than reality.

## Exercises

1. Skip the library-size normalisation and re-run. How many false positives does
   depth alone manufacture?
2. Test on raw counts rather than logs. What happens to the high-expression genes?
3. Drop to 3 samples per group. Which of the four expression bands survives?
4. Filter out genes below 20 counts before testing and compare the FDR results.
   Why does removing tests *increase* the number of discoveries?
