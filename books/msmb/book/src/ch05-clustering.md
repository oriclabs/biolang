# 5. Clustering

Following Chapter 5 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch05/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch05)

---

## The idea in one paragraph

Clustering asks: are there groups here, and which points belong to which? Note
what is absent — no labels, no training set, no right answer to check against.
That absence is the whole difficulty, and it is why every clustering method
produces confident output on data with no structure at all. The useful skill is
not running the algorithms; it is telling the two situations apart.

---

## 5.1 k-means, and the two questions it cannot answer

**Run:** `bl run 01-kmeans.bl`

180 points in three genuine groups. k-means with k = 3 recovers them. Agreement
with the truth is measured on *pairs* — do two points that belong together end up
together? — because cluster labels are arbitrary and pairs are invariant to
relabelling.

### It always answers

The same call on a single structureless Gaussian blob:

```
  |x    x x    xxx   x  ++++ + ++ + + +          |
  |   x   xxx      xxxx    +++ ++                |
  |      x         x  x   +        +             |
```

Three tidy wedges, none of them real. **k-means partitions; it does not test
whether a partition is warranted.** Nothing in its output can say "this data has
no groups", because it has no way to express that.

### It cannot tell you k

```
  k    within-cluster sum of squares
  1        10984.47
  2         1506.52
  3          352.71
  4          316.57
  5          270.46
```

The score falls at every k and reaches zero when k equals the number of points —
a perfect, useless fit. You cannot choose k by minimising it. Here the fall from
2→3 is large and 3→4 tiny, which points at 3 and is correct. On real data the
elbow is often not obvious, and reading one into an ambiguous curve is how
clustering results become unreproducible.

---

## 5.2 Hierarchical clustering

**Run:** `bl run 02-hierarchical.bl`

k-means must be told k up front. Hierarchical clustering builds the entire
nesting — every point separate, through to everything in one group — and you
choose where to cut afterwards, or never.

BioLang has no `hclust`, so it is written out. The algorithm is one idea: every
point its own cluster, repeatedly merge the two closest, stop at one.

**"Closest" is the decision that matters.** Two points have one distance; two
*groups* have many:

| linkage | between-group distance |
|---|---|
| single | the closest pair |
| complete | the furthest pair |
| average | the mean over all pairs |

Same data, same algorithm, different trees. Not a knob to leave at its default.

On eight tissues with three-dimensional expression profiles, average linkage:

```
   0.69 --- heart + lung
   0.71 --- liver + kidney
   0.84 --- (liver kidney) + spleen
   0.87 --- (brain cortex) + cerebellum
   6.23 ------------------------- ((liver kidney) spleen) + (heart lung)
   8.94 ------------------------------------ (abdominal) + (brain)
```

All three rules recover the same biology here because the groups are well
separated. The heights differ systematically — single lowest, complete highest,
average between — which is definitional, not a finding.

Where they genuinely disagree is on elongated or touching groups: single linkage
chains clusters together through one bridging point, complete linkage breaks
elongated clusters apart. Neither is wrong; they encode different ideas of what a
cluster *is*.

### Where to cut

```
  cut at height  0.7  ->  6 groups
  cut at height  0.8  ->  5 groups
  cut at height  0.9  ->  3 groups
  cut at height  3.0  ->  3 groups
  cut at height  7.0  ->  2 groups
```

A dendrogram is not a clustering — it is every clustering at once. Notice the
answer does not move between 0.9 and 7.0. That flat stretch is the signal: it
corresponds to the gap between the last cheap merge (0.87) and the first
expensive one (6.23). **A clustering that survives a wide range of cuts is worth
more than one appearing at exactly one threshold.** Where merge heights rise
smoothly with no gap, no cut is defensible — and a dendrogram will still happily
draw one.

---

## 5.3 Are the clusters real?

**Run:** `bl run 03-are-the-clusters-real.bl`

Given that every method always answers, this is the section that matters.

### Silhouette

For each point, compare the mean distance to its own cluster (`a`) against the
mean distance to the nearest other cluster (`b`): `s = (b - a) / max(a, b)`. Near
1 is comfortably inside; near 0 is on the border; negative means it sits closer to
another cluster than its own.

```
   k    three real groups    one uniform blob
   2          0.540               0.350
   3          0.716               0.332
   4          0.568               0.327
   5          0.459               0.300
```

The structured data **peaks sharply at the correct k = 3, and peaks high**. The
blob peaks too — something always wins — but low and flat. That contrast is the
whole point: ~0.7 means well-separated groups; ~0.3 means the partition is barely
better than cutting arbitrarily, which is what it is.

### Stability

Real groups should not depend on which samples you happened to collect. Drop a
random fifth, re-cluster, and check whether the survivors are grouped the same:

```
   k    three real groups    one uniform blob
   3          1.000               0.896
```

Real groups reproduce perfectly. The blob is lower but not dramatically — and the
reason is worth stating rather than overselling the contrast: **most pairs of
points sit in different clusters under any partition, and those agree
trivially**, inflating the score toward 1 for any k. Stability is the weaker of
the two checks as measured here; a better version would count only the pairs
actually at risk of moving.

### The habit

Neither number is a p-value and neither has a threshold worth quoting. They are
comparisons — this clustering against what data with no groups would give.

That is Chapter 1's lesson in different clothes: **before believing a pattern,
generate the boring version, run the same procedure on it, and check that what you
found looks different from what noise produces.**

---

## What to take away

1. **Clustering algorithms cannot decline to answer.** Confident output is not
   evidence of structure.
2. **The within-cluster score cannot choose k** — it always improves.
3. **Linkage choice is a modelling decision**, encoding what you think a cluster
   is.
4. **A dendrogram is every clustering at once.** The cut is yours, and one stable
   across a wide range of cuts is the trustworthy kind.
5. **Always compare against structureless data** run through the identical
   pipeline.

## Notes on BioLang

`kmeans(points, k)` returns `{clusters, centroids, iterations, inertia}` — note
`clusters`, not `assignments`. There is no `hclust`, `dendrogram`, `cutree` or
`silhouette`, so 5.2 and 5.3 write the linkage rules and the silhouette out by
hand.

One gap worth knowing: BioLang has no way to insert a computed key into a map —
`merge` needs literal keys — so the ASCII plots look up grid cells with
`find_index` over a list instead. Workable at this size; it would not scale.

## Exercises

1. Make two elongated, nearly-touching clusters and compare single against
   complete linkage. Which recovers them?
2. Run k-means twenty times on the blob with different seeds. How much does the
   partition change, and what does that say about reporting one run?
3. The silhouette peaked at the correct k here. Move the three centres closer
   until it stops doing so — at what separation does it fail?
4. Devise a stability measure counting only pairs at risk of moving, and check
   whether it separates the two cases better than the one in 5.3.
