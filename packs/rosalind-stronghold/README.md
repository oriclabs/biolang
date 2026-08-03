# Rosalind — Bioinformatics Stronghold

Worked solutions to the [Bioinformatics Stronghold](https://rosalind.info/problems/list-view/),
**all 105 of its problems**, every one asserted against the official answer or
against a property that answer must satisfy.

## Notes on a few of them

Several problems have more than one correct answer, and those assert the
defining property rather than the published string:

- `GASM` returns a rotation of the reverse-complement strand. A read carries no
  strand information, so the graph holds every read and its mirror and falls
  into two cycles; either spells the genome.
- `GREP` returns all six Eulerian cycles rather than one. Repeats make the
  assembly genuinely ambiguous, and reporting a single genome would be picking
  arbitrarily.
- `ALPH` and `CHBP` find a different labelling and a different rooting at the
  same cost, so they recount the changes and compare induced splits.

Two carry caveats worth reading before trusting them at scale:

- `KSIM` checks every (start, length) pair, which is O(n^2). Correct and instant
  on the sample; a 50 kbp genome would need the fitting-alignment form instead.
- `MPRT` fetches from UniProt, so it runs in the advisory job rather than the
  hermetic gate. Its motif matcher is asserted offline, so the logic stays
  gated even when the service is unreachable.

`REAR` and `SORT` are the only two whose search lives in Rust rather than in
BioLang. Ten elements admit 45 reversals and 3.6 million reachable orders, and
the distance reaches 9 — a bidirectional search finishes in milliseconds
compiled and not at all interpreted.

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
