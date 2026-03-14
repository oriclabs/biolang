# Day 24: Language Comparison --- Programmatic Database Access

## Line Counts

| Operation | BioLang | Python (requests + Biopython) | R (httr2 + rentrez) |
|-----------|---------|-------------------------------|---------------------|
| Load gene list | 2 | 4 | 2 |
| NCBI gene search | 3 | 10 | 9 |
| Ensembl lookup | 3 | 10 | 12 |
| UniProt search | 3 | 11 | 12 |
| Reactome pathways | 3 | 14 | 16 |
| GO annotations | 3 | 12 | 12 |
| STRING network | 2 | 12 | 12 |
| Annotate one gene | 18 | 22 | 22 |
| Map over gene list | 1 | 3 | 1 |
| Error handling per call | 1 | 4 | 3 |
| Rate limiting | 1 | 1 | 1 |
| Write results | 1 | 5 | 2 |
| **Total script** | **~55** | **~130** | **~140** |

## Key Differences

### API Calls

```
# BioLang --- built-in, one line
let gene = ncbi_gene("TP53")

# Python --- import, configure, call, parse
from Bio import Entrez
Entrez.email = "user@example.com"
handle = Entrez.esearch(db="gene", term="TP53[Gene Name] AND Homo sapiens[ORGN]")
result = Entrez.read(handle)
handle.close()

# R --- library, build request, parse
library(rentrez)
result <- entrez_search(db = "gene",
                        term = "TP53[Gene Name] AND Homo sapiens[ORGN]")
```

### Error Handling

```
# BioLang --- try/catch returns value
let gene = try { ncbi_gene("TP53") } catch err { nil }

# Python --- try/except with explicit None
try:
    gene = ncbi_gene("TP53")
except Exception:
    gene = None

# R --- tryCatch with function
gene <- tryCatch(ncbi_gene_search("TP53"), error = function(e) NULL)
```

### Multi-Database Annotation

```
# BioLang --- pipe + map, 2 lines
let symbols = genes |> map(|row| row.symbol)
let annotations = symbols |> map(|s| annotate_gene(s))

# Python --- list comprehension or loop, 4 lines
symbols = [g["symbol"] for g in genes]
annotations = []
for s in symbols:
    annotations.append(annotate_gene(s))

# R --- lapply + rbind, 2 lines
symbols <- genes$symbol
annotations <- do.call(rbind, lapply(symbols, annotate_gene))
```

### Rate Limiting

```
# BioLang --- sleep() inline
sleep(200)

# Python --- time.sleep()
import time
time.sleep(0.2)

# R --- Sys.sleep()
Sys.sleep(0.2)
```

## Setup Comparison

| Aspect | BioLang | Python | R |
|--------|---------|--------|---|
| Dependencies | None (built-in) | requests, biopython | httr2, jsonlite, rentrez |
| API configuration | None | Entrez.email required | None |
| HTTP handling | Automatic | Manual (requests) | Manual (httr2) |
| JSON parsing | Automatic | Manual (resp.json()) | Manual (resp_body_json()) |
| Rate limiting | sleep() | time.sleep() | Sys.sleep() |

## Strengths

### BioLang
- All database clients are built-in --- zero setup
- HTTP, JSON parsing, and response mapping handled automatically
- `try/catch` is an expression, so error handling fits in one line
- Pipe syntax makes annotation pipelines read top-to-bottom

### Python
- Biopython's Entrez module is mature and well-documented
- requests library is extremely flexible for custom API calls
- Large ecosystem of bioinformatics packages (bioservices, mygene, etc.)

### R
- Bioconductor packages (biomaRt, AnnotationDbi) provide deep integration
- rentrez is purpose-built for NCBI E-utilities
- Tidyverse pipes (`|>`) work similarly to BioLang pipes for data manipulation
