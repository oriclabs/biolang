# Introduction

This is a BioLang companion to Susan Holmes and Wolfgang Huber's *Modern
Statistics for Modern Biology*. It follows the book's ideas and worked examples,
and rewrites the analysis in BioLang instead of R.

See [Attribution and licence](attribution.md) — the examples and the statistical
arguments are the authors', and the book is worth reading in its own right.

## Who this is for

Someone who wants the statistics, not just the syntax. Every chapter starts from
a question a biologist would actually ask — *is this peak real?*, *are these four
bases equally common?*, *would my experiment have detected anything?* — and
builds the machinery needed to answer it.

You need no statistics background. You need to be able to read BioLang at the
level of the [language book](https://lang.bio/books/language/html/ch01-getting-started.html):
`let`, `fn`, lists, and the pipe operator.

## How it is organised

```
books/msmb/
  book/src/     the prose you are reading
  code/ch01/    runnable BioLang for each chapter
  code/ch01/lib/  shared implementations
```

Every script in `code/` runs on its own:

```bash
cd books/msmb/code/ch01
bl run 01-poisson.bl
```

No downloads, no setup, no network. See [Getting the data](data.md) for why.

The prose and the code are meant to be read together, but the code is the
substance. If you only do one thing, run the scripts and change the numbers.

## What BioLang has, and what it does not

BioLang is not R, and this companion does not pretend otherwise. Chapter 1
needs `dpois`, `dbinom`, `rpois`, `rbinom`, `ppois`, `quantile`, `frequencies`
and `set_seed`, and BioLang has all of them with the same argument order as R.

It has no `rmultinom` or `dmultinom`. Rather than work around that, the
multinomial is written out in `code/ch01/lib/multinomial.bl` and explained —
which turns out to be the most instructive part of the chapter, because the
multinomial is just the binomial with more than two boxes, and implementing it
is what makes that concrete.

Where a chapter of the original needs machinery BioLang genuinely lacks, this
companion says so rather than substituting something weaker.

## A note on what statistics is for

The single idea underneath this whole chapter, and most of the book:

> To judge whether a result is surprising, you need a description of what
> unsurprising looks like — precise enough to generate fake data from.

That description is a **generative model**. Once you have one, you stop arguing
about whether a number "looks big" and start counting how often a model that
assumes nothing interesting would produce something at least that big. Almost
everything else — p-values, thresholds, power, sample size — falls out of
turning that crank.
