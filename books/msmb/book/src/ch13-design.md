# 13. Design of Experiments

Following Chapter 13 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch13/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch13)

---

## The idea in one paragraph

Every chapter so far analysed data that already existed. This one is about the
choices made *before* any data exists — and they matter more, because no analysis
can repair a design that threw the information away. Three failures account for
most of it: confounding, mistaking the unit of replication, and running an
experiment too small to answer its own question.

---

## 13.1 Confounding: the failure no method can fix

**Run:** `bl run 01-design.bl`

A study runs all treated samples on Monday and all controls on Tuesday.
Sequencing runs differ — reagents, machine drift, whoever was on shift. Each
measurement now carries a treatment effect and a day effect added together, and
nothing in the data says which is which.

```
  true treatment effect: 1.0
  true batch effect:     2.5
  measured difference:   3.512
  p-value:               0.000003
```

Highly significant, and the estimate is the **sum of the two effects**. The test
is not wrong — it faithfully reports that Monday samples differ from Tuesday
samples. It was never asked about treatment, because the experiment made those
two questions the same question.

### The fix is in the design

Run half of each group on each day:

```
  measured difference:   1.043   (true value 1.0)
  p-value:               0.031
  paired p-value:        0.00000042
```

The estimate now lands on the real effect. The batch effect is still there and
still large — it just no longer masquerades as treatment, because it moves both
groups together. Pairing removes it from the comparison entirely, and the paired
test is sharper by four orders of magnitude.

> Same data, a better test, **because the design licensed it**. Design decisions
> buy statistical power that no later cleverness can recover.

---

## 13.2 What counts as a replicate

Two mice, each sequenced four times, is **not** eight replicates. The four runs
from one mouse share everything that makes that mouse itself.

800 experiments with no real difference between groups:

```
  treating 8 runs as 8 replicates:  significant 49.4% of the time
  averaging each mouse first (2v2): significant  4.0% of the time
```

Nominally 5%. Technical runs are nearly identical to each other, so the test sees
a tiny standard error and declares a difference whenever the two *mice* happened
to differ — which is most of the time. **Half of all such experiments produce a
false positive.**

Averaging per mouse gives the correct rate, and an appropriately underpowered
experiment — which is the honest description of two animals per group.

> Technical replicates improve the measurement of each animal. They say nothing
> about animals in general. **The unit of replication has to match the unit you
> want to generalise to.**

---

## 13.3 Deciding the size beforehand

```
  effect (sd)   power 0.80   power 0.90
          0.3          175          234
          0.5           63           85
          0.8           25           33
          1.0           16           22
```

Read it backwards from the power you need. A half-standard-deviation effect at
the conventional 80% power takes **63 samples per group**; most published
experiments of that size have far fewer.

This calculation costs nothing and can only be done beforehand. Run the
experiment first and the same arithmetic becomes a *post-hoc* power analysis,
which answers a question nobody asked and is not evidence of anything.

---

## What to take away

1. **Confounding is not a statistical problem**, it is a design problem, and it
   is unfixable after the fact.
2. **Block and randomise.** A nuisance variable applied equally to both groups
   stops being a rival explanation.
3. **Pairing is free power** when the design supports it.
4. **Count animals, not measurements.** Pseudo-replication took the false
   positive rate from 5% to 49%.
5. **Size the experiment before running it**, and treat a post-hoc power
   calculation as the non-answer it is.

## Notes on BioLang

`power_t_test`, `power_analysis`, `anova`, `batch_correct` and `ttest_paired` all
exist. `power_t_test(effect, alpha, power)` returns a record — it computes the
required **sample size**, not the power, despite the name.

**One defect found and fixed.** It used `n = ((z_α/2 + z_β)/d)²`, the *one-sample*
formula, and returned 32 per group for `d = 0.5` at 80% power where the answer is
64. Comparing two groups estimates two means, so the difference carries twice the
variance and the formula needs a factor of 2. A sample-size calculator that
advises experiments at half the size they need is worse than none, since it
supplies a number to put in the methods section. Now within one of R's
`power.t.test` across the range, with tests pinning it there.

## Exercises

1. In 13.1, make the batch effect 0.2 instead of 2.5. Is the confounded estimate
   still misleading, and would you have noticed?
2. Vary the mice-to-runs ratio in 13.2. At what point does pseudo-replication
   stop mattering, and why is that the wrong question to ask?
3. `power_t_test` uses a normal approximation, so it gives 63 where R's t-based
   calculation gives 64. At what sample size does that gap matter?
4. Design an experiment that is confounded with something you cannot randomise —
   age, sex, hospital. What can you still conclude?
