# 2. Statistical Modeling

Following Chapter 2 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch02/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch02)

---

## The idea in one paragraph

Chapter 1 always handed us the parameter. "The background rate is 0.5" arrived
from calibration and we took it on faith. Real work almost never does that: you
have data, and the parameter is what you are missing. This chapter turns the
question around — instead of *given the model, how likely is this data?*, it
asks *which model makes this data least surprising?* That is *maximum
likelihood*, and it is the single most reusable idea in statistics.

---

## 2.1 Maximum likelihood, on the Poisson

**Run:** `bl run 01-likelihood-poisson.bl`

The epitope array again, now as its tally: 58 positions saw nothing, 34 saw one
hit, 7 saw two, 1 saw seven.

If `lambda` were true, the probability of *this whole dataset* is the product of
each observation's probability:

```biolang
fn likelihood(lambda) {
    range(0, len(values))
        |> map(|i| pow(dpois(values[i], lambda), counts[i]))
        |> reduce(|a, b| a * b, 1.0)
}
```

```
lambda = 3.0  ->  1.2e-56
lambda = 0.4  ->  3.6e-45
```

Both are absurdly small — *any* particular dataset is unlikely. Only the
**comparison** carries meaning, and 0.4 beats 3.0 by eleven orders of magnitude.

### Why logs, always

A hundred probabilities multiplied together underflow to zero on any computer.
Taking logs turns products into sums, tiny numbers into manageable negatives,
and — crucially — **does not move the maximum**, because `log` is monotonic.
Whatever maximises the likelihood maximises the log-likelihood.

That is why you will never see the likelihood itself in practice.

### The punchline

```
grid maximum at lambda = 0.55
sample mean = 55 hits / 100 positions = 0.55
```

For a Poisson, the maximum likelihood estimate **is** the sample mean. Not a
coincidence, not a rule of thumb — it drops out of setting the derivative of the
log-likelihood to zero.

The value of the exercise is the *method*, not the answer. Here the answer was
already obvious. The same procedure keeps working when it is not.

### The part usually skipped

```
  loglik at the maximum   : -101.2582
  loglik at lambda = 0.45 : -102.2951
  loglik at lambda = 0.65 : -102.0703
```

Move a tenth either way and almost nothing happens. The data pin `lambda` down
to about ±0.1 and no better. **A point estimate with no sense of its own
flatness is half an answer** — and that flatness is where standard errors and
confidence intervals come from.

---

## 2.2 The same method on a binomial

**Run:** `bl run 02-likelihood-binomial.bl`

120 men screened for colour blindness, 10 affected. Change the model, keep the
method:

```
grid maximum at p = 0.085
sample proportion = 10 / 120 = 0.0833
```

(The grid steps by 0.005, which is why it lands on 0.085 rather than exactly
1/12.) Again the estimate is the obvious summary of the data.

### Sample size is curvature

Two studies, both estimating a rate. How far does the log-likelihood fall 0.03
above the estimate?

```
  10 of 120        0.585
  40 of 300        1.042
```

The larger study's curve is **steeper**, so the same displacement costs more
likelihood. A sharp peak means the data exclude nearby values; a flat one means
they do not. The estimate alone never tells you which situation you are in.

> **Watch out.** The binomial log-likelihood is
> `log C(n,y) + y log p + (n-y) log(1-p)`. The first term contains no `p`, so it
> shifts the curve without moving its peak — drop it. Keeping it means computing
> `C(300, 40)` ≈ 10^50, which overflows. Anything *proportional* to the
> likelihood works; only differences matter.

---

## 2.3 Do ten genes share one composition?

**Run:** `bl run 03-staph-nucleotide-bias.bl`

Base counts of the first ten genes of *Staphylococcus aureus* MW2. Average each
gene's proportions to get one candidate composition for all ten — the null
hypothesis:

```
average (p0):  0.3471  0.1518  0.2011  0.2999
```

Strongly AT-rich, and the ten rows look broadly similar. *"Broadly similar"* is
exactly the kind of judgement that needs a number.

Give each gene its real length but assume it draws bases from the shared `p0`,
then sum the chi-square contribution over all 40 cells:

```
The statistic for the real table: 70.14
```

Is that big? **The number alone cannot say.** So manufacture ten fake genes with
the real lengths, drawing every base from `p0` — by construction the null holds
— and see how large the statistic gets by chance:

```
  median:   29.39
  95th %:   43.96
  maximum:  58.29

simulated tables at least as extreme as 70.1: 0 of 1000
```

None in a thousand. The ten genes do **not** share one composition.

> **Zero is not a p-value.** "0 of 1000" means *smaller than this experiment can
> measure*, not zero. Reporting `p = 0` is always wrong; report `p < 1/1000`, or
> simulate more.

### A free check worth stealing

Theory says this statistic follows χ² with degrees of freedom equal to the free
cells: 40 cells minus the 10 gene lengths held fixed = 30. A χ² with `d` degrees
of freedom has mean exactly `d`:

```
  degrees of freedom:              30
  mean of the simulated statistic: 30.07
```

Nothing in the code arranged that. When a simulation independently reproduces a
number theory predicts, both are probably right — a cheap check, worth making
whenever one is available.

---

## 2.4 Chargaff's rule, and statistics without a distribution

**Run:** `bl run 04-chargaff-permutation.bl`

Erwin Chargaff's base composition measurements, from the late 1940s — years
before the double helix:

```
  species            A - T    C - G
  Human-Thymus         1.5      0.1
  Mycobac.Tuber        0.5     -0.5
  Sea Urchin           0.7      0.4
  E.coli               1.1      0.3
```

A tracks T and C tracks G, in every species, across compositions ranging from
15% A to 33% A. *Mycobacterium* is nothing like sea urchin overall — yet within
each, the pairs match. This is the observation that made base pairing the
obvious explanation.

### Testing a pattern nobody has tabulated

Invent a statistic that is small when the pattern holds:

```biolang
fn chargaff_stat(table) {
    table |> map(|r| pow(r[0] - r[1], 2) + pow(r[2] - r[3], 2)) |> sum()
}
// observed: 11.08
```

Small — but compared to *what*? There is no textbook distribution for this. We
invented it ten lines ago.

**So build the comparison.** Shuffle each species' four numbers among
themselves:

```
  minimum:     11.08
  median:    1557.16
  maximum:   2220.94
  observed:    11.08

shuffled tables at least as low: 17 of 100000
p-value: 0.00017
```

The observed table is essentially the *best possible* arrangement of its own
numbers.

> **Permuting within rows is the whole design.** Every species keeps its own four
> values, so overall composition, totals and the spread across species survive
> untouched. The only thing destroyed is which base sits in which column —
> exactly the claim under test. Choosing what to hold fixed *is* the hypothesis.

This test needed no distribution theory at all. When your statistic has no name,
you can still calibrate it, and that freedom is worth more than any table.

---

## 2.5 Hardy-Weinberg: estimating an allele frequency

**Run:** `bl run 05-hardy-weinberg.bl`

580 people from Tahiti, typed for the MN blood group: 188 MM, 296 MN, 96 NN.

### The step that trips people

We observe 580 **people** but the parameter is about **alleles**, and there are
twice as many:

```
  M alleles: 2 x 188 (from MM) + 296 (one from each MN) = 672
  total alleles: 1160
```

If individuals pair at random with respect to this gene, a genotype is two
independent draws from the allele pool:

```
    P(MM) = p²      P(MN) = 2pq      P(NN) = q²
```

The 2 is there because MN and NM are the same genotype — the same reason a
binomial coefficient appears in Chapter 1.

### Maximum likelihood, and the shortcut

```
grid maximum at p = 0.58
M alleles / total alleles = 672 / 1160 = 0.5793103
```

For the third time, maximum likelihood recovers the obvious estimator — here,
just counting alleles.

### Testing the model rather than the parameter

Estimating `p` *assumed* Hardy-Weinberg. That assumption deserves its own check,
because the interesting biology is in its failures:

```
  genotype   observed   expected
  MM              188   194.6483
  MN              296   282.7034
  NN               96   102.6483

distance: 1.2831   ->   p-value 0.5242
```

Comfortably inside what random mating produces. Hardy-Weinberg is not rejected.

Which is the outcome worth being able to recognise, because the model *failing*
is the signal: population structure, inbreeding, selection at the locus, or —
most often in practice — a genotyping error.

---

## What to take away

1. **Maximum likelihood is one procedure, not one formula.** Poisson, binomial,
   Hardy-Weinberg — same three steps every time.
2. **Always work in logs.** Products underflow; the maximum does not move.
3. **The peak's sharpness matters as much as its location.** Flat likelihood,
   weak data — regardless of how confident the point estimate looks.
4. **Anything proportional to the likelihood will do.** Constants that do not
   contain the parameter can be dropped, and often must be.
5. **A statistic with no named distribution is not a problem.** Permute or
   simulate, and calibrate it yourself.
6. **What you hold fixed when permuting is the hypothesis.** Get that wrong and
   the p-value answers a question you did not ask.
7. **Fitting a model and testing that model are different jobs.** Do both.

## Notes on BioLang

Three defects surfaced while writing this chapter. All have since been fixed,
and the notes are kept because the failures are instructive.

- **`chi_square` returned wrong p-values.** The statistic was correct; the
  p-value was not — χ²=2 on 1 df came back as 0.000000 where the answer is
  0.1573, and χ²=20 on 3 df as 0.114 where it is 0.00017. Large statistics
  returning large p-values inverts every conclusion drawn from them. `gamma_cdf`
  used the wrong Poisson identity for integer shapes and omitted the `1/Γ(a)`
  normalisation otherwise; six other tests shared the same CDF. The existing
  test asserted only that the *statistic* exceeded 3.0, which is why it survived.
- **`choose` overflowed silently.** `choose(300, 40)` returned 3.457e36 where the
  answer is 9.793e49 — wrong by thirteen orders of magnitude, with no error,
  because the accumulator was i128 and the value is not.
- **`x |> user_fn()` ran ~1750× slower than `user_fn(x)`.** The pipe reaches a
  function through a path that probed the environment with a lookup that runs a
  "did you mean?" search over every bound name when it misses — which it always
  did. Nine milliseconds, once per call.

Every p-value in this chapter is simulated regardless, which is the chapter's
own method and, as it turned out, the reason none of the results depended on the
broken one.

## Exercises

1. In 2.1, cut the dataset to its first 20 positions. How much flatter is the
   log-likelihood? Estimate the sample size needed to pin `lambda` to ±0.05.
2. Section 2.3 reports "0 of 1000". Raise the count until you get a non-zero
   tally, or argue why you never will.
3. Redo Chargaff permuting *columns within a column* instead of within rows.
   Which hypothesis does that test, and why is it the wrong one here?
4. The Hardy-Weinberg test had p = 0.52. Construct genotype counts with the same
   `p` that would reject it, and say what biology each departure suggests.
