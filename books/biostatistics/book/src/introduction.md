# Practical Biostatistics in 30 Days

> *From p-values to pipelines — a structured journey through the statistics every biologist actually needs.*

You have the data. Thousands of gene expression measurements. Hundreds of patient outcomes. Millions of variants. You know the biology. You understand the experiment. But when it comes time to choose a statistical test, set a significance threshold, or interpret a confidence interval, the ground shifts under your feet.

This book fixes that. In 30 days, you will go from statistical anxiety to statistical fluency — not by memorizing formulas, but by solving real biological problems with real data. Every test you learn has a reason. Every formula has a story. Every p-value has a context.

And you will do it all in BioLang, whose current runtime provides hundreds of
builtins across statistics, data handling, biological formats, and
visualization. That lets you express an analysis — from data loading to
hypothesis testing to publication-quality visualization — in readable,
pipe-chained steps.

## Who This Book Is For

This book is for anyone who works with biological data and needs to make sound statistical decisions. You might be:

- **A biologist who dreads the statistics section.** You can design elegant experiments, but when the reviewer asks why you used a t-test instead of a Mann-Whitney U, you panic. You have tried statistics textbooks, but they are full of coin flips and card games when you need differential expression and survival curves. This book teaches statistics through biology, using datasets and questions you actually care about.

- **A developer entering biotech.** You can write production code and build data pipelines, but you do not know the difference between a parametric and a non-parametric test. You have heard that bioinformatics requires "statistical thinking," but nobody has explained what that means in practice. This book gives you the statistical intuition alongside the implementation, so you understand *why* you are computing a fold change, not just *how*.

- **A graduate student facing qualifying exams.** Your program expects fluency in biostatistics, but your coursework is a blur of Greek letters and proof sketches. You need a practical guide that connects the math to the biology and shows you how to actually run these tests on real data. This book builds that bridge in 30 structured days.

- **A clinical researcher designing or analyzing studies.** You work with patient cohorts, treatment outcomes, and survival data. You need to choose the right test, compute adequate sample sizes, and report results that satisfy both statisticians and regulatory reviewers. This book covers clinical biostatistics end to end — from power analysis through Cox proportional hazards.

No matter which category you fall into, you share one thing: you want statistical skills that solve real problems, not abstract exercises. Every day in this book produces an analysis you can adapt to your own data.

### Your Path Through the Book

The first week builds foundations for every reader, but your starting point may differ. Here is which days to prioritize based on your background:

| Your background | Focus on | Skim or review |
|---|---|---|
| **Biologist, limited stats training** | Days 1-3 (distributions, central tendency, variability) | Day 4 if you already know probability basics |
| **Statistician, new to biology** | Days 5-7 (biological context for common tests) | Days 1-3 (you know the math already) |
| **New to both stats and biology** | Every day — they are written for you | Nothing — read it all |
| **Some stats background** | Skim Week 1 for BioLang syntax, start deeply at Week 2 | Days 1-4 for review only |

> **Complete beginner?** That is completely fine. Day 1 starts with a single concept — what is a distribution? — and builds from there. No calculus. No linear algebra. No prior programming experience beyond basic BioLang familiarity. If you can type `bl run script.bl`, you are ready.

## What You Will Learn

Over 30 days, you will go from statistical uncertainty to being able to:

- Describe and summarize any biological dataset (distributions, central tendency, spread, outliers)
- Choose the correct statistical test for any experimental design
- Perform and interpret t-tests, ANOVA, chi-square tests, and their non-parametric alternatives
- Run linear and logistic regression on biological data
- Analyze time-to-event data with Kaplan-Meier curves and Cox proportional hazards models
- Reduce high-dimensional data with PCA and interpret biplots
- Cluster samples and genes using hierarchical and k-means methods
- Correct for multiple testing with Bonferroni, Benjamini-Hochberg, and permutation approaches
- Compute effect sizes, confidence intervals, and statistical power
- Design experiments with proper sample size calculations
- Build volcano plots, Manhattan plots, Q-Q plots, and forest plots
- Apply Bayesian reasoning to biological problems
- Complete three capstone analyses that mirror real research publications

You will learn all of this in BioLang, which provides dedicated builtins for every test and method. But you will not be locked in. Every day includes comparison examples in Python (scipy/statsmodels) and R (base stats/survival/ggplot2), so you can translate your skills to any environment.

## How This Book Is Structured

The book is organized into four weeks plus capstone projects:

| Week | Days | Theme | What You Build |
|------|------|-------|----------------|
| **Week 1** | 1-5 | Foundations | Understand distributions, probability, and descriptive statistics |
| **Week 2** | 6-12 | Core Methods | Master hypothesis testing, t-tests, ANOVA, chi-square, non-parametric tests |
| **Week 3** | 13-20 | Modeling | Correlation, regression, survival, design, effect size, and batch effects |
| **Week 4** | 21-27 | Advanced Topics | PCA, clustering, resampling, Bayesian methods, visualization, and reproducibility |
| **Capstone** | 28-30 | Projects | Clinical trial, differential expression, and GWAS analyses |

Each day follows the same structure:

1. **The Problem** — a vivid scenario that shows *why* you need today's method. A researcher staring at ambiguous results. A clinician choosing between treatments. A graduate student defending a finding.
2. **What Is [Topic]?** — a plain-language explanation of the statistical concept, free of jargon. If your collaborator asked "what is a p-value?" at a coffee shop, this is how you would explain it.
3. **Core Concepts** — the ideas, assumptions, and mechanics, presented with tables, diagrams, and worked examples. Formulas appear when they clarify; they are never the point.
4. **[Topic] in BioLang** — working code that applies the concept to biological data. Pipe-chained, readable, annotated.
5. **Python and R Comparison** — the same analysis in scipy/statsmodels and R, so you can see how the languages compare.
6. **Exercises** — practice problems at three difficulty levels (Foundations, Applied, Challenge).
7. **Key Takeaways** — the essential points to remember, in bold-and-explanation format.

Days are designed to take 1-3 hours each. Concept-heavy days (like Day 1 on distributions) are shorter. Method-heavy days (like Day 14 on logistic regression) are longer. Work at your own pace — there is no penalty for spending two days on one topic.

## Prerequisites

You need:

- **A computer** running Windows, macOS, or Linux
- **BioLang installed** — see the setup section below or [Appendix A](appendix-setup.md)
- **Basic BioLang familiarity** — you can write variables, use pipes, and call functions. If you have completed *Practical Bioinformatics in 30 Days* or the BioLang tutorials, you are ready.
- **High school math** — you understand addition, multiplication, fractions, and basic algebra. That is all.

You do *not* need:

- A statistics course (this book *is* the course)
- Calculus or linear algebra (we explain everything from scratch)
- Prior experience with R, Python, or any statistics software
- A powerful machine (a laptop with 4 GB of RAM handles every exercise)

If you can run `bl --version` and get a version number, you are ready.

## The Companion Files

Every day has a companion directory with a BioLang setup script and Python and
R comparison scripts. The runnable BioLang analyses are the code blocks in the
chapter itself. The current companion structure is:

```
biostatistics/
  days/
    day-01/
      init.bl             # Setup script — run this first
      scripts/
        analysis.py       # Python equivalent
        analysis.R        # R equivalent
      expected/
        .gitkeep          # Reserved for chapter-specific baselines
    day-02/
      ...
```

To use the companion files:

1. **Run `init.bl` first.** Each day's init script generates sample datasets, downloads reference data, or creates whatever that day's exercises need. Run it with `bl run init.bl`.

2. **Run the BioLang blocks in the chapter.** Blocks on the same page may build
   on variables defined earlier. File-backed and network examples are marked as
   CLI-only.

3. **Validate your script.** Run `bl check analysis.bl`, then compare important
   estimates with the Python and R scripts. Statistical results should agree
   within the stated rounding tolerance and method assumptions.

4. **Use `scripts/analysis.py` and `scripts/analysis.R` for independent
   validation.** Check test variants, missing-value behavior, correction
   methods, and model families before treating small numerical differences as
   errors.

To get the companion files:

```bash
git clone https://github.com/oriclabs/biolang.git
cd biolang/books/biostatistics
```

Or download the ZIP from the book's website and extract it.

## Setting Up Your Environment

Full installation instructions are in [Appendix A](appendix-setup.md), but here is the short version:

```bash
# Install BioLang
curl -sSf https://biolang.org/install.sh | sh

# Verify it works
bl --version

# Launch the REPL to test
bl repl
```

On Windows, use the PowerShell installer:

```powershell
irm https://biolang.org/install.ps1 | iex
```

If you want to run the Python comparison scripts (optional but recommended):

```bash
pip install scipy numpy pandas matplotlib statsmodels lifelines scikit-learn
```

If you want to run the R comparison scripts (optional but recommended):

```r
install.packages(c("stats", "survival", "ggplot2", "dplyr", "pwr", "lme4", "boot"))
```

## A Quick Taste

Here is what statistical analysis looks like in BioLang. This script loads a
differential-expression result table, adjusts p-values, filters significant
genes, and generates a volcano plot:

```bio
# Load a compact differential-expression result table
let expression = read_csv("data/expression.csv")
let adjusted = p_adjust(col(expression, "pvalue"), "BH")
let results = zip(to_records(expression), adjusted)
  |> map(|pair| {...pair[0], padj: pair[1]})
  |> to_table()

# How many genes are differentially expressed?
let significant = results
  |> filter(|row| row.padj < 0.05 && abs(row.log2fc) > 1.0)
println("Differentially expressed genes: " + str(len(significant)))

# Volcano plot
volcano(results, {fc: "log2fc", p: "padj"})
```

The pipe operator makes the analytical logic visible: load, correct, combine,
filter, and plot. You will understand every line by the end of Week 2.

Here is another example — survival analysis in three lines:

```bio
let patients = read_csv("data/clinical.csv")
let survival = patients |> map(|row| {
  time: row.survival_months,
  event: if row.status == "deceased" { 1 } else { 0 },
  group: row.treatment
}) |> to_table()
kaplan_meier(survival, {title: "Overall Survival by Treatment"})
```

And power analysis for planning your next experiment:

```bio
# Power calculation: how many samples per group?
let result = power_t_test(0.5, 0.05, 0.8)
println("Required sample size per group: {result.n}")
println("Effect size: {result.effect_size}, alpha: {result.alpha}, power: {result.power}")
```

BioLang's statistical, tabular, and visualization builtins keep the analysis
close to the biological question.

## Week-by-Week Overview

### Week 1: Foundations (Days 1-5)

You start where every statistical analysis starts — with the data itself. Day
1 explains why statistics matters; Day 2 covers descriptive statistics; Day 3
examines distributions; Day 4 introduces probability; and Day 5 covers
sampling, bias, and sample size.

### Week 2: Core Methods (Days 6-12)

Day 6 introduces confidence intervals. Day 7 covers hypothesis tests and
p-values; Day 8 covers t-tests; Day 9 handles non-parametric alternatives; Day
10 covers ANOVA; Day 11 tackles categorical tests; and Day 12 addresses
multiple-testing correction.

### Week 3: Modeling (Days 13-20)

You move from association to modeling. Day 13 covers correlation; Day 14
introduces linear regression; Day 15 extends it to multiple predictors; Day 16
covers logistic regression; Day 17 covers survival analysis; Day 18 covers
experimental design and power; Day 19 focuses on effect sizes; and Day 20
handles batch effects and confounding.

### Week 4: Advanced Topics and Capstones (Days 21-27)

Day 21 covers PCA and dimensionality reduction. Day 22 covers clustering. Day
23 introduces bootstrap and permutation methods; Day 24 introduces Bayesian
thinking; Day 25 develops statistical visualization; Day 26 covers
meta-analysis; and Day 27 turns the methods into a reproducible workflow.

### Capstone Projects (Days 28-30)

Three full projects integrate the book. Day 28 analyzes a clinical trial with
survival and subgroup outcomes. Day 29 conducts a differential-expression
study. Day 30 analyzes a genome-wide association study with Manhattan and Q-Q
plots and genomic-inflation checks.

## Conventions Used in This Book

Throughout this book, you will see several recurring elements:

### Code Blocks

BioLang code appears in fenced code blocks:

```bio
let data = [2.3, 4.1, 3.7, 5.2, 4.8]
mean(data)         # 4.02
stdev(data)        # 1.082
```

When a code block shows REPL interaction, lines starting with `bl>` are what you type:

```
bl> ttest([23.1, 25.4, 22.8], [19.2, 20.1, 18.7])
TTestResult { t: 4.12, df: 4, p: 0.0146 }
```

Shell commands use `bash` syntax:

```bash
bl run day07_ttest.bl
```

### Python and R Comparisons

Multi-language comparisons appear with labeled blocks:

**BioLang:**
```bio
let data = read_csv("data/expression.csv")
let control = data |> filter(|row| row.condition == "control") |> map(|row| row.sample1)
let treated = data |> filter(|row| row.condition == "treatment") |> map(|row| row.sample1)
println(ttest(control, treated))
```

**Python:**
```python
import pandas as pd
from scipy.stats import ttest_ind
data = pd.read_csv("expression.csv")
stat, p = ttest_ind(data["ctrl"], data["treated"])
print(f"t={stat:.4f}, p={p:.4f}")
```

**R:**
```r
data <- read.csv("expression.csv")
t.test(data$ctrl, data$treated)
```

### Callout Boxes

Important notes, insights, and warnings appear as blockquotes throughout:

> **Key insight:** A statistically significant result is not necessarily biologically meaningful. Always report effect sizes alongside p-values.

> **Clinical relevance:** In oncology trials, a hazard ratio below 0.7 is typically considered clinically meaningful, regardless of the p-value.

> **Common pitfall:** Running 20 t-tests on the same dataset without multiple testing correction gives you a 64% chance of at least one false positive at alpha = 0.05.

### Exercises

Each day ends with exercises labeled by difficulty:

- **Foundations** — reinforce the core concept with guided problems
- **Applied** — use the method on a new biological dataset
- **Challenge** — extend the method or combine it with previous days

### Key Takeaways

Each day concludes with a bulleted list of the most important points:

- **The p-value is not the probability that your hypothesis is wrong.** It is the probability of observing data this extreme if the null hypothesis were true. This distinction matters enormously.

## A Note on the Multi-Language Approach

This book uses BioLang as its primary language because its statistical builtins let you focus on the concepts rather than the plumbing. A t-test is one function call, not a chain of imports and data manipulations. A volcano plot is one line, not thirty.

But the real world uses Python and R for most biostatistics. We include comparisons for two reasons:

1. **Translation.** If you already know scipy or R's stats package, seeing the BioLang equivalent helps you learn faster. If you learn BioLang first, seeing the Python and R equivalents prepares you for collaborative work.

2. **Verification.** Running the same analysis in three languages and getting the same answer builds confidence. When your BioLang t-test gives p = 0.014 and your R t-test gives p = 0.014, you know you have done it right.

The `compare.md` file in each day's companion directory provides a detailed side-by-side comparison. The `analysis.py` and `analysis.R` scripts are runnable equivalents you can execute and compare.

## Let's Begin

You have everything you need. The next 30 days will transform how you think about biological data — not just how to analyze it, but how to reason about uncertainty, variability, and evidence.

Day 1 starts with the most fundamental question in statistics: what does your data look like?

Turn the page. Your journey starts now.
