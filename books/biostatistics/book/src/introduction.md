# Practical Biostatistics in 30 Days

> *From p-values to pipelines — a structured journey through the statistics every biologist actually needs.*

You have the data. Thousands of gene expression measurements. Hundreds of patient outcomes. Millions of variants. You know the biology. You understand the experiment. But when it comes time to choose a statistical test, set a significance threshold, or interpret a confidence interval, the ground shifts under your feet.

This book fixes that. In 30 days, you will go from statistical anxiety to statistical fluency — not by memorizing formulas, but by solving real biological problems with real data. Every test you learn has a reason. Every formula has a story. Every p-value has a context.

And you will do it all in BioLang, a language with over 400 statistical builtins that lets you express an entire analysis — from data loading to hypothesis testing to publication-quality visualization — in a handful of readable, pipe-chained lines.

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
| **Week 3** | 13-20 | Modeling | Regression, survival analysis, dimensionality reduction, clustering |
| **Week 4** | 21-27 | Advanced Topics | Multiple testing, Bayesian methods, power analysis, resampling, study design |
| **Capstone** | 28-30 | Projects | Differential expression study, GWAS analysis, clinical trial analysis |

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

Every day has a companion directory with runnable code, sample data, and expected output. The structure looks like this:

```
biostatistics/
  days/
    day-01/
      README.md           # Day overview and instructions
      init.bl             # Setup script — run this first
      scripts/
        analysis.bl       # BioLang solutions
        analysis.py       # Python equivalent
        analysis.R        # R equivalent
      expected/
        output.txt        # Expected output for verification
      compare.md          # Side-by-side language comparison
    day-02/
      ...
```

To use the companion files:

1. **Run `init.bl` first.** Each day's init script generates sample datasets, downloads reference data, or creates whatever that day's exercises need. Run it with `bl run init.bl`.

2. **Work through the exercises.** Try to solve them yourself before looking at the solutions in `scripts/`.

3. **Check your output.** Compare your results against the files in `expected/` to verify correctness. Statistical results should match within rounding tolerance.

4. **Read `compare.md`.** After completing a day in BioLang, read the comparison document to see how the same analyses look in Python and R. This is especially valuable if you plan to work in multi-language environments.

To get the companion files:

```bash
git clone https://github.com/bioras/practical-biostatistics.git
cd practical-biostatistics
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

Here is what statistical analysis looks like in BioLang. This script loads gene expression data, runs a t-test between two conditions, and generates a volcano plot — all in pipe style:

```bio
# Load expression data for two conditions
let ctrl = read_csv("control_expression.csv")
let treat = read_csv("treated_expression.csv")

# Run t-tests for every gene, correct for multiple testing
let results = ctrl
  |> join(treat, "gene_id")
  |> mutate("pvalue", ttest(ctrl_expr, treat_expr).p)
  |> mutate("log2fc", log2(mean(treat_expr) / mean(ctrl_expr)))
  |> mutate("padj", p_adjust(pvalue, "BH"))
  |> mutate("significant", padj < 0.05 and abs(log2fc) > 1.0)

# How many genes are differentially expressed?
results
  |> filter(significant)
  |> len()
  |> println("Differentially expressed genes: {}")

# Volcano plot
results |> volcano_plot("log2fc", "padj", "gene_id")
```

Twelve lines. No imports. No boilerplate. The pipe operator makes the analytical logic visible: load, join, test, correct, filter, plot. You will understand every line of this by Day 10.

Here is another example — survival analysis in three lines:

```bio
let patients = read_csv("clinical_outcomes.csv")

patients
  |> kaplan_meier("months", "deceased", "treatment")
  |> surv_plot({title: "Overall Survival by Treatment Arm"})
```

And power analysis for planning your next experiment:

```bio
# Power calculation: how many samples per group?
let result = power_t_test(0.5, 0.05, 0.8)
println("Required sample size per group: {result.n}")
println("Effect size: {result.effect_size}, alpha: {result.alpha}, power: {result.power}")
```

BioLang's 400+ statistical builtins mean you spend your time thinking about the biology, not fighting the syntax.

## Week-by-Week Overview

### Week 1: Foundations (Days 1-5)

You start where every statistical analysis starts — with the data itself. What does a distribution look like? How do you measure center and spread? What is probability, and why does it matter for hypothesis testing? Day 1 introduces distributions with real gene expression data. Day 2 covers descriptive statistics. Day 3 tackles probability and the normal distribution. Day 4 introduces sampling and the central limit theorem. Day 5 covers confidence intervals. By Friday, you have the vocabulary and intuition to understand every test that follows.

### Week 2: Core Methods (Days 6-12)

Now the testing begins. Day 6 introduces hypothesis testing and p-values. Day 7 covers t-tests — one-sample, two-sample, paired — with gene expression data. Day 8 is ANOVA for comparing multiple groups. Day 9 handles non-parametric alternatives for when your data violates assumptions. Day 10 tackles chi-square and Fisher's exact tests for categorical data. Day 11 introduces correlation and simple linear regression. Day 12 brings multiple testing correction — Bonferroni, Benjamini-Hochberg, and permutation — the single most important topic for genomics.

### Week 3: Modeling (Days 13-20)

You move from testing to modeling. Day 13 covers multiple regression. Day 14 introduces logistic regression for binary outcomes. Day 15 is survival analysis — Kaplan-Meier curves and log-rank tests. Day 16 continues with Cox proportional hazards models. Day 17 introduces PCA and dimensionality reduction. Day 18 covers clustering — hierarchical, k-means, and silhouette analysis. Day 19 tackles effect sizes and confidence intervals as alternatives to p-values. Day 20 brings statistical visualization — volcano plots, Manhattan plots, Q-Q plots, forest plots, and heatmaps.

### Week 4: Advanced Topics and Capstones (Days 21-27)

You tackle the hard problems. Day 21 covers bootstrap and permutation methods. Day 22 introduces Bayesian statistics with biological examples. Day 23 is power analysis and sample size calculation. Day 24 covers experimental design — randomization, blocking, batch effects. Day 25 tackles mixed models for repeated measures and nested designs. Day 26 introduces enrichment analysis — gene ontology, pathway analysis, GSEA. Day 27 covers meta-analysis for combining results across studies.

### Capstone Projects (Days 28-30)

Three full projects that integrate everything you have learned. Day 28: conduct a complete RNA-seq differential expression study with quality control, normalization, testing, multiple correction, and pathway enrichment. Day 29: analyze a genome-wide association study with Manhattan plots, Q-Q plots, and genomic inflation correction. Day 30: analyze a clinical trial dataset with survival analysis, subgroup comparisons, and a statistical report suitable for publication.

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
let data = read_csv("expression.csv")
data |> ttest(ctrl, treated) |> println()
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
