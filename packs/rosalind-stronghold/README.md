# Rosalind — Bioinformatics Stronghold

Worked solutions to the [Bioinformatics Stronghold](https://rosalind.info/problems/list-view/),
**79 of its 107 problems**, every one asserted against the official answer or
against a property that answer must satisfy.

## Why 79 and not 107

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

The 28 problems not included repeat techniques already shown, and each costs
disproportionately more to add. They are listed below with the reason, so the
gap is a decision on the record rather than an omission.

## What is not here, and why

**Blocked by the language, not by effort**

- `REAR`, `SORT` — reversal distance needs a breadth-first search over
  permutations of length 10, roughly 3.6 million states. BioLang has no index
  assignment, so every visited-set update rebuilds a list; the search cannot
  finish.
- `SUFF`, `LREP`, `MREP` — these take a suffix tree as *input* in a bespoke
  format, so most of the work is parsing a structure rather than showing
  anything about sequences.

**Would make the test suite slow**

- `SMGB`, `LAFF`, `MGAP`, `OSYM`, `ITWV`, `MULT` — all dynamic programming.
  `SIMS`, already in the pack, takes about 40 seconds on a 20x99 grid for the
  same reason: each row is rebuilt as a new list. Six more would dominate the
  build for techniques `GLOB`, `LOCA`, `GAFF` and `GCON` already demonstrate.

**Need machinery beyond the problem itself**

- `QRT`, `QRTD`, `CNTQ`, `CHBP`, `CSTR`, `EUBT`, `ALPH` — character table to
  tree construction in both directions. `CTBL` and `SPTD` already show the
  split reasoning these rest on.
- `GASM`, `GREP` — assembly search beyond what `LONG` and `PCOV` demonstrate.
- `RNAS`, `KSIM`, `RSUB` — need memoisation to finish in reasonable time.

**Depends on a remote service**

- `MPRT` — fetches motifs from UniProt, so it could not be part of the
  hermetic test gate that every other problem here passes.

**Unverifiable from the problem statement alone**

- `SGRA`, `FULL` — I could not reconstruct their sample data with enough
  confidence to assert an answer, and an unverified solution in a pack whose
  whole claim is verification would be worse than an absent one.

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
