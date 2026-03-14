# Day 20: Language Comparison --- Multi-Species Comparison

## Line Counts

| Operation | BioLang | Python (requests) | R (httr + jsonlite) |
|-----------|---------|-------------------|---------------------|
| Fetch orthologs (4 species) | 12 | 20 | 22 |
| GC content comparison | 2 | 8 | 10 |
| K-mer Jaccard function | 6 | 8 | 10 |
| Pairwise k-mer comparison | 5 | 4 | 6 |
| Amino acid composition | 10 | 8 | 10 |
| Comparison table + CSV | 8 | 15 | 10 |
| Phylogenetic tree display | 1 | 10+ (ete3/dendropy) | 8+ (ape/ggtree) |
| Export to FASTA | 2 | 4 | 6 |
| Multi-gene comparison | 12 | 16 | 20 |
| **Total script** | **~80** | **~130** | **~140** |

## Key Differences

### Fetching Orthologs

```
# BioLang — two function calls per species
let gene = ensembl_symbol(sp.id, "BRCA1")
let protein = ensembl_sequence(gene.id, type: "protein")

# Python — manual HTTP requests, JSON parsing, error handling
url = f"https://rest.ensembl.org/lookup/symbol/{species}/{symbol}"
resp = requests.get(url, headers={"Content-Type": "application/json"})
resp.raise_for_status()
gene = resp.json()
url = f"https://rest.ensembl.org/sequence/id/{gene['id']}?type=protein"
# ... same pattern again

# R — similar manual HTTP
url <- paste0("https://rest.ensembl.org/lookup/symbol/", species, "/", symbol)
resp <- GET(url, content_type_json())
stop_for_status(resp)
gene <- content(resp, "parsed")
```

BioLang wraps Ensembl REST calls into typed functions (`ensembl_symbol`, `ensembl_sequence`) that handle HTTP, JSON parsing, and error formatting internally. Python and R require manual URL construction and response parsing.

### K-mer Similarity

```
# BioLang — built-in kmers(), set(), intersection(), union()
let k1 = set(kmers(seq1, k))
let k2 = set(kmers(seq2, k))
let shared = len(intersection(k1, k2))

# Python — list comprehension + set operations
k1 = set(seq1[i:i+k] for i in range(len(seq1) - k + 1))
k2 = set(seq2[i:i+k] for i in range(len(seq2) - k + 1))
shared = len(k1 & k2)

# R — manual k-mer generation + set operations
k1 <- unique(sapply(1:(nchar(seq1)-k+1), function(i) substr(seq1, i, i+k-1)))
k2 <- unique(sapply(1:(nchar(seq2)-k+1), function(i) substr(seq2, i, i+k-1)))
shared <- length(intersect(k1, k2))
```

The logic is similar across languages. BioLang's `kmers()` function is slightly more concise than the Python comprehension and significantly cleaner than R's `sapply` + `substr` pattern.

### Phylogenetic Tree Visualization

```
# BioLang — one function call
phylo_tree(newick_string, title: "Species Phylogeny")

# Python — requires ete3 or dendropy, complex setup
from ete3 import Tree
t = Tree(newick_string)
print(t.get_ascii(show_internal=False))

# R — requires ape package
library(ape)
tree <- read.tree(text = newick_string)
plot(tree)
```

BioLang's `phylo_tree()` renders Newick trees directly in ASCII or SVG. Python requires the `ete3` library (which has complex dependencies including Qt). R's `ape` package is more straightforward but still requires installation.

### Multi-Gene Comparison

```
# BioLang — flat_map for concise multi-gene iteration
let all_results = genes |> flat_map(|g| compare_gene_across_species(g, species))

# Python — nested loops with list extension
all_results = []
for gene_symbol in genes:
    for sp in species:
        # ... try/except block ...
        all_results.append(...)

# R — nested loops with rbind
for (gene_symbol in genes) {
  for (sp in species) {
    tryCatch({ ... all_results <- rbind(all_results, ...) }, ...)
  }
}
```

BioLang's `flat_map` compresses the nested iteration into a single pipeline expression.

## Summary

| Feature | BioLang | Python | R |
|---------|---------|--------|---|
| Ensembl API calls | Built-in typed functions | Manual HTTP | Manual HTTP |
| K-mer generation | `kmers()` builtin | List comprehension | sapply + substr |
| Set operations | `set()`, `intersection()`, `union()` | Native set ops | `intersect()`, `union()` |
| Phylo tree rendering | `phylo_tree()` | Requires ete3 | Requires ape |
| Bar charts | `bar_chart()` | matplotlib | ggplot2/barplot |
| FASTA export | `write_fasta()` | Manual file I/O | Manual file I/O |
| Error handling | `try/catch` | `try/except` | `tryCatch()` |
