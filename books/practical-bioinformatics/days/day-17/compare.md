# Day 17: Language Comparison — Protein Analysis

## UniProt Lookup

### BioLang
```
let entry = uniprot_entry("P04637")
print(f"Name: {entry.name}")
print(f"Length: {entry.sequence_length} aa")
let fasta = uniprot_fasta("P04637")
print(f"Sequence: {substr(fasta, 0, 50)}...")
```

### Python (requests)
```python
import requests

url = "https://rest.uniprot.org/uniprotkb/P04637.json"
data = requests.get(url).json()
name = data["proteinDescription"]["recommendedName"]["fullName"]["value"]
length = data["sequence"]["length"]
print(f"Name: {name}")
print(f"Length: {length} aa")

fasta_url = "https://rest.uniprot.org/uniprotkb/P04637.fasta"
lines = requests.get(fasta_url).text.strip().split("\n")
seq = "".join(lines[1:])
print(f"Sequence: {seq[:50]}...")
```

### R (httr2 + jsonlite)
```r
library(httr2)
resp <- request("https://rest.uniprot.org/uniprotkb/P04637.json") |> req_perform()
data <- resp_body_json(resp)
name <- data$proteinDescription$recommendedName$fullName$value
length <- data$sequence$length
cat(sprintf("Name: %s\nLength: %d aa\n", name, length))
```

**BioLang advantage**: Built-in `uniprot_entry()` and `uniprot_fasta()` — no HTTP boilerplate, no JSON parsing, direct field access.

## Protein Features

### BioLang
```
let features = uniprot_features("P04637")
let domains = features |> filter(|f| f.type == "Domain")
for d in domains {
    print(f"  {d.description}: {d.location}")
}
```

### Python
```python
data = requests.get("https://rest.uniprot.org/uniprotkb/P04637.json").json()
for f in data["features"]:
    if f["type"] == "Domain":
        start = f["location"]["start"]["value"]
        end = f["location"]["end"]["value"]
        print(f"  {f['description']}: {start}..{end}")
```

### R
```r
data <- resp_body_json(request("https://rest.uniprot.org/uniprotkb/P04637.json") |> req_perform())
for (f in data$features) {
    if (f$type == "Domain") {
        cat(sprintf("  %s: %s..%s\n", f$description, f$location$start$value, f$location$end$value))
    }
}
```

**BioLang advantage**: One function call returns typed records. Pipe-friendly filtering without nested JSON traversal.

## Amino Acid Composition

### BioLang
```
let seq = protein"MEEPQSDPSVEPPLSQETFSDLWKLL"
let counts = base_counts(seq)
let residues = split(str(seq), "")
let groups = residues |> map(|aa| classify_aa(aa)) |> frequencies()
```

### Python
```python
from collections import Counter
seq = "MEEPQSDPSVEPPLSQETFSDLWKLL"
counts = Counter(seq)
groups = Counter(classify_aa(aa) for aa in seq)
```

### R
```r
seq <- "MEEPQSDPSVEPPLSQETFSDLWKLL"
residues <- strsplit(seq, "")[[1]]
counts <- table(residues)
groups <- table(sapply(residues, classify_aa))
```

**Comparable**: All three languages handle this well. BioLang's `protein"..."` literal adds type safety (validates amino acid characters at parse time).

## PDB Structure Lookup

### BioLang
```
let structure = pdb_entry("1TUP")
print(f"Title: {structure.title}")
print(f"Resolution: {structure.resolution}")
```

### Python
```python
url = "https://data.rcsb.org/rest/v1/core/entry/1TUP"
data = requests.get(url).json()
print(f"Title: {data['struct']['title']}")
print(f"Resolution: {data['rcsb_entry_info']['resolution_combined'][0]}")
```

### R
```r
library(bio3d)
pdb <- read.pdb("1TUP")
cat(sprintf("Title: %s\n", pdb$call$title))
```

**BioLang advantage**: `pdb_entry()` returns a flat record — no nested JSON paths to remember. `pdb_search()` and `pdb_sequence()` provide the same simplicity for search and sequence retrieval.

## Summary

| Task | BioLang | Python | R |
|------|---------|--------|---|
| UniProt lookup | `uniprot_entry(acc)` | ~10 lines (requests + JSON) | ~8 lines (httr2 + jsonlite) |
| Protein features | `uniprot_features(acc)` | ~8 lines (requests + JSON) | ~8 lines |
| GO terms | `uniprot_go(acc)` | ~15 lines (JSON traversal) | ~12 lines |
| PDB structure | `pdb_entry(id)` | ~5 lines (requests) | ~3 lines (bio3d) |
| Sequence type | `protein"..."` (validated) | `str` (no validation) | `character` (no validation) |
| Composition | `base_counts(seq)` | `Counter(seq)` | `table(strsplit(...))` |
| K-mers | `kmers(seq, 3)` | list comprehension | `sapply()` loop |

BioLang provides the most concise protein analysis workflow by combining built-in API clients with native protein sequence types. Python requires manual HTTP requests and JSON parsing. R has strong packages (bio3d, UniProt.ws) but requires package installation and has more verbose syntax for API access.
