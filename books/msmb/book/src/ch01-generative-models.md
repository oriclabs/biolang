# 1. Generative Models for Discrete Data

Following Chapter 1 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch01/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch01)

---

## The idea in one paragraph

You run an experiment and see something striking. Is it real? You cannot answer
that by staring at the number. You answer it by writing down what *boring* would
have looked like — precisely enough that a computer can manufacture boring data
for you — and then asking how often boring produces something at least this
striking. That description of boring is a **generative model**. This chapter
builds four of them.

---

## 1.1 Counting rare things: the Poisson

**Run:** `bl run 01-poisson.bl`

A genome has millions of sites. Each mutates with tiny probability. You count
mutations. That shape — many chances, each unlikely — is so common in biology
that it gets its own distribution, controlled by a single number: `lambda`, the
average count.

```biolang
dpois(3, 5)     // 0.1403738958
```

Read it as: *if the average is 5 mutations, about 14% of genomes carry exactly
3.* Not a prediction of one number, but a probability for every possible count.

The script prints the whole distribution:

```
  k    P(X = k)
  3  0.1404  ############################
  4  0.1755  ###################################
  5  0.1755  ###################################
  6  0.1462  #############################
 12  0.0034  #
```

Two things matter here, and they are why this is the right tool:

**The peak is not an answer.** The mode sits at 4 *and* 5. A Poisson does not
say "expect 5", it says "expect somewhere around 5, and here is exactly how
vague I am about it".

**The tail never closes.** Twelve mutations has probability 0.0034 — rare, but
not impossible. This is what lets the Poisson serve as a *null* model. It never
lets you say "that could not have happened". It only lets you say "that happens
3 times in 1000 if I am right", and then you decide whether you still believe
yourself.

The script also computes `dpois` by hand, from `lambda^k · e^-lambda / k!`, and
gets the same ten decimal places. Worth doing once so the builtin stops being
magic.

---

## 1.2 One trial, then many: Bernoulli and binomial

**Run:** `bl run 02-bernoulli-binomial.bl`

The smallest experiment: one attempt, two outcomes, one probability `p`.

```biolang
rbinom(15, 1, 0.5)    // [1, 0, 1, 0, 1, 1, 0, ...]
```

`size = 1` is what makes each entry a separate trial. Set `size = 12` instead
and you get one number: how many of twelve succeeded.

Nothing about the experiment changed — only the bookkeeping. Throwing away the
*order* of the trials costs nothing when they are independent and share the same
`p`. That discarded ordering is exactly what the binomial coefficient puts back:

```
orderings with 4 successes: choose(15, 4) = 1365
probability of any one of them:            0.0001601635
product:                                   0.2186231313
dbinom(4, 15, 0.3):                        0.2186231313
```

> **A trap worth hitting on purpose.** In BioLang `2 / 3` is integer division and
> gives `0`. Written into `rbinom(12, 1, 2 / 3)` it silently becomes a coin that
> never lands heads — no error, just wrong results. Write `2.0 / 3.0`.

---

## 1.3 Why the Poisson turns up everywhere

**Run:** `bl run 03-binomial-to-poisson.bl`

The Poisson is not a separate idea. It is what the binomial *becomes* when there
are many chances and each is unlikely:

> when `n` is large and `p` is small, `B(n, p)` behaves like `Poisson(n·p)`

With n = 10,000 and p = 0.0005, the two agree to five decimals:

```
  k   binomial   Poisson    difference
  3   0.140367   0.140374   0.00000703
  5   0.175511   0.175467   0.00004388
```

This matters practically: the Poisson has one parameter where the binomial has
two, and needs no factorial of 10,000.

The script then checks it the honest way — by simulating 20,000 whole binomial
experiments and tallying. The simulated fractions wobble around the Poisson
probabilities by roughly what 20,000 draws should wobble. **That wobble is not
error.** It is the sampling variability every real experiment also has, and
measuring its size is what the rest of the chapter does.

---

## 1.4 The chapter's real lesson: where did you look?

**Run:** `bl run 04-epitope-detection.bl`

An ELISA array tests 100 positions along a protein. Even with nothing real
there, positions light up at an average rate of 0.5. You run it. One position
shows **7 hits**.

**First attempt.** How surprising is 7, given background 0.5?

```biolang
1.0 - ppois(6, 0.5)     // 0.0000010024
```

About one in a million. Case closed?

**No.** That is the answer to a question nobody asked. It is the probability
that *a particular, pre-chosen* position shows 7. But no position was chosen in
advance — 100 were scanned and then the biggest was pointed at. The question
matching what was actually done is:

> how often does the **largest** of 100 background counts reach 7?

Simulate the entire experiment 100,000 times under pure noise, and keep only the
maximum each time, because the maximum is what triggered the reaction:

```
distribution of the maximum: {1: 6, 2: 23578, 3: 60507, 4: 14230, 5: 1540, 6: 132, 7: 7}
arrays whose maximum reached 7: 7 of 100000
simulated p-value: 0.000070
```

There is a closed form too, and it agrees:

```biolang
fn poismax(lambda, n, m) {
    let epsilon = 1.0 - ppois(m - 1, lambda)
    1.0 - exp(-1.0 * n * epsilon)
}
// poismax(0.5, 100, 7) = 0.000100
```

The finding survives — 1 in 10,000 rather than 1 in a million. But the honest
number is **a hundred times larger** than the naive one, and that gap is the
whole point. It is the same arithmetic behind every multiple-testing correction
you will meet later.

**Then the sobering part.** The script sweeps the background rate:

```
  lambda   P(max of 100 reaches 7)
   0.5     0.000100
   1.0     0.008290
   1.5     0.088441
   2.0     0.364524
```

At `lambda = 0.5` a 7 is remarkable. At `lambda = 2.0` it happens in a third of
all experiments. **The entire conclusion rests on the calibration of the assay,
not on the statistics.** No amount of analysis rescues a wrong noise model.

---

## 1.5 More than two boxes: the multinomial

**Run:** `bl run 05-multinomial-dna.bl`

DNA has four outcomes, not two. The multinomial is the binomial with `k` boxes:

```
binomial      P(x) = C(n, x) · p^x (1-p)^(n-x)
multinomial   P(x) = n! / (x₁!···x_k!) · p₁^x₁ ··· p_k^x_k
```

Same structure: a coefficient counting the orderings that give the same tally,
times the probability of one such ordering.

BioLang has no multinomial, so
[`lib/multinomial.bl`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch01/lib/multinomial.bl)
writes it out — and that turns out to be the most useful part of the chapter.
It contains **two** samplers:

- `rmultinom_slow` — the definition. One categorical pick per observation.
  Obvious, and unusably slow past a few thousand.
- `rmultinom_one` — a chain of binomials. *How many A?* is binomial: A against
  everything else. Having answered that, the rest spread over C, G, T alone, so
  *how many C, out of what remains?* is binomial again with `p` rescaled to the
  shrunken pool. The last box takes the remainder.

Four binomial draws instead of 13,794 categorical ones. `lib/verify-samplers.bl`
checks they agree.

### Measuring "how far from expected"

We need one number for how unlike the expectation a tally is:

```biolang
fn chi_stat(observed, expected) {
    range(0, len(observed))
        |> map(|i| pow(observed[i] - expected[i], 2) / expected[i])
        |> sum()
}
```

Dividing each squared error by what was expected there is the important part.
Being 5 off matters enormously when you expected 5, and not at all when you
expected 5000.

### The threshold comes from the model

Draw 1000 fair sequences of 20 bases, compute the distance for each, and look:

```
  median:  2.800
  95th %:  7.600
  max:    14.400
```

Under a fair model the distance exceeds 7.6 only 5% of the time. **That
threshold was not looked up in a table.** It was manufactured from the model, at
this sample size, by simulation — which means it is still available when your
situation has no textbook formula.

### Applying it

The *C. elegans* mitochondrial genome, 13,794 bases:

```
observed A, C, G, T: [4335, 1225, 2055, 6179]
expected if equal:   [3448.5, 3448.5, 3448.5, 3448.5]
distance from flat:  4386.6
```

Against a fair model at the same sample size, that distance runs to a maximum of
about 12 in 300 simulations. The observed value is **4386**. The four bases are
not equally likely, and it is not close.

> This reproduces the book's published value of 4386.634 exactly — a useful
> check that the hand-written multinomial is right.

---

## 1.6 The question asked too late: power

**Run:** `bl run 06-power-simulation.bl`

Everything so far controlled one failure: crying wolf when nothing is there.
Nothing yet addresses the other — **something is there and the experiment misses
it**.

Power is the probability of noticing. You cannot measure it from your data. You
simulate it, and the time to do it is *before* running the experiment, while the
sample size is still a choice.

Three steps:

1. **Threshold from the null.** 1000 fair draws of 20 bases; take the 95th
   percentile of the distance → 7.6.
2. **State the alternative.** Power is never a property of a test alone, only of
   a test *against a specific departure*. Here: `[3/8, 1/4, 1/4, 1/8]`. Ask "what
   is the power?" and the honest reply is "to detect *what*?"
3. **Run the experiment 1000 times under that alternative** and count how often
   the distance clears the threshold — measuring distance from the *null*
   expectation throughout, because that is what the test does.

```
exceeded the threshold: 194 of 1000
POWER = 0.194
```

**Four experiments in five come back negative although the skew is real and
large.** A negative result from that design is not evidence of no effect. It is
no information.

The fix is sample size, and it can be priced in advance:

```
  n      power
    20   0.217
    50   0.500
   100   0.865
   200   0.993
   400   1.000
```

Both the threshold and the power are recomputed at every `n`, which is why this
is simulated rather than read off a curve.

---

## What to take away

1. **A generative model is a mechanism you can run**, not a summary of data.
2. **The question you answer must match the question you asked.** Scanning 100
   positions and reporting the best one is a different experiment from testing
   one position, and the arithmetic differs by a factor of 100.
3. **Thresholds can be manufactured by simulation**, so you are never stuck
   because your statistic has no named distribution.
4. **Power is a design decision, not an analysis result.** An underpowered
   negative is not a finding.
5. **The model's assumptions carry the conclusion.** Change the background rate
   from 0.5 to 2.0 and a one-in-ten-thousand result becomes routine.

## Exercises

1. In `04-epitope-detection.bl`, change `background` to 1.0 and rerun. At what
   count does a peak become convincing again?
2. `poismax` uses `exp(-n·epsilon)` as an approximation to `(1-epsilon)^n`.
   Compute both and find an `n` and `epsilon` where they visibly disagree.
3. `dmultinom` uses `factorial`, which overflows quickly. Find the size at which
   it breaks, then rewrite it with `log` to survive further.
4. In `06-power-simulation.bl`, keep n = 20 and make the skew progressively
   milder. Plot power against skew — what size of effect is a 20-base experiment
   simply blind to?
