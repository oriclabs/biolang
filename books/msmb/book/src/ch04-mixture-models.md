# 4. Mixture Models

Following Chapter 4 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch04/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch04)

---

## The idea in one paragraph

Every model so far assumed the data came from one mechanism. Real populations
are rarely so tidy — tissue holds several cell types, a tumour several clones, a
sequencing run several sources of variation. A **mixture** says each observation
comes from one of several components, chosen with some probability, and *you do
not get told which*. Recovering the components anyway is what this chapter does,
and the most important mixture in biology turns out to be one nobody draws.

---

## 4.1 When one distribution is really two

**Run:** `bl run 01-recognising-mixtures.bl`

We build a 65/35 mixture of `N(4, 1.2)` and `N(8, 1.0)`, so the truth is known.

### The mean averages; the spread does not

```
  observed mean:   5.42     weighted mean:  5.40    <- agree
  observed sd:     2.20     average within: 1.13    <- do not
```

The mean of a mixture *is* the weighted mean of its components. The spread is
not: the extra width comes from the gap **between** components, which no
component contains. Fit one normal to this and you report a standard deviation
describing a population that does not exist.

### When it stops being visible

The same mixture with the components moved progressively closer:

```
  gap  4.0 sd:  ....+++####++++++#++....
  gap  2.5 sd:  .....+++#####+#+#++.....
  gap  1.5 sd:  ......++++#+#####++.....
  gap  0.8 sd:  .......++++#####++......
```

By a gap of 1.5 the dip is gone. **The components have not merged** — the data
is still a mixture and the means still differ. What is gone is your ability to
see it. Below roughly two standard deviations of separation, no amount of
staring helps; you need more data or outside knowledge of the groups.

---

## 4.2 EM: recovering what you were not told

**Run:** `bl run 02-em-algorithm.bl`

Maximum likelihood worked in Chapter 2 because the data was complete. A mixture
breaks that circularly: to estimate a component's mean you need to know which
observations belong to it, and to know that you need the means.

**EM breaks the circle by refusing to choose.** Instead of assigning each point
to a component, it assigns a *share*.

**E step** — given parameters, how much does each observation belong to
component 2? Not yes or no, but a weight from the relative density heights:

```biolang
fn responsibility(x, mix, p) {
    let a = (1.0 - mix) * dnorm(x, p[0].mu, p[0].sd)
    let b = mix * dnorm(x, p[1].mu, p[1].sd)
    if a + b <= 0.0 { 0.5 } else { b / (a + b) }
}
```

**M step** — given those weights, re-estimate by weighted mean and weighted
variance. Every observation contributes to both components, in proportion.

Starting from a deliberately poor guess (both means near 5, both sds 2):

```
  step     mu1     sd1     mu2     sd2   weight   log-likelihood
     1   4.936   1.686   5.947   1.941    0.492    -1417.6289
     4   4.216   1.269   7.855   0.949    0.318    -1223.6607
    60   4.189   1.212   7.981   0.848    0.321     -1223.4009
converged after 68 steps
```

| | true | estimated |
|---|---|---|
| μ₁ | 4.00 | 4.189 |
| σ₁ | 1.20 | 1.213 |
| μ₂ | 8.00 | 7.982 |
| σ₂ | 1.00 | 0.847 |
| weight | 0.35 | 0.321 |

**Five parameters recovered from data carrying no labels at all.**

### What EM promises, and what it does not

Every iteration increases the likelihood or leaves it alone. That is a theorem,
not a heuristic — which is why stopping when the change gets small is legitimate.

It does **not** promise the global maximum. EM only walks uphill. Here all four
starting points reach the same answer, including one beginning with the
components 20 apart — that is what a well-separated two-component problem looks
like. Add components, add dimensions, or move the components closer and the
surface grows local maxima EM cannot escape. Serious implementations run it from
many starts and keep the best likelihood; **an EM fit reported without its
starting values is not reproducible.**

---

## 4.3 The mixture nobody draws: overdispersion

**Run:** `bl run 03-overdispersion.bl`

This is the section with the most practical consequence in the book.

A Poisson has a property that looks harmless and is not: **its variance equals
its mean.** One number fixes both.

```
  lambda    1.0  ->  mean    1.00   variance    1.02
  lambda  100.0  ->  mean   99.88   variance  100.63
```

Real sequencing counts do not behave like that. Measure one gene across
*biological* replicates — different mice, not the same sample twice — and the
variance is far larger. Poisson noise describes only the sampling step. The mice
differ.

### The fix is a mixture

Keep the Poisson for the sequencing and let its rate vary from mouse to mouse:
draw the rate from a gamma, then the count from a Poisson with that rate. That
is a mixture with infinitely many components, one per possible rate — and it has
a name:

```
  dispersion   mean   variance   var/mean
        1000  20.09      20.44       1.02      <- effectively Poisson
          20  20.03      39.53       1.97
           5  20.29      99.68       4.91
           1  19.63     409.11      20.84
```

```
variance = mu + mu²/size
```

Large `size`: the rate barely varies, and you have a Poisson. Small `size`: the
variance runs away. One extra parameter buys freedom from the mean–variance lock.

### What using the wrong one costs

400 comparisons of two groups with **identical true means** — every rejection is
a false positive — where the data is overdispersed but the test assumes Poisson:

```
  significant at 5%: 276 of 400 (69.0%)
```

A test calibrated to reject 5% of the time rejects **69%**, because it measures
the observed difference against a yardstick built from Poisson variance, which is
far smaller than the real variance. Every one of those is a gene someone would
have called differentially expressed, written up, and failed to replicate.

> This is why DESeq2 and edgeR are built on the negative binomial and spend most
> of their effort estimating dispersion. It is not a refinement. It is the
> difference between a calibrated test and one that is wrong most of the time.

---

## 4.4 The bootstrap

**Run:** `bl run 04-bootstrap.bl`

Chapter 1 built sampling distributions by simulating from a known model. Often
you have no model — just a sample, a statistic, and no theory for how it varies.

The bootstrap's idea is almost impudent: **treat the sample as the population,
and draw from it.**

For a skewed sample of 120, resampling *with replacement* 2000 times:

```
  observed median:        11.056
  standard error:          1.633
  95% interval:  [8.161, 14.510]
```

Replacement is the essential part — without it every resample is the original
reordered and the median never moves. Replacement lets some observations appear
twice and others not at all, which is how a fresh sample would differ.

The interval is read straight off the percentiles: no normality assumed, no
formula derived, and free to be asymmetric — which for skewed data it should be.

### The check, and the failure

```
  standard error of the mean:      3.229
    ...compared to sd/sqrt(n):     3.216     <- agrees, so the method works
  standard error of the median:    1.633
  standard error of the 90th pct: 10.329     <- no textbook formula
  standard error of the sd:        6.458
```

Agreement on the mean is the check. The other three have no elementary formula
and the bootstrap does not care.

But it cannot invent information the sample never held:

```
  observed maximum:            213.032
  largest bootstrap maximum:   213.032
  distinct bootstrap maxima:         7
```

Every replicate's maximum is one of seven observed values. For statistics that
depend on the extremes, the bootstrap distribution is discrete, bounded by what
you happened to see, and tells you almost nothing about the truth.

---

## What to take away

1. **A mixture's spread is not the average of its components' spreads.** The gap
   between components contributes, and no component contains it.
2. **Undetectable is not absent.** Below ~2 sd separation the mixture is still
   there; only your ability to see it has gone.
3. **EM assigns shares, not labels** — that is the trick that breaks the
   circularity.
4. **EM climbs the hill it starts on.** Report your starting values.
5. **Poisson locks variance to mean.** Biological replication breaks that lock,
   and using Poisson anyway inflated the false positive rate from 5% to 69%.
6. **The bootstrap replaces a formula you do not have** — but not information the
   sample does not contain.

## Notes on BioLang

BioLang ships `rnorm`, `rpois` and `rbinom` but nothing from the gamma family, so
`code/packages/msmbstats/src/distributions.bl` adds `rexp`, `rgamma`, `rnbinom`,
`dnbinom` and a Lanczos `ln_gamma`.

Writing `rnbinom` out is not busywork — the construction *is* Section 4.3's
argument. A negative binomial is a Poisson whose rate is itself random, and
implementing it that way makes overdispersion something you build rather than
something you are told:

```biolang
fn rnbinom_one(size, mu) {
    let rate = rgamma_one(size, mu / size)
    rpois(1, rate)[0]
}
```

Verified against theory: `size = 4, mu = 10` gives mean 9.91 and variance 35.03,
against the predicted 10 and 35; `dnbinom` sums to 1.000000 over its support;
`ln_gamma` matches published values to six decimals.

One language note: `guard` is a reserved word in BioLang, which the rejection
sampler discovered the hard way.

## Exercises

1. In 4.1, find the separation at which a histogram stops showing two humps,
   then check whether EM still recovers the right parameters below it.
2. Give EM starting values with both components at the same mean. What happens,
   and why?
3. In 4.3, re-run the false-positive experiment with `size = 20` and `size = 100`.
   At what dispersion does the Poisson test become acceptable?
4. Bootstrap the *ratio* of two medians. Does the interval contain 1?
