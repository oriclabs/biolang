# Rosalind — Bioinformatics Textbook Track

Not a course companion.

The [Textbook Track](https://rosalind.info/problems/list-view/?location=bioinformatics-textbook-track)
follows Compeau and Pevzner's *Bioinformatics Algorithms*. Its 124 problems
exist so that students implement the algorithms themselves — Viterbi, the Gibbs
sampler, Burrows-Wheeler, cyclopeptide sequencing. Handing in
`de_bruijn_graph(reads)` would answer the question and teach nothing.

**If you are taking that course, write these yourself.** The answers are here
because the track is a good external test of the language, not to be handed in.

## What it shows

The Textbook Track is the recognised curriculum for bioinformatics algorithms.
Checking its canonical algorithms against BioLang's builtin list is a fair,
externally-defined measure of how much of that curriculum the language already
covers. Sampling twenty of them:

| Algorithm | BioLang |
|---|---|
| Pattern counting, frequent words | `find_motif`, `kmers`, `kmer_count` |
| Reverse complement, Hamming distance | `reverse_complement`, `hamming_distance` |
| k-mer encoding, frequency arrays | `kmer_encode`, `kmer_decode`, `kmer_spectrum` |
| k-mer composition, De Bruijn graphs | `kmers`, `de_bruijn_graph` |
| Translation | `translate` |
| Global, local and edit-distance alignment | `align`, `edit_distance` |
| UPGMA, neighbour joining | `phylo_tree` |
| Lloyd k-means, hierarchical clustering | `kmeans`, `knn_graph`, `leiden_graph` |
| **Suffix arrays** | **none** |
| **Burrows-Wheeler transform** | **none** |
| **HMMs and Viterbi** | **none** |

**17 of 20 already exist as builtins.** The three that do not are a roadmap
derived from an outside standard rather than from opinion: suffix arrays,
Burrows-Wheeler, and hidden Markov models.

## Coverage

33 of 124, each asserted against the official answer.

The BA1, BA2, BA3 and BA5 series are complete or nearly so — pattern finding,
motif finding, assembly graphs, and the alignment problems. BA9 has begun with
the suffix array, which was one of the three gaps this pack was written to
measure and is a builtin now.

What remains is honest about itself: BA10 and BA11 are hidden Markov models, and
without Viterbi in the language they cannot be written here at all. BA6 is
genome rearrangement and BA7 phylogeny, both of which need real work rather than
a builtin call.

## One finding worth keeping

`BA1B` asks for the most frequent k-mers. The obvious call is `kmer_count()`,
and it gives the wrong answer — it tallies **canonical** k-mers, pooling each
with its reverse complement, so `GCAT` and `ATGC` are reported as a single
count of 4 rather than 3 and 1. That is the right default for genomics, where
the strand is often unknown, and the wrong one here.

The example counts raw windows from `kmers()` instead and says why. It is the
kind of mismatch a showcase is useful for surfacing.

## Running it

```sh
bl run packs/rosalind-textbook/examples/ba1a.bl   # one problem
bl test packs/rosalind-textbook                   # check every answer
```

## Licensing

Solutions are MIT, like the rest of BioLang. Problem statements belong to
[rosalind.info](https://rosalind.info) — each file paraphrases Given/Return in
a line or two and links to the original rather than reproducing the prose.
