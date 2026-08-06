# Attribution and licence

## The source

This companion follows the structure and worked examples of:

> **Modern Statistics for Modern Biology**
> Susan Holmes and Wolfgang Huber
> Cambridge University Press, 2018. ISBN 9781108705295.
> Freely readable at <https://www.huber.embl.de/msmb/>

The book is the work of Holmes and Huber. The choice of examples — the epitope
array, the *C. elegans* mitochondrial base composition, the multinomial power
simulation — is theirs, and the statistical arguments are theirs. If this
companion is useful to you, the book it follows is more so; buy it, or read it
free at the link above.

## What is original here

- **All prose in this companion is written from scratch.** No text is copied
  from *MSMB*. Where a passage explains the same idea, it explains it in
  different words, because the point of this companion is the BioLang code and
  not a paraphrase of someone else's writing.
- **All code is original BioLang**, not a transliteration of the book's R. Where
  BioLang lacks an equivalent (there is no `rmultinom`), the algorithm is
  written out and explained rather than approximated.
- **Numerical results are cross-checked against the book** where it publishes a
  value. Chapter 1's `dmultinom([4,2,0,0])` and the *C. elegans* goodness-of-fit
  statistic of 4386.6 both reproduce the book's figures exactly, which is the
  strongest evidence available that the BioLang implementations are correct.

## What is reproduced

- **Data values.** A handful of small constants appear here so the examples run
  offline — the *C. elegans* mitochondrial base counts (4335, 1225, 2055, 6179)
  are four integers derived from a public reference genome, and are facts about
  that genome rather than authored content.
- **No datasets are redistributed.** The book's data bundle is downloaded from
  the authors' own server if you want it; see [Getting the data](data.md).

## Licence

This companion is part of the BioLang repository and carries its MIT licence.
That licence covers the prose and BioLang code written here. It does not and
cannot extend to *Modern Statistics for Modern Biology*, which remains
© Susan Holmes and Wolfgang Huber, published by Cambridge University Press.

## Citing

If you cite anything from this companion, cite the source it follows:

```bibtex
@book{holmes2018modern,
  title     = {Modern Statistics for Modern Biology},
  author    = {Holmes, Susan and Huber, Wolfgang},
  year      = {2018},
  publisher = {Cambridge University Press},
  isbn      = {9781108705295},
  url       = {https://www.huber.embl.de/msmb/}
}
```
