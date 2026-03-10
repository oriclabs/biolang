# Functions and Closures

Functions are the primary unit of reuse in BioLang. Whether you are writing a
normalisation routine for RNA-seq counts, a scoring function for sequence
alignment, or a filter predicate for variant calls, functions let you name a
computation and invoke it wherever it is needed.

## Defining Functions

A function is introduced with the `fn` keyword, followed by a name, a
parameter list, and a body enclosed in braces.

```biolang
fn gc_content(seq) {
    let gc = seq |> filter(|b| b == "G" || b == "C") |> len()
    gc / seq_len(seq)
}

let ratio = gc_content("ATGCGCTA")
# ratio => 0.5
```

The last expression in the body is the implicit return value. There is no need
for a `return` keyword in the common case.

## Explicit Return

When you need to exit early -- for example after detecting an invalid input --
use `return`.

```biolang
fn median_quality(quals) {
    if len(quals) == 0 then {
        return 0.0
    }
    let sorted = quals |> sort()
    let mid = len(sorted) / 2
    if len(sorted) % 2 == 0 then {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}
```

## Default Parameters

Parameters can carry default values. Callers may omit them.

```biolang
fn trim_reads(records, min_qual: 20, min_len: 36) {
    records
        |> filter(|r| mean(r.quality) >= min_qual)
        |> filter(|r| seq_len(r.seq) >= min_len)
}

# Use defaults
let trimmed = trim_reads(raw_reads)

# Override quality threshold only
let strict = trim_reads(raw_reads, min_qual: 30)
```

Default parameters must appear after positional parameters.

## Named Return Values

You can declare named return fields to make the return shape explicit.

```biolang
fn count_variants(vcf_path) -> (snps: Int, indels: Int, other: Int) {
    let records = read_vcf(vcf_path)
    let snps = 0
    let indels = 0
    let other = 0
    records |> each(|v| {
        let vt = variant_type(v)
        if vt == "Snp" then snps = snps + 1
        else if vt == "Indel" then indels = indels + 1
        else other = other + 1
    })
}
```

## Closures and Lambdas

A closure is an anonymous function written with pipe delimiters. Short closures
use a single expression; longer ones use `{ }` for a block body.

```biolang
# Single-expression closure
let is_high_qual = |v| v.qual >= 30.0

# Block closure
let normalise = |counts, size_factors| {
    let total = counts |> reduce(0, |a, b| a + b)
    counts |> map(|c| (c / total) * 1e6 / size_factors)
}
```

Closures capture variables from the surrounding scope, which makes them ideal
for building specialised predicates on the fly.

```biolang
fn make_depth_filter(min_depth) {
    |record| record.info.DP >= min_depth
}

let deep_enough = make_depth_filter(30)
let filtered = variants |> filter(deep_enough)
```

## Higher-Order Functions

A higher-order function accepts another function (or closure) as an argument,
returns one, or both.

```biolang
fn apply_qc_chain(reads, filters) {
    filters |> reduce(reads, |current, f| current |> filter(f))
}

let filters = [
    |r| mean(r.quality) >= 20,
    |r| seq_len(r.seq) >= 50,
    |r| gc_content(r.seq) < 0.8
]

let passing = apply_qc_chain(raw_reads, filters)
```

Because pipe inserts as the first argument, you can chain higher-order
functions fluently:

```biolang
let top_genes = counts
    |> filter(|g| g.biotype == "protein_coding")
    |> sort_by(|g| -g.tpm)
    |> take(100)
    |> map(|g| g.gene_name)
```

## Where Clauses

A `where` clause attaches a precondition to a function. If the predicate is
false at call time, a runtime error is raised.

```biolang
fn normalise_counts(counts) where len(counts) > 0 {
    let total = counts |> reduce(0, |a, b| a + b)
    counts |> map(|c| c / total)
}

fn log2_fold_change(treated, control) where control > 0.0 {
    log2(treated / control)
}

fn call_consensus(reads) where len(reads) >= 3 {
    let bases = reads |> map(|r| {base: r.base})
    let counts = table(bases) |> group_by("base")
      |> summarize(|base, group| {base: base, n: nrow(group)})
    counts |> sort_by(|g| -g.n) |> first() |> |g| g.base
}
```

Where clauses serve as executable documentation: they state the contract
and enforce it at the boundary.

## The @memoize Decorator

Bioinformatics computations are often called repeatedly with the same inputs --
the same gene queried in an annotation database, the same k-mer scored in a
seed-extension aligner. The `@memoize` decorator caches results keyed by
argument values.

```biolang
@memoize
fn gc_of_kmer(kmer) {
    gc_content(dna(kmer))
}

# First call computes and caches the result.
# Subsequent calls with the same k-mer return instantly.
let g1 = gc_of_kmer("ATCGGC")  # cache miss
let g2 = gc_of_kmer("ATCGGC")  # cache hit
```

Memoization is especially valuable when combined with recursion.

## Decorator Notes

`@memoize` is currently the only built-in decorator. For cross-cutting
concerns like timing or input validation, use wrapper functions or `where`
clauses.

```biolang
# Use a wrapper function for timing
fn timed_align(fastq, ref_genome) {
    let t0 = now()
    let result = align_reads(fastq, ref_genome)
    print("Alignment took " + str(now() - t0) + "s")
    result
}

# Use where clauses for input validation
fn merge_bams(bam_paths) where len(bam_paths) > 0 {
    # ... merge logic ...
}
```

## Recursion

Recursive functions call themselves. BioLang supports standard recursion,
which is useful for tree-structured biological data such as phylogenies and
ontology DAGs.

```biolang
fn tree_leaf_count(node) {
    if len(node.children) == 0 then {
        return 1
    }
    node.children |> map(tree_leaf_count) |> reduce(0, |a, b| a + b)
}
```

Combine `@memoize` with recursion for dynamic-programming-style algorithms:

```biolang
@memoize
fn needleman_wunsch_score(seq1, seq2, i, j, gap_penalty: -2) {
    if i == 0 then { return j * gap_penalty }
    if j == 0 then { return i * gap_penalty }

    let match_score = if seq1[i - 1] == seq2[j - 1] then 1 else -1

    let diag   = needleman_wunsch_score(seq1, seq2, i - 1, j - 1) + match_score
    let up     = needleman_wunsch_score(seq1, seq2, i - 1, j) + gap_penalty
    let left   = needleman_wunsch_score(seq1, seq2, i, j - 1) + gap_penalty

    max(diag, max(up, left))
}

let score = needleman_wunsch_score("AGTACG", "ACATAG", 6, 6)
```

## Example: Memoized K-mer Scoring Function

Counting k-mer matches across many sequences calls the same function millions
of times. Caching the GC computation removes redundant work.

```biolang
@memoize
fn kmer_gc(kmer_str) {
    gc_content(dna(kmer_str))
}

fn score_sequence_kmers(seq, k: 21) {
    let kmer_list = seq |> kmers(k)
    let gc_scores = kmer_list |> map(|km| kmer_gc(to_string(km)))
    {
        n_kmers: len(gc_scores),
        mean_gc: mean(gc_scores),
        high_gc_count: gc_scores |> filter(|g| g > 0.6) |> len()
    }
}

let result = score_sequence_kmers(dna"ATCGATCGCCCCGGGGATATATATCGCGCGATCG")
print(f"K-mers: {result.n_kmers}, Mean GC: {result.mean_gc:.3f}")
```

## Example: Reusable QC Filter Factory

A factory function returns a closure configured with experiment-specific
thresholds. Different sequencing protocols produce different acceptable ranges.

```biolang
fn make_qc_filter(min_qual: 20, min_len: 50, max_n_frac: 0.05) {
    |read| {
        let avg_q = mean(read.quality)
        let n_frac = (read.seq |> filter(|b| b == "N") |> len()) / seq_len(read.seq)
        avg_q >= min_qual && seq_len(read.seq) >= min_len && n_frac <= max_n_frac
    }
}

# Whole-genome: permissive
let wgs_filter = make_qc_filter(min_qual: 15, min_len: 36)

# Amplicon: strict
let amp_filter = make_qc_filter(min_qual: 30, min_len: 100, max_n_frac: 0.01)

let wgs_reads = read_fastq("sample_wgs.fastq.gz") |> filter(wgs_filter)
let amp_reads = read_fastq("sample_amp.fastq.gz") |> filter(amp_filter)

print("WGS passing: " + to_string(len(wgs_reads)))
print("Amplicon passing: " + to_string(len(amp_reads)))
```

## Example: Recursive Phylogenetic Tree Traversal

Phylogenetic trees are naturally recursive. This example collects all leaf taxa
whose branch length from the root exceeds a threshold -- useful for detecting
fast-evolving lineages.

```biolang
fn collect_distant_leaves(node, dist_so_far, threshold) {
    let current_dist = dist_so_far + (node.branch_length ?? 0.0)

    if len(node.children) == 0 then {
        # Leaf node
        if current_dist > threshold then {
            return [{name: node.name, distance: current_dist}]
        } else {
            return []
        }
    }

    # Internal node: recurse into children and concatenate results
    node.children
        |> flat_map(|child| collect_distant_leaves(child, current_dist, threshold))
}

# Build a simple tree as nested records
let tree = {
    name: "root", branch_length: 0.0, children: [
        {name: "primates", branch_length: 0.08, children: [
            {name: "human", branch_length: 0.1, children: []},
            {name: "chimp", branch_length: 0.12, children: []}
        ]},
        {name: "rodents", branch_length: 0.20, children: [
            {name: "mouse", branch_length: 0.35, children: []},
            {name: "rat", branch_length: 0.33, children: []}
        ]}
    ]
}

let fast_evolving = collect_distant_leaves(tree, 0.0, 0.30)

fast_evolving |> each(|taxon|
    print(taxon.name + " => total branch length " + to_string(taxon.distance))
)
# mouse => total branch length 0.55
# rat => total branch length 0.53
```

## Summary

| Feature | Syntax | Use Case |
|---|---|---|
| Function | `fn name(params) { }` | Named, reusable computation |
| Default params | `fn f(x, k: 10) { }` | Flexible call sites |
| Where clause | `fn f(x) where x > 0 { }` | Precondition contracts |
| Closure | `\|x\| expr` | Inline predicates, callbacks |
| Block closure | `\|x\| { stmts }` | Multi-step anonymous logic |
| @memoize | `@memoize fn f() { }` | Cache repeated computations |
| Recursion | Self-call in body | Trees, dynamic programming |

Functions and closures are the building blocks that the rest of BioLang --
pipelines, parallel maps, and the standard library -- is built upon. Master
them and every subsequent chapter becomes easier.
