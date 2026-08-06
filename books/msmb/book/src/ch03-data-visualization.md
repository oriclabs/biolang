# 3. Data Visualization

Following Chapter 3 of Holmes & Huber — see [Attribution](attribution.md).

Code: [`code/ch03/`](https://github.com/oriclabs/biolang/tree/main/books/msmb/code/ch03)

---

## The idea in one paragraph

A plot is an argument, and like any argument it can be made badly without anyone
noticing. The choices that decide what a figure says — bin width, scale, whether
overlapping points are drawn or counted — are invisible in the finished picture.
This chapter is about those choices, and about why you plot at all when a table
of summary statistics is so much more compact.

All the data here is generated, so the truth is known. When you mix two
populations yourself, "does this plot reveal two populations?" has a right
answer.

---

## 3.1 One distribution, three stories

**Run:** `bl run 01-showing-a-distribution.bl`

1000 cells from two populations — one low, one high, mixed 70/30. Anyone looking
at the data should be able to discover there are two.

### The five-number summary cannot help

```
  min 1.68   Q1 3.69   median 4.43   Q3 6.60   max 10.51
```

Not one of those says "two populations". They cannot — **quantiles describe
position, not shape**. And since a boxplot draws exactly those five numbers, the
boxplot inherits the blind spot:

```
  ├────────────[────|──────────────]─────────────────────────┤
  1.68                                                   10.51
  n=1000  mean=5.01  median=4.43  Q1=3.70  Q3=6.60
```

A fine tool for comparing many groups' locations at a glance. A bad one for
asking whether a single distribution has the shape you assume.

### The histogram, and its free parameter

Bin count is not cosmetic. Same data, three times:

**5 bins — one lump:**
```
  [ 1.7,  3.4) |██████████████ 173
  [ 3.4,  5.2) |████████████████████████████████████████ 494
  [ 5.2,  7.0) |██████████ 129
  [ 7.0,  8.7) |██████████████ 185
  [ 8.7, 10.5] |█ 19
```

**25 bins — two populations, unmistakable:**
```
  [ 3.8,  4.2) |████████████████████████████████████████ 129
  [ 4.2,  4.5) |███████████████████████████████████ 116
  [ 5.2,  5.6) |███████ 25
  [ 5.6,  5.9) |███ 12          <- the gap
  ...
```

**120 bins — static.**

Unimodal, bimodal, and noise, from one dataset. Too few bins smooths real
features away; too many turns sampling noise into apparent features. **Nothing
in the figure warns you which you are looking at.**

> The practical rule: never trust one binning. Vary it, and keep only the
> features that survive.

### The plot with nothing to tune

The empirical cumulative distribution (ECDF) plots, for each `x`, the fraction of
data at or below it. Every observation is drawn, nothing is grouped, so there is
no parameter at all:

```
     x     fraction at or below
   4.0     0.352  ##############
   5.0     0.638  ##########################
   6.0     0.708  ############################   <- barely moved
   7.0     0.797  ################################
```

**Read the slope, not the height.** Steep means observations packed together;
flat means a gap. From 5 to 6 the curve gains only 0.07 — that flat stretch is
the space between the two populations, visible without choosing anything.

---

## 3.2 Two variables: overplotting and scale

**Run:** `bl run 02-showing-a-relationship.bl`

### Failure 1: the plot that hides its own data

4000 genuinely correlated points (r = 0.738). How many can you actually see?
Round to the ~200×200 positions a printed figure resolves and count collisions:

```
distinct plotting positions occupied: 2901
points drawn on top of an existing point: 1099 (27.5%)
```

**Better than a quarter of the data is invisible**, painted over by points drawn
later — and that estimate is generous, since a figure in a paper column resolves
far less. The dense centre looks like a solid blob whether it holds 500 points or
5000, so the eye reads the *outline*, which is set by the rarest values.

> Overplotting makes a scatterplot report its outliers.

### The fix: count, don't draw

Divide the plane into cells and shade by count. All the data contributes, none
is hidden:

```
  density (each cell shaded by count, peak = 151)
  |        ....:::.....|
  |      ..:.:---::... |
  |      .::-=+=-::..  |
  |     .::=***=-:.    |
  |    .:-=*@%*-:. ..  |
  |   ..:=##@+=::..    |
  | ...:-*##*=::..     |
  | ..::=+*==::.       |
  |..:::::::..         |
```

The correlated ridge is now legible, and the outliers are correctly demoted to
faint single cells rather than defining the picture.

> **An implementation note that matters more than it looks.** The obvious way to
> build this asks each cell which points belong to it — `bins² × n`, 1.6 million
> comparisons, **42 seconds**. Assigning each point to a cell once and tallying
> is linear: **188 ms**, same output. The nested-loop version is the one that
> reads most naturally, and it is 226× slower.

### Failure 2: the wrong scale

600 expression counts spanning orders of magnitude — the usual situation for
sequencing data.

**Raw scale:** almost everything lands in the first bin; the plot is mostly empty
space describing a handful of large values.

**log10:** symmetric, spread across the axis, readable.

The log is not applied to make the picture prettier. Multiplicative quantities —
counts, concentrations, fold changes — are symmetric in log space and skewed in
linear space. **On a linear axis, "twice as much" occupies a different distance
at every position**, which is not what you want an axis to do.

---

## 3.3 Why plot at all?

**Run:** `bl run 03-summaries-that-lie.bl`

*Anscombe's quartet (1973) — not from MSMB, included because it is the shortest
complete argument for this chapter's thesis.*

Four datasets, summarised as you would in a paper:

```
  set   n   mean x   mean y    sd x    sd y      r    slope  intercept
  I    11     9.00    7.50   3.317   2.032  0.816   0.500     3.000
  II   11     9.00    7.50   3.317   2.032  0.816   0.500     3.001
  III  11     9.00    7.50   3.317   2.030  0.816   0.500     3.002
  IV   11     9.00    7.50   3.317   2.031  0.817   0.500     3.002
```

Every column agrees. Same centre, same spread, same correlation, same fitted
line. On this evidence the four datasets are interchangeable.

They are not:

```
  dataset II                             dataset IV
  |                  o                 | |                                 o  |
  |              o o  o o              | |                                    |
  |            o          o            | |            o                       |
  |          o                         | |            o                       |
  |        o                           | |            o                       |
  |      o                             | |            o                       |
  |    o                               | |            o                       |
```

- **I** — a genuine linear relationship with scatter. The summary is honest.
- **II** — a clean *curve*. The relationship is perfect and not linear; the line
  discards the real structure and reports r = 0.816 as though the fit were
  mediocre rather than wrong.
- **III** — a tight straight line plus one outlier that drags the fit off the
  other ten points.
- **IV** — ten points at a single `x`, plus one far away. The slope is set
  entirely by that one point; remove it and the slope is undefined. **r = 0.816
  describes a relationship that does not exist.**

Summary statistics compress, and compression discards. Mean and standard
deviation are the right summary only when the data has the shape those two
numbers assume — roughly symmetric, one lump, no outliers. When it does not,
they do not warn you. They quietly answer a different question.

> Plotting is not presentation. It is the check that the summary you are about to
> publish means what you think it means, and it costs one line.

---

## What to take away

1. **Quantiles describe position, not shape** — so boxplots cannot show
   multimodality, and neither can the five-number summary.
2. **Bin width is a hidden parameter.** Vary it; keep the features that survive.
3. **The ECDF has no parameter**, which makes it the honest first look at a
   distribution.
4. **A dense scatterplot reports its outliers**, not its data. Bin and shade
   instead.
5. **Log scales are not decoration.** They match how multiplicative quantities
   behave.
6. **Identical summaries do not mean similar data.** Plot before you summarise.

## Notes on BioLang

BioLang's plotting is much smaller than ggplot2, and this chapter does not
pretend otherwise. What exists and was used here:

| what | call | returns |
|---|---|---|
| ASCII histogram | `hist(list, bins)` | `Str` |
| SVG histogram | `histogram(list, {title, bins})` | `Str` |
| SVG scatter / line | `plot(table, {title, xlabel, ylabel, kind})` | `Str` |
| ASCII boxplot | `boxplot(list)` | prints, returns `nil` |
| write a figure | `save_svg(svg, path)` | — |

There is no faceting, no alpha blending, no hexbin, no density estimate and no
colour scale. The 2D density plot in 3.2 and the ECDF in 3.1 are written out by
hand, which is the honest way to present the gap.

One documentation defect found and fixed: `bl metadata` described
`scatter(list1, list2)` as returning ASCII. It returns SVG.

## Exercises

1. In 3.1, change the mixture from 70/30 to 95/5. At what bin count does the
   minor population stop being visible? Does the ECDF still show it?
2. Make `bin2d` take rectangular grids and re-plot 3.2 at 60×20. Which aspect
   ratio makes the correlation easiest to judge?
3. Anscombe IV has one point holding up the whole correlation. Compute `r` with
   that point removed for all four datasets; which conclusions change?
4. `boxplot` prints and returns `nil`, while `hist` returns a string. Which is
   easier to compose with, and what does that suggest about API design for
   things that draw?
