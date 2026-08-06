# 6. Testing

Following Chapter 6 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch06/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch06)

---

## The idea in one paragraph

A hypothesis test is a decision rule with a known error rate. That is all it is,
and saying it that way makes clear what it cannot do: it does not tell you
whether an effect is real, how large it is, or whether it matters. Chapter 6 is
about the two ways such a rule fails — and about what happens when you run twenty
thousand of them at once, which is the situation genomics is actually in.

---

## 6.1 What a test does

**Run:** `bl run 01-what-a-test-is.bl`

2000 comparisons of two **identical** populations — every rejection is a mistake:

```
  p < 0.05 in 102 of them (5.1%)
```

Exactly as designed. **The 5% is not an accident to be minimised** — it is the
rate you chose when you picked the threshold. A test that never rejected a true
null would never detect anything either.

Under the null, p-values are uniform:

```
  [0.0, 0.1) |███████████████████ 305
  [0.4, 0.5) |██████████████████ 292
  [0.9, 1.0] |███████████████████ 301
```

Every value equally likely. **A p-value of 0.5 is not evidence of no effect** —
it is what half of all null experiments produce.

### The error nobody reports

```
  effect (sd)     n     power
       0.25      12      9.7%
       0.25     100     42.7%
       0.50      12     23.3%
       0.50      30     47.8%
       1.00      12     64.8%
       1.00      30     96.3%
```

A real half-standard-deviation effect with 12 per group is detected **less than a
quarter of the time**. Three experiments in four come back negative although the
effect is real, and get written up as "no significant difference".

> The 5% false positive rate is fixed by the threshold and reported in every
> paper. The false negative rate depends on effect and sample size, is usually far
> larger, and is almost never mentioned.

### When assumptions fail

```
  normal, equal spread                     6.1%
  normal, one group 5x wider               6.9%
  heavily skewed (log-normal)              1.5%
  the same data, log transformed           6.0%
```

Unequal spreads barely move it — this is Welch's t-test, which does not assume
equal variances.

The skewed row fails in **the opposite direction from the usual warning**: 1.5%,
far *below* nominal. The test has become conservative, not liberal. It is not
producing false alarms; it is throwing away power, so real effects go undetected.
Log-transforming restores it to 6.0%.

The lesson is not "the t-test is fragile" but that a skewed measurement is usually
asking to be analysed on a different scale — and moving it there fixes the test
and the interpretation at once.

---

## 6.2 Twenty thousand tests at once

**Run:** `bl run 02-multiple-testing.bl`

10,000 genes, **no real differences anywhere**:

```
  genes with p < 0.05: 508
```

Five per cent of 10,000 is 500, and that is what we got. Every one is a false
positive. Publish that and it is 500 genes, a pathway analysis, and a story.

Now with 300 genes genuinely changed:

| method | called | real | false | false share | found |
|---|---|---|---|---|---|
| uncorrected p < 0.05 | 762 | 297 | 465 | **61.0%** | 99.0% |
| Bonferroni | 24 | 24 | 0 | 0.0% | 8.0% |
| Benjamini–Hochberg | 222 | 211 | 11 | **5.0%** | 70.3% |

**Uncorrected** finds nearly all the truth and drags in 465 false genes — three
in five of its calls are wrong, which makes the list useless *as a list* even
though it contains almost everything real.

**Bonferroni** multiplies every p-value by the number of tests, controlling the
probability of making *even one* mistake. Nothing on its list is spurious; it
finds one real gene in twelve.

**Benjamini–Hochberg** controls a different quantity — not the chance of any
error, but the expected *proportion* of errors among the genes you call. Note what
it actually delivered: **5.0% false, exactly its nominal level**, while recovering
70% of the truth. That is not a lucky seed; it is the quantity the method
controls, hitting its target.

### The choice is about consequences

```
  Bonferroni  ->  is ANY of this wrong?     (one confirmatory experiment)
  BH / FDR    ->  what FRACTION is wrong?   (a candidate list)
```

Neither is a default. Choosing before you look at the data is what keeps the
answer honest.

---

## What to take away

1. **A test is a decision rule with a known error rate** — not a verdict on
   reality.
2. **Under the null, p is uniform.** A large p-value is not evidence of absence.
3. **Power is the unreported error.** Most published negative results from small
   samples carry no information.
4. **Skew made the t-test conservative, not liberal** — it lost power rather than
   inventing findings, and a transform fixed it.
5. **At genome scale, 5% of nothing is still 500 genes.**
6. **Bonferroni and FDR answer different questions.** Pick by what the list is
   for.

## Notes on BioLang

Chapter 6 is the best-supported chapter in this companion: `ttest`,
`ttest_paired`, `wilcoxon`, `ks_test`, `anova`, `fisher_exact`,
`permutation_test`, `p_adjust` and `power_t_test` are all builtins, and
`p_adjust(pvals, "bonferroni" | "fdr")` does what its name says.

Two defects surfaced and were fixed while writing it:

- **`fisher_exact` aborted the process.** The lower bound for the hypergeometric
  should be `max(0, row1 + col1 - n)`; the guard tested `row1 < col1` instead, so
  a 2×2 table like (10, 5, 3, 12) computed `28 - 30` on an unsigned integer.
  Debug builds panicked; release wrapped to ~1.8e19 and looped over an
  essentially unbounded range. Now verified against R on published tables:
  Fisher's tea-tasting gives 0.4857 and (1, 9, 11, 3) gives 0.002759.
- **Five documented return types were wrong.** `bl metadata` advertised
  `ttest(...) → Record{t,p,df}`; the fields are `statistic`, `p_value`, `df`,
  `mean_diff` — there is no `t` and no `p`. The same for `ttest_one`,
  `ttest_paired`, `anova` and `lm`. Corrected.

## Exercises

1. Rerun 6.1's power table with Wilcoxon instead of the t-test. Where does each
   win?
2. In 6.2, vary the number of truly-changed genes from 10 to 3000. At what point
   does Bonferroni stop being hopeless?
3. The uncorrected list contained 99% of the truth. Construct a situation where
   that list is the right one to use.
4. BH assumes the p-values are independent. Make the genes correlated and check
   whether the realized false share still lands on 5%.
