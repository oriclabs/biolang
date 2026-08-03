# Rosalind — Bioinformatics Armory

All 15 problems from the [Bioinformatics Armory](https://rosalind.info/problems/list-view/?location=bioinformatics-armory)
track, worked in BioLang.

The Armory track is about *using* existing bioinformatics tools rather than
implementing algorithms, which makes it a good fit for BioLang: most problems
reduce to one or two builtin calls plus a little glue.

## Coverage

| # | Problem | Status | Verified by `bl test` |
|---|---------|--------|-----------------------|
| INI  | Introduction to the Bioinformatics Armory | solved | yes |
| GBK  | GenBank Introduction | solved | no — needs NCBI, answer drifts |
| FRMT | Data Formats | solved | no — needs NCBI |
| MEME | New Motif Discovery | partial | yes (approximation) |
| NEED | Pairwise Global Alignment | partial | no — needs NCBI |
| TFSQ | FASTQ format introduction | solved | yes |
| PHRE | Read Quality Distribution | solved | yes |
| PTRA | Protein Translation | solved | yes |
| FILT | Read Filtration by Quality | solved | yes |
| RVCO | Complementing a Strand of DNA | solved | yes |
| SUBO | Suboptimal Local Alignment | solved | yes |
| BPHR | Base Quality Distribution | solved | yes |
| CLUS | Global Multiple Alignment | solved | yes |
| ORFR | Finding Genes with ORFs | solved | yes |
| BFIL | Base Filtration by Quality | solved | yes |

**13 solved, 2 partial. 12 of 15 are asserted against the official Rosalind
answer** and run on every commit; the other 3 call out to NCBI and are checked
on a separate, non-blocking schedule.

### What "partial" means

Two problems run and teach the technique but do not reproduce the official
answer, and say so in their own output rather than pretending:

- **MEME** expects a position-weight motif from the MEME Suite. BioLang has no
  probabilistic motif discovery, so this finds the exact substrings shared by
  every input instead. It asserts *that* result, not MEME's.
- **NEED** expects EMBOSS Needle's score of 257 using the DNAfull substitution
  matrix with affine gaps. `align()` has neither, so the score differs.

Both are recorded in `pack.toml` with a `blocked_on` field naming the missing
capability. That list doubles as a roadmap: implement affine gaps and a
substitution-matrix parameter and NEED moves to `solved`.

## Running it

```sh
bl run packs/rosalind-armory/examples/ini.bl   # run one problem
bl test packs/rosalind-armory                  # check every asserted answer
node scripts/verify-packs.mjs                  # check the manifest matches disk
```

`bl test` only executes files that define `test_*` functions, so the three
network problems stay inert during a test run.

## Licensing

Solutions are MIT, like the rest of BioLang. Problem statements belong to
[rosalind.info](https://rosalind.info) — each file paraphrases Given/Return in a
line or two and links to the original rather than reproducing the prose. Sample
datasets are the short ones printed in the problem statements; per-user
generated datasets are not redistributed.
