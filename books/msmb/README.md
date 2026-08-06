# Modern Statistics for Modern Biology, in BioLang

A BioLang companion to Susan Holmes and Wolfgang Huber's *Modern Statistics for
Modern Biology* (Cambridge University Press, 2018), free to read at
<https://www.huber.embl.de/msmb/>.

The examples and statistical arguments are theirs. The prose and BioLang code
here are original — see [book/src/attribution.md](book/src/attribution.md).

## Layout

```
book/src/              the written companion (mdBook)
code/ch01/             runnable BioLang, one script per section
code/ch02/
code/packages/msmbstats/   shared implementations (a local BioLang package)
```

## Running it

Nothing to download, no network:

```bash
cd code/ch01 && bl run 01-poisson.bl        # ...through 06-power-simulation.bl
cd code/ch02 && bl run 01-likelihood-poisson.bl   # ...through 05-hardy-weinberg.bl
```

Every script seeds its own generator, so the numbers are the same on every
machine. `code/packages/msmbstats/src/verify-samplers.bl` cross-checks the two
multinomial samplers against each other.

## Building the book

```bash
mdbook build book
```

## Status

Chapters 1-8, 10 and 13, as 28 runnable scripts. Every script runs offline with no
downloads.

Not attempted, and why:

| chapter | reason |
|---|---|
| 9 Heterogeneous data | BioLang has no `cca`, `mds` or correspondence analysis; writing three matrix algorithms by hand would make it a chapter about linear algebra |
| 11 Image data | no image I/O or segmentation at all |
| 12 Supervised learning | no random forest, LDA, SVM or cross-validation |

## Defects this found in BioLang

Ten, all fixed, all with regression tests:

| defect | symptom |
|---|---|
| `gamma_cdf` | every chi-square p-value wrong - chi2=2, df=1 gave 0.000000 (true 0.1573). Shared by six other tests |
| `choose` | `choose(300,40)` gave 3.457e36; true 9.793e49. Silent in release, panic in debug |
| pipe dispatch | `x \|> user_fn()` ~1750x slower than `user_fn(x)` |
| `fisher_exact` | aborted the process on ordinary 2x2 tables via unsigned underflow |
| recursion | ran out of stack at depth ~120 and killed the process with no BioLang error |
| stack traces | 400-frame dumps buried the error message |
| `upgma` | emitted `:0` for every branch length, discarding the height it had just computed |
| `power_t_test` | used the one-sample formula, advising experiments at half the size they need - 32 per group where the answer is 64 |
| interpreter stack | ~477 KB per BioLang call; two 200-line match arms were charging every recursive evaluation for locals it never used |
| metadata | six documented return types wrong (`scatter`, `ttest`, `ttest_one`, `ttest_paired`, `anova`, `lm`) |

The chi-square one is the instructive case: a test existed and asserted only that
the *statistic* exceeded 3.0. The p-value - the number every caller uses - was
never checked.

## Tests

```bash
node scripts/check-msmb.mjs      # from the repository root
```

Runs all 30 scripts and fails if any does. The scripts `assert` the figures this
README and the prose quote - `dmultinom` to ten decimals, the C. elegans
goodness of fit, Hardy-Weinberg expected counts, the staph composition - so a
change to `gamma_cdf`, `pca`, `upgma` or the multinomial sampler cannot move a
published number while the prose quoting it stays put.

Wired into `ci.yml` and the pre-push hook. Verified to fail: changing one
genotype count from 188 to 187 breaks the Hardy-Weinberg assertion, as it should.

Every script is seeded, so this is deterministic. A failure means either a real
regression or a figure that legitimately moved - in which case **the prose needs
updating too**, which is the point.

## Verified against the source

Where the book publishes a number, this companion reproduces it:

| quantity | book | here |
|---|---|---|
| `dmultinom(c(4,2,0,0), rep(1/4,4))` | 0.003662109 | 0.003662109375 |
| *C. elegans* chrM goodness of fit | 4386.634 | 4386.6 |
| `poismax(0.5, 100, 7)` | 0.0001002329 | 0.000100 |
| 95th percentile of the null statistic | 7.6 | 7.600 |
| power at n = 20 | 0.199 (199/1000) | 0.194 (194/1000) |
| Poisson MLE for the epitope data | 0.55 | 0.55 |
| *S. aureus* mean composition `p0` | 0.3470531, 0.1518313, 0.2011442, 0.2999714 | identical to 7 dp |
| staph tables as extreme as observed | 0 of 1000 | 0 of 1000 |
| Chargaff permutation p-value | 0.00019 (19/100000) | 0.00017 (17/100000) |
| Hardy-Weinberg allele frequency | 0.5793103 | 0.5793103 |
| expected MM / MN / NN | 194.6483, 282.7034, 102.6483 | identical |

Chapter 3 uses generated data, so there is nothing to match against the book.
Its one external check is Anscombe's quartet, which reproduces the published
summaries: mean x 9.00, mean y 7.50, r 0.816, slope 0.500, intercept 3.00,
identical across all four datasets.

The first four are deterministic and match. The last is a simulation with an
independent random number generator, so agreeing to 0.005 is what success looks
like there.

The book leaves choosing an adequate sample size as an exercise;
`06-power-simulation.bl` answers it directly — power reaches 0.865 at n = 100
and 0.993 at n = 200.
