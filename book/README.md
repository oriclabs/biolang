# The BioLang Book

A comprehensive guide to BioLang — the pipe-first bioinformatics programming language.

## Building

This book uses [mdBook](https://rust-lang.github.io/mdBook/). Install it and run:

```bash
cd book
mdbook serve --open
```

## Structure

Each chapter focuses on a language feature with real bioinformatics examples.
No "hello world" — every example processes sequences, genomes, variants, or biological data.

## Chapters

1. Getting Started — install, REPL, first script
2. Bio Literals & Sequences — DNA, RNA, protein, quality scores
3. Variables, Types & Records — let bindings, records, type system
4. Collections & Tables — lists, sets, tables, column operations
5. Pipes & Transforms — |> pipe-first, |>> tap-pipe, chaining
6. Functions & Closures — fn, lambdas, where clauses, memoize
7. Control Flow — if/else, match, for/else, given/otherwise, guard
8. Error Handling — try/catch, retry, null coalesce, optional chaining
9. File I/O — FASTA, FASTQ, VCF, BED, GFF readers/writers
10. Genomic Intervals & Variants — interval trees, variant classification
11. Pipelines — pipeline blocks, stages, parallel execution
12. Statistics & Linear Algebra — hypothesis tests, matrices, ODE solvers
13. Bio APIs — NCBI, Ensembl, UniProt, KEGG, STRING, PDB
14. Streams & Scale — lazy evaluation, parallel maps, large files
15. Plugins & Extending — writing plugins, import system
