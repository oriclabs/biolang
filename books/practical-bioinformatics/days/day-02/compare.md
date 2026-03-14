# Day 2: Language Comparison

Lines of code to perform the same Day 2 analyses.

| Task | BioLang | Python | R |
|------|---------|--------|---|
| Variables + type checking | 7 | 7 | 7 |
| Pipe operator demo | 5 | 6 | 6 |
| Central dogma pipeline | 4 | 5 | 5 |
| Motif search | 3 | 7 | 10 |
| Lists and records | 8 | 8 | 10 |
| Functions + lambdas | 5 | 6 | 5 |
| Control flow (if/for/match) | 12 | 14 | 15 |
| Higher-order functions | 10 | 12 | 16 |
| Mini-analysis | 15 | 18 | 22 |
| Comparison task | 5 | 5 | 6 |
| **Imports / setup** | **0** | **4** | **5** |
| **Total** | **74** | **92** | **107** |

## Key Differences

**BioLang**: No imports needed. Pipes are a language feature, not a library.
HOFs (`map`, `filter`, `reduce`, `each`) work identically on lists. Pattern
matching with `match` is cleaner than if/elif chains. Bio types are first-class.

**Python**: Requires Biopython for sequence types. `gc_fraction()` instead of
`gc_content()`. No built-in pipe operator. `map`/`filter` return iterators
requiring `list()` wrapping. Pattern matching added in 3.10 but not widely used.
Motif search needs regex with lookahead for overlapping matches.

**R**: Requires Bioconductor installation (slow first setup). `letterFrequency()`
is the GC content function. `|>` pipe added in R 4.1 but less flexible than
BioLang pipes. HOFs use `sapply`/`lapply`/`Reduce` with verbose syntax.
Nested lists require `[[` vs `[` distinction. 1-based indexing.
