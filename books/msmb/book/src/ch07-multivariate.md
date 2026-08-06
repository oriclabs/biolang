# 7. Multivariate Analysis

Following Chapter 7 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch07/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch07)

---

## The idea in one paragraph

Measure twenty things about each sample and you cannot look at the data. PCA
finds the directions along which samples differ most, so a few numbers can stand
in for many. It is the most used method in genomics and the most often misread —
mostly because it always produces a picture, and the picture always looks like it
means something.

---

## 7.1 Principal components

**Run:** `bl run 01-pca.bl`

18 samples in three groups, six measured genes — but built from only **two**
underlying biological axes, so the genes are not independent:

```
  correlation between genes:
              1      2      3      4      5      6
  gene 1   1.00   0.98   0.95   0.44   0.48   0.08
  gene 4   0.44   0.53   0.24   1.00   0.97   0.89
  gene 6   0.08   0.20  -0.09   0.89   0.86   1.00
```

Genes 1–3 move together, 4–6 move together. **Six measurements, two stories** —
and that redundancy is exactly what PCA exploits.

```
  component   share   cumulative
  PC1          69.3%     69.3%
  PC2          28.8%     98.1%
  PC3           0.9%     99.0%
```

Two components carry 98%, which is right — we built the data from two axes. The
scree plot flattening after PC2 is how you would have discovered that without
knowing.

Projected onto the first two components, the three groups separate cleanly —
**and nothing told PCA the labels**. It found the directions of greatest
variation, and the group structure happened to lie along them.

```
  |           BB B                                     |
  |             B                                      |
  |                                             A  A   |
  |                                                   A|
  |  C                                                 |
  |CC                                                  |
```

## Three things PCA is not

**1. It is not a clustering.** It produced coordinates, not groups. The
separation above is something your eye did, not something the method asserted.

**2. It does not know what is interesting.** It maximises *variance*, and
variance is not importance. Adding a strong alternating processing effect to the
same data:

```
  PC1          96.7%      (was 69.3%)
  mean PC1 for even-numbered samples:   14.61
  mean PC1 for odd-numbered samples:   -14.61
```

PC1 is now the **batch**, separating samples by processing order rather than by
tissue — and the plot looks every bit as convincing as the real one. The only way
to notice is to colour the points by something other than the variable you hope
to see.

**3. Its axes are not measurements.** PC1 is a weighted blend of all six genes.
Asking "what is PC1?" has no answer beyond that list of weights, which is why
"PC1 correlates with survival" is a much weaker statement than it sounds.

---

## What to take away

1. **PCA works because measurements are correlated.** With independent variables
   it has nothing to compress.
2. **The scree plot tells you the dimensionality**, and flattening is the signal.
3. **Separation in a PCA plot is not a clustering result** and carries no p-value.
4. **The largest source of variance is often technical.** Always colour by batch,
   date and operator before believing the biology.
5. **Components are blends, not quantities.** They do not inherit the
   interpretability of the things they combine.

## Notes on BioLang

`pca(matrix)` returns `{components, explained_variance, explained_variance_ratio,
transformed}`.

Two traps found while writing this:

- **`explained_variance` is raw variance, not a share.** Treating it as a
  proportion gave "PC1 explains 1577%". The proportion is
  `explained_variance_ratio`.
- **`transformed` is a `Matrix`, not a list of lists**, so `scores[i][0]` fails
  with `cannot index Matrix with Int`. Use `matrix_at(m, i, j)`.

Neither is a defect exactly, but the documented signature —
`pca(data, [n]) → Record{explained_variance,...}` — stops at the first field and
mentions neither.

## A note on Chapter 9

Holmes & Huber's Chapter 9 extends this to heterogeneous data, where the methods
are correspondence analysis, canonical correlation and multidimensional scaling.
BioLang has none of `cca`, `mds` or a correspondence analysis, and writing three
substantial matrix algorithms out by hand would produce a chapter about linear
algebra rather than about biology. It is not attempted here.

## Exercises

1. Generate six genuinely independent genes and run PCA. What does the scree plot
   look like, and what does that tell you about when PCA helps?
2. Scale one gene by 1000 and re-run. Does PC1 change, and what does that say
   about standardising before PCA?
3. In the batch example, remove the batch effect by centring each batch
   separately, then re-run. Does the biology come back?
4. Colour the batch-contaminated projection by tissue rather than batch. How
   convincing is the wrong picture?
