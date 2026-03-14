# Day 1: Language Comparison

Lines of code to perform the same Day 1 analyses.

| Task | BioLang | Python | R |
|------|---------|--------|---|
| Create + inspect sequence | 4 | 5 | 4 |
| Central dogma (transcribe + translate) | 5 | 7 | 8 |
| Base composition + GC content | 3 | 5 | 5 |
| Motif search | 2 | 3 | 4 |
| Pipe chain (complement + revcomp + transcribe) | 2 | 4 | 4 |
| Exercise solutions | 3 | 5 | 6 |
| **Imports / setup** | **0** | **4** | **2** |
| **Total** | **19** | **33** | **33** |

## Key Differences

**BioLang**: No imports needed. Sequence types are built-in. Pipe operator chains
operations naturally. Motif search returns 0-based positions directly.

**Python**: Requires Biopython install and imports. `Seq` objects work well but
`translate(to_stop=True)` is an extra parameter to remember. Motif search requires
regex with lookahead for overlapping matches.

**R**: Requires Bioconductor installation (slow first-time setup). `Biostrings`
is powerful but verbose. Must manually convert between DNA/RNA string types.
1-based indexing requires adjustment for bioinformatics conventions.
