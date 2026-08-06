# 10. Networks and Trees

Following Chapter 10 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch10/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch10)

---

## The idea in one paragraph

A phylogeny is a hypothesis about history drawn as a picture. Like the dendrogram
in Chapter 5, it will be produced from any data at all, and the drawing looks
equally confident either way. This chapter builds trees, then asks the question
the picture cannot answer on its own: which parts of it would survive different
data?

---

## 10.1 Distances and trees

**Run:** `bl run 01-trees.bl`

Six primate sequences, 34 aligned bases, distances counted as mismatches:

```
              human  chimp  goril  orang  macaq  mouse
  human           0      1      2      4      6     10
  chimp           1      0      1      3      5     10
  mouse          10     10      9      7      5      0
```

UPGMA recovers the expected topology:

```
(((((human:0.5,chimp:0.5):0.25,gorilla:0.75):1.25,
   (orangutan:1.0,macaque:1.0):1.0):2.1,mouse:4.1));
```

Worth checking that by hand: 0.5 + 0.25 + 1.25 + 2.1 = **4.1**, exactly mouse's
branch. Every tip sits the same distance from the root, which is UPGMA's defining
property — and a useful sanity check that the implementation is right.

### UPGMA vs neighbour-joining

UPGMA assumes every lineage changes at the same rate, so root-to-tip distance is
constant. When that holds it is efficient; when it does not — and for real
sequence data it usually does not — it places a fast-evolving lineage too far from
the root and can group unrelated fast-evolvers together.

Neighbour-joining drops the clock assumption, which is why it is the default for
sequence data. The price is that its branch lengths are no longer times; they are
amounts of change.

---

## 10.2 A tree drawn from noise

Six **unrelated random sequences**:

```
((r1:13.8,((r2:12.0,(r3:11.5,r5:11.5):0.5):1.5,(r4:13.0,r6:13.0):0.5):0.3));
```

A complete, confident, entirely meaningless tree. The method has no way to report
that the sequences share no history, because it was never asked that — it was
asked for the best tree, and it produced one.

This is Chapter 5's lesson again, and it will keep recurring: **methods that
always return an answer cannot be used to decide whether there is an answer.**

---

## 10.3 Which branches would survive different data?

Resample alignment columns with replacement, rebuild, and count how often each
grouping reappears — the bootstrap from Chapter 4, applied to a tree:

```
  human + chimp together:        42% of replicates
  great apes apart from mouse:   98% of replicates
```

The deep split is rock solid. The recent one is not, because it rests on fewer
differing columns — human and chimp differ at a single site here, so one resampled
column decides it.

**That gradient is the useful output.** A tree without support values presents its
shakiest branch and its firmest with identical confidence, and the reader has no
way to tell them apart.

---

## What to take away

1. **A tree is a hypothesis, not a measurement.**
2. **Check ultrametricity when using UPGMA** — equal root-to-tip distances are the
   assumption, and if the real data violates it the tree is wrong in a way the
   picture hides.
3. **Tree-building methods cannot decline**, so a tree is not evidence of shared
   ancestry.
4. **Bootstrap support is the part worth reporting.** Topology alone overstates
   what the data supports.

## Notes on BioLang

`upgma(labels, distance_table)` and `neighbor_joining(distance_list)` both exist —
note the asymmetry: `upgma` takes labels first and wants a `Table`, while
`neighbor_joining` takes one argument and wants a `Matrix` or `List`. The
documented signatures (`upgma(arg1, arg2)`, `neighbor_joining(arg1)`) say neither.

**One defect found and fixed.** `upgma` emitted `:0` for every branch length. It
computed the node height correctly as `min_d / 2.0`, formatted the Newick string
with a hardcoded `":0"`, and then discarded the height with `let _ = half;` to
silence the unused-variable warning. Every tree came back topologically correct
and quantitatively empty — fine if you only look at the shape, useless to anything
reading distances. Now fixed and covered by tests asserting the ultrametric
property above.

## Exercises

1. Make one lineage evolve three times faster and compare UPGMA with
   neighbour-joining. Which recovers the true topology?
2. Bootstrap the random-sequence tree. What support values does noise produce, and
   what threshold would you need to reject it?
3. Lengthen the alignment to 300 bases. How does human+chimp support change, and
   why?
4. UPGMA is average-linkage hierarchical clustering under another name. Run
   Chapter 5's hand-written version on this distance matrix and compare.
