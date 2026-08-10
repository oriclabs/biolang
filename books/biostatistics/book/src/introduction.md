# Practical Biostatistics in 30 Days

Biostatistics helps you answer ordinary biological questions:

- What does this dataset look like?
- Is the difference between two groups larger than the usual variation?
- How uncertain is the estimate?
- Could a technical batch explain the pattern?
- Is an effect large enough to matter biologically?

You do not need to memorize a wall of formulas before you begin. This book
starts with a question and the data, explains one idea in plain language, and
then shows how to apply it in BioLang.

## The easiest way to use this book

If you want a fast refresher, begin with the [Fast Review Labs](labs.md). The
first notebook joins four ideas that are often taught separately: the normal
curve, probability, confidence intervals, and false positives.

For the full course, read one chapter at a time. On a first pass:

1. Read the **Practical question**.
2. Look at the values or plot before choosing a method.
3. Read **What to remember**.
4. Copy and run the BioLang example locally if you want hands-on practice.
5. Leave derivations and language comparisons for a later pass.

There is no requirement to finish a chapter in one sitting. The day numbers
give the topics an order; they are not a deadline.

## A simple pattern for every analysis

Use the same five questions throughout the book:

1. **What is the biological question?** State the comparison or relationship
   before naming a statistical test.
2. **What do the data look like?** Plot the individual values, check missing
   data, and note unusual observations.
3. **Is the method suitable?** Check the study design and the assumptions that
   matter for that method.
4. **How large and how uncertain is the result?** Report an effect size and an
   interval, not only a p-value.
5. **What could provide another explanation?** Consider batch, confounding,
   selection bias, and multiple testing.

This pattern matters more than remembering the name of every test.

## About the examples

Examples in this book use three labels:

- **Synthetic teaching data** are generated for learning. They do not describe
  a real patient, laboratory, institution, trial, or regulatory decision.
- **Adapted example** means the question or data structure comes from a named
  source, but values may have been simplified. The source must be cited.
- **Real dataset** means the data source and accession or publication are
  identified.

Unless a chapter explicitly identifies a source, treat its numbers as
**synthetic teaching data**. They are useful for learning a method, not for
making clinical or biological claims. Clinical examples are educational only
and must not be used for patient care.

## What you will learn

The chapters build in a practical order:

| Part | Days | Main question |
|---|---:|---|
| Foundations | 1-5 | What do the data look like, and where did they come from? |
| Comparing groups | 6-12 | Is a difference credible, and how many tests were run? |
| Relationships and models | 13-20 | Which variables move together, and what else could explain it? |
| Larger datasets and workflows | 21-27 | How do we explore many variables and keep an analysis reproducible? |
| Guided projects | 28-30 | How do the ideas fit into a complete analysis? |

The later chapters cover regression, survival analysis, PCA, clustering,
resampling, Bayesian reasoning, visualization, and meta-analysis. These names
may look technical now. Each chapter introduces them only when a practical
question needs them.

## What you need

You need BioLang and basic familiarity with variables, lists, function calls,
and the pipe operator. Setup instructions are in [Appendix A](appendix-setup.md).
High-school arithmetic is enough for the first pass. Calculus and formal proofs
are not prerequisites.

Code blocks have a **Copy** button. They are intentionally not executed inside
the book: some examples depend on earlier code, local files, or native BioLang
features. Copy a complete example into a `.bl` file or use the downloadable
`.bln` notebook, where the order is explicit.

## Your first BioLang example

Start with five measurements. Look at them before doing anything complicated:

```bio
let values = [11.2, 13.1, 12.8, 10.9, 14.2]

println("values: " + str(values))
println("mean: " + str(mean(values)))
println("median: " + str(median(values)))
println("standard deviation: " + str(stdev(values)))
```

The purpose is not to collect four numbers. It is to answer three plain
questions: where is the center, how much do observations vary, and is any value
surprising? Day 2 develops that habit.

## Optional comparisons

Some companion folders include Python and R versions. Use them when you want to
translate a method or independently check a result. They are optional for
learning the concept; BioLang remains the main language of the book.

When two tools disagree, compare the test variant, treatment of missing values,
tails, variance assumption, correction method, and rounding before concluding
that one implementation is wrong.

## Where to begin

- New to statistics: start with [Day 1](day-01.md), then continue in order.
- Revisiting familiar ideas: start with the [Fast Review Labs](labs.md).
- Solving a current problem: use the [test-selection flowchart](appendix-flowchart.md)
  and then read the linked chapter.

The goal is modest and useful: understand what the data can support, what it
cannot support, and how to communicate the difference clearly.
