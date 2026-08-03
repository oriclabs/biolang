# Rosalind — Algorithmic Heights

All 34 problems, each asserted against the official answer.

The [Algorithmic Heights track](https://rosalind.info/problems/list-view/?location=algorithmic-heights)
is the odd one out among the four. It is not bioinformatics: it is the standard
algorithms course — sorting, heaps, breadth-first search, Dijkstra,
Bellman-Ford, strongly connected components, 2-satisfiability.

That makes it a different kind of test. The other three packs largely measure
the **library**: how much of the domain is already a builtin. Here there is no
builtin to reach for, so every one of these is written in BioLang itself. What
this pack measures is the **language**.

## What it found

Writing it turned up two defects, both now fixed.

**`{}` was not an empty map.** It parsed as an empty block and evaluated to nil,
so opening a tally the obvious way — `let counts = {}` — failed later at the
first use, reporting a type error that named Nil, a type appearing nowhere in
the source. Which of the two meanings applies is now decided by position, as it
is in JavaScript: a statement of its own is a block, a value is an empty map.

**Assignment searched every name in scope.** Each assignment probed for a
`__const_` marker, and each function call probed for four more markers. Those
probes are meant to miss, and a miss made the interpreter look for what the
caller might have meant — a Levenshtein comparison against every binding in
scope, about a thousand of them since the builtins are global. Assigning into a
list had a second problem: the list was read out, edited and written back, so it
was copied in full on every element write and a sort became cubic.

The measured effect on this pack: **the four packs' 115 assertions went from
161 s to 1.4 s.** A 10,000-iteration loop went from 5.5 s to 3.7 ms.

Neither defect is visible in bioinformatics code, where the loops are inside
Rust builtins. It took a track made of nothing but hand-written loops to show
them.

## A caveat worth stating

These solutions use Rosalind's **sample** datasets, which are small — arrays of
five to twelve elements, graphs of three to twelve vertices. The real datasets
go up to n = 10⁵.

Even after the fix, BioLang runs a tight scalar loop about 19× slower than
CPython. An insertion sort over 2,000 elements takes about 1.9 s here against
73 ms in Python. Quadratic solutions at n = 10⁵ are not practical in BioLang
today, and this pack does not pretend otherwise: it demonstrates the algorithms,
not production throughput for them.

Where BioLang wins is the work it was built for. On a 1 Mb FASTA — read,
reverse complement, GC content, 8-mer counts — it takes 44 ms against 184 ms for
pure Python and 235 ms for R, because that work happens in Rust.

## Where more than one answer is correct

Rosalind accepts any valid answer for several of these, so asserting the sample
text would be asserting an accident of one implementation. Those problems check
the definition instead:

| Problem | Why |
|---|---|
| 2SUM | The scan reaches 8 and −8 first and reports `1 5`; the sample reports `2 4`. |
| HEA | Floyd's construction builds `7 3 5 1 2`; the sample shows `7 5 1 3 2`. |
| PAR, PAR3 | Any partition around the pivot qualifies. |
| TS | Any order with every edge pointing forwards. |
| 2SAT | Any satisfying assignment; answers are substituted back into the formula. |

## Running it

```sh
bl run packs/rosalind-algorithmic-heights/examples/dij.bl   # one problem
bl test packs/rosalind-algorithmic-heights                  # check every answer
```

## Licensing

Solutions are MIT, like the rest of BioLang. Problem statements belong to
[rosalind.info](https://rosalind.info) — each file paraphrases Given/Return in
a line or two and links to the original rather than reproducing the prose.
