# Day 30: Language Comparison --- Multi-Species Gene Family Analysis

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| FASTA loading | 1 (built-in) | 16 | 20 |
| TSV loading | 2 (built-in) | 6 | 3 |
| Species name lookup | 4 | 3 | 4 |
| K-mer similarity function | 7 | 9 | 8 |
| K-mer distance function | 3 | 2 | 3 |
| Sequence summary table | 7 | 8 | 8 |
| Similarity table | 8 | 10 | 7 |
| Distance matrix | 7 | 7 | 8 |
| Phylogenetic tree | 6 (built-in) | 0 (not built-in) | 0 (needs ape) |
| Window identity function | 18 | 18 | 22 |
| Domain conservation scoring | 9 | 11 | 10 |
| Domain architecture table | 7 | 9 | 10 |
| Evolutionary rate analysis | 18 | 17 | 16 |
| Visualization (dotplot, line, scatter) | 5 (built-in) | 0 (needs matplotlib) | 0 (needs ggplot2) |
| Summary generation | 30 | 30 | 28 |
| **Total** | **~144** | **~156** | **~147** |

## Key Differences

### FASTA Parsing

```
# BioLang --- single built-in call
let orthologs = read_fasta("data/orthologs.fasta")

# Python --- manual parser (16 lines)
def read_fasta(path):
    sequences = []
    current_id = None
    current_seq = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line.startswith(">"):
                if current_id is not None:
                    sequences.append({"id": current_id, "sequence": "".join(current_seq)})
                current_id = line[1:].split()[0]
                current_seq = []
            else:
                current_seq.append(line)
    if current_id is not None:
        sequences.append({"id": current_id, "sequence": "".join(current_seq)})
    return sequences

# R --- manual parser (20 lines) or Biostrings::readAAStringSet (1 line, but heavy dep)
read_fasta <- function(path) { ... }  # 20 lines
```

### K-mer Similarity

```
# BioLang --- uses built-in kmers() and pipe operators
fn kmer_similarity(seq_a, seq_b, k) {
    let kmers_a = kmers(seq_a, k)
    let kmers_b = kmers(seq_b, k)
    let set_a = kmers_a |> sort() |> filter(|x| true)
    let set_b = kmers_b |> sort() |> filter(|x| true)
    let shared = set_a |> filter(|kmer| set_b |> filter(|b| b == kmer) |> len() > 0) |> len()
    let total = len(set_a) + len(set_b) - shared
    round(float(shared) / float(total), 4)
}

# Python --- uses set operations (cleaner for this task)
def kmer_similarity(seq_a, seq_b, k):
    kmers_a = set(get_kmers(seq_a, k))
    kmers_b = set(get_kmers(seq_b, k))
    shared = len(kmers_a & kmers_b)
    total = len(kmers_a | kmers_b)
    return round(shared / total, 4)

# R --- uses unique() and intersect/union
kmer_similarity <- function(seq_a, seq_b, k) {
    ka <- unique(get_kmers(seq_a, k))
    kb <- unique(get_kmers(seq_b, k))
    shared <- length(intersect(ka, kb))
    total <- length(union(ka, kb))
    round(shared / total, 4)
}
```

### Phylogenetic Tree

```
# BioLang --- built-in (one line produces SVG)
phylo_tree(labels, matrix, "data/output/phylo_tree.svg")

# Python --- requires BioPython or scipy for neighbor-joining
# Not included in stdlib analysis

# R --- requires ape package
# library(ape); nj_tree <- nj(as.dist(dist_matrix)); plot(nj_tree)
```

### Visualization

```
# BioLang --- built-in visualization (no imports needed)
dotplot(seq_a, seq_b, "output.svg")
line(table, "x", "y", "output.svg")
scatter(table, "x", "y", "output.svg")

# Python --- requires matplotlib (not in stdlib)
# import matplotlib.pyplot as plt; plt.scatter(...); plt.savefig(...)

# R --- requires ggplot2
# library(ggplot2); ggplot(df, aes(x, y)) + geom_point()
```

## Observations

1. **BioLang advantage: built-in bio I/O.** FASTA parsing is one function call vs 16--20 lines of manual parsing in Python/R (without heavy dependencies like Biopython or Biostrings).

2. **BioLang advantage: built-in visualization.** Dotplots, line plots, scatter plots, and phylogenetic trees are built-in. Python and R require external packages (matplotlib, ggplot2, ape).

3. **Python advantage: set operations.** Python's native set type (`&`, `|`) makes k-mer similarity computation cleaner than BioLang's filter-based approach.

4. **R advantage: vectorized operations.** R's vectorized comparisons (`ref_chars[positions] != other_chars[positions]`) make position-by-position divergence computation more concise.

5. **All three languages** produce equivalent analytical results. The pipeline logic --- k-mer similarity, sliding window conservation, domain-level divergence rates --- is the same across all implementations.

6. **For production phylogenetics**, all three languages would need specialized external tools (RAxML, IQ-TREE, BEAST). BioLang's `phylo_tree()` and R's `ape::nj()` provide quick exploration; Python requires BioPython or dendropy for similar functionality.
