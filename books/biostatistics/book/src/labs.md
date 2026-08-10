# Fast Review Labs

The main book develops biostatistics over thirty days. These shorter notebooks
are for learning by doing: make a prediction, run the BioLang code, look at the
figure, and change one value to see what changes.

## Learn statistics by looking

**[Download the visual statistics notebook](downloads/visual-statistics-lab.bln)**

Start here if terms such as variance, skewness, z-score, or p-value feel
abstract. The notebook builds each idea from a small dataset and a picture:

- mean, median, mode, variance, standard deviation, and IQR;
- normal-shaped, skewed, uniform, and bimodal distributions;
- histograms, density curves, outlier flags, and log transformations;
- areas under a curve and z-scores; and
- independent, paired, and one-sample t-tests with careful p-value language.

All code uses BioLang's actual built-in statistics functions. No package or
dataset download is needed.

Run it in the terminal:

```sh
bl notebook visual-statistics-lab.bln
```

Or create the editable browser version introduced by the live notebook
runtime:

```sh
bl notebook visual-statistics-lab.bln --export html-wasm > visual-statistics-lab.html
```

Serve the generated page from a static web host. Each cell is editable, shares
one browser session, and can be run with Shift+Enter. Running a later cell first
automatically evaluates the earlier lesson cells it depends on. The saved
figures remain visible even when WebAssembly is unavailable.

## Seeing probability and false positives

**[Download the runnable notebook](downloads/normal-distribution-lab.bln)**

This guided lab takes about 60-90 minutes. You will:

- see probability as area under a curve;
- compare density, cumulative probability, and a probability cutoff;
- turn a biological measurement into a z-score;
- see where one- and two-sided cutoffs appear on a plot;
- connect a cutoff, a p-value, and a confidence interval; and
- run a seeded simulation showing that false positives still occur when no
  effect was added.

It needs no external package or dataset. Download the notebook, open a terminal
in the directory containing it, and run:

```sh
bl notebook normal-distribution-lab.bln
```

To produce an executed HTML lesson with the generated SVG figures:

```sh
bl notebook normal-distribution-lab.bln --export html > normal-distribution-lab.html
```

All measurements in the notebook are **synthetic teaching data**. Assertions
check the numerical identities, and a fixed random seed makes repeated runs
reproducible.
