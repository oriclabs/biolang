# Rosalind — Bioinformatics Stronghold

Worked solutions to the [Bioinformatics Stronghold](https://rosalind.info/problems/list-view/),
**93 of its 105 problems**, every one asserted against the official answer or
against a property that answer must satisfy.

## Why 93 and not 105

The pack is not a completion badge. It exists so a reader can see how BioLang
handles each kind of bioinformatics problem, and that coverage is already
complete — every technique in the track appears here, most of them repeatedly:

| Technique | Problems |
|---|---|
| Sequence operations | DNA, RNA, REVC, HAMM, GC, TRAN, SUBS, KMER, CONS, REVP, CORR |
| Combinatorics | FIB, FIBD, MRNA, PERM, PPER, SSET, LEXF, LEXV, SIGN, ASPC, ROOT, CUNR |
| Probability and genetics | IPRB, IEV, LIA, SEXL, AFRQ, PROB, INDC, WFMD, EBIN, FOUN, RSTR, MEND |
| Alignment | EDIT, EDTA, CTEA, GLOB, LOCA, GAFF, GCON, OAP, SIMS |
| RNA structure | PMCH, MMCH, CAT, MOTZ |
| Phylogeny and Newick | NWCK, NKEW, CTBL, SPTD, TREE, INOD, PDST, MEND, CSET |
| Assembly and graphs | GRPH, DBRU, LONG, PCOV, ASMQ |
| Mass spectrometry | PRTM, SPEC, CONV, PRSM |
| Strings and search | LCSM, LCSQ, SCSP, KMP, TRIE, LGIS, SSEQ, LING, SETO, PDPL |
| Translation and ORFs | PROT, SPLC, ORF |

The 12 problems not included are listed below with the reason, so the gap is a
decision on the record rather than an omission.

## What is not here, and why

**Genuinely expensive**

- `REAR`, `SORT` — reversal distance needs a breadth-first search over
  permutations of length 10, roughly 3.6 million states. Reachable in principle;
  it would dominate the test suite's runtime, so it is a deliberate deferral
  rather than a blocker.
- `RNAS`, `KSIM`, `RSUB` — need memoisation to finish in reasonable time. If
  BioLang lacks a good idiom for it, that is a language gap worth fixing rather
  than routing around, and these are the problems that would surface it.

**Would make the test suite slow for no new technique**

- `OSYM`, `ITWV` — dynamic programming over grids that `GLOB`, `LOCA`, `GAFF`,
  `GCON`, `SIMS` and `MULT` already demonstrate between them.

**Need machinery beyond the problem itself**

- `QRT`, `QRTD`, `CNTQ`, `CHBP`, `EUBT` — character table to tree construction
  in both directions. `CTBL`, `SPTD` and `CSTR` already show the split reasoning
  these rest on.

## What used to be here, and is not any more

This section previously listed 28 problems. Sixteen of them have since been
solved, because the reasons had been overtaken:

- `LREP`, `MREP`, `SUFF` were called blocked by a bespoke suffix-tree input
  format. `suffix_array` and `lcp_array` exist now, and `SUFF` builds the tree
  from those rather than parsing one.
- `SMGB`, `LAFF`, `MGAP`, `MULT` were called too slow. Runtime work on scope
  lookup, `push` and `unique` changed the arithmetic.
- `CSTR`, `ALPH` needed phylogeny machinery that the Textbook Track's BA7
  series supplied.
- `GASM`, `GREP` needed assembly search that the BA3 series supplied.
- `SGRA`, `FULL` were called unverifiable from the problem statement; their
  sample data is now checked against the published answers directly, and the
  mass-spectrometry work for BA4 and BA11 made both routine.
- `MPRT` was excluded for needing UniProt. It is here now, marked `network`,
  and runs in the advisory job alongside the Armory's three — with its motif
  matcher asserted offline so the logic is still gated.

One claim in the old text was simply wrong: it said BioLang has no index
assignment. `list[i] = value` works and always has; it is *nested* assignment,
`grid[i][j] = value`, that is missing. The stated reason for deferring `REAR`
and `SORT` was therefore inaccurate, and they are deferred above on cost alone.

## Running it

```sh
bl run packs/rosalind-stronghold/examples/dna.bl   # one problem
bl test packs/rosalind-stronghold                  # check every answer
node scripts/verify-packs.mjs                      # check the manifest
```

Every problem also runs in a browser — see the `Runs in` column on the
generated docs page, which is audited against the real WASM module on each
build.

## Licensing

Solutions are MIT, like the rest of BioLang. Problem statements belong to
[rosalind.info](https://rosalind.info) — each file paraphrases Given/Return in
a line or two and links to the original rather than reproducing the prose.
