# PDB Structures & Enrichment Analysis

BioLang provides direct access to the RCSB Protein Data Bank and built-in gene set enrichment analysis.

## PDB Structures

### Fetching Entries

```
let entry = pdb_entry("4HHB")
print(entry.title)          # "THE CRYSTAL STRUCTURE OF HUMAN DEOXYHAEMOGLOBIN"
print(entry.method)         # "X-RAY DIFFRACTION"
print(entry.resolution)     # 1.74
print(entry.organism)       # "Homo sapiens"
print(entry.release_date)   # "1984-07-17"
```

### Searching

```
let ids = pdb_search("insulin receptor")
print("Found " + str(len(ids)) + " structures")
ids |> take(5) |> each(|id| print(id))
```

### Chains and Entities

```
# Get all polymer entities (chains) in a structure
let chains = pdb_chains("4HHB")
chains |> each(|c|
    print(c.description + " (" + c.entity_type + "): " + str(len(c.sequence)) + " residues")
)

# Get a specific entity
let entity = pdb_entity("4HHB", 1)
print(entity.description)
print(entity.sequence)

# Get sequence as Protein value
let seq = pdb_sequence("4HHB", 1)
print(len(seq))             # sequence length
```

### Real-World Example: Compare Hemoglobin Chains

```
let alpha = pdb_entity("4HHB", 1)
let beta = pdb_entity("4HHB", 2)
print("Alpha chain: " + str(len(alpha.sequence)) + " residues")
print("Beta chain: " + str(len(beta.sequence)) + " residues")
print("Alpha type: " + alpha.entity_type)
```

## PubMed Integration

Search PubMed and retrieve abstracts:

```
# Search for recent CRISPR papers
let results = pubmed_search("CRISPR Cas9 therapy", 5)
print("Total results: " + str(len(results)))

# Fetch abstract for a specific paper
let abstract = pubmed_abstract(results[0])
print(abstract)
```

### Literature Review Pipeline

```
# Search for papers about a gene of interest
let gene = "BRCA1"
let results = pubmed_search(gene + " cancer therapy 2024", 20)

print("Found " + str(len(results)) + " papers for " + gene)
results
  |> take(5)
  |> each(|pmid|
    let text = pubmed_abstract(pmid)
    print("PMID " + pmid + ":")
    print(text)
    print("---")
  )
```

## Enrichment Analysis

### Over-Representation Analysis (ORA)

Test whether your gene list is enriched for specific biological pathways:

```
# Load gene sets (GMT format from MSigDB, GO, KEGG, etc.)
let gene_sets = read_gmt("h.all.v2024.1.Hs.symbols.gmt")

# Your differentially expressed genes
let de_genes = ["BRCA1", "TP53", "CDK2", "CCND1", "RB1", "E2F1", "PCNA", "MCM2"]

# Run ORA with background size (total genes measured)
let results = enrich(de_genes, gene_sets, 20000)

# Filter significant results
results
  |> filter(|r| r.fdr < 0.05)
  |> each(|r| print(r.term + ": p=" + str(r.p_value) + " FDR=" + str(r.fdr)))
```

The `enrich()` function uses the hypergeometric test with Benjamini-Hochberg FDR correction.

Output columns: `term`, `overlap`, `p_value`, `fdr`, `genes`

### Gene Set Enrichment Analysis (GSEA)

For ranked gene lists (e.g., by fold change or t-statistic):

```
# Prepare ranked table with gene and score columns
let ranked = tsv("de_results.tsv")
  |> select(["gene", "log2fc"])
  |> rename({log2fc: "score"})
  |> sort_by(|r| r.score, desc: true)

# Load gene sets
let gene_sets = read_gmt("c2.cp.kegg.v2024.1.Hs.symbols.gmt")

# Run GSEA (1000 permutations)
let results = gsea(ranked, gene_sets)

# Top enriched pathways
results
  |> filter(|r| r.fdr < 0.25)
  |> each(|r| print(r.term + ": NES=" + str(r.nes) + " FDR=" + str(r.fdr)))
```

Output columns: `term`, `es` (enrichment score), `nes` (normalized ES), `p_value`, `fdr`, `leading_edge`

### GMT File Format

The GMT (Gene Matrix Transposed) format is tab-delimited:

```
PATHWAY_NAME\tDescription\tGENE1\tGENE2\tGENE3\t...
```

Standard sources:
- **MSigDB** (Broad Institute) — Hallmark, KEGG, Reactome, GO sets
- **Enrichr** — curated gene set libraries
- **WikiPathways** — community-curated pathways

### Real-World Example: RNA-seq Enrichment Pipeline

```
# 1. Read differential expression results
let de = tsv("deseq2_results.tsv")

# 2. Get significant genes
let sig_genes = de
  |> filter(|r| r.padj < 0.05 and abs(r.log2FoldChange) > 1)
  |> map(|r| r.gene)
  |> collect()

print("Significant DE genes: " + str(len(sig_genes)))

# 3. Load pathway gene sets
let hallmark = read_gmt("h.all.v2024.1.Hs.symbols.gmt")
let kegg = read_gmt("c2.cp.kegg.v2024.1.Hs.symbols.gmt")

# 4. Run ORA on both
let h_results = enrich(sig_genes, hallmark, 20000)
let k_results = enrich(sig_genes, kegg, 20000)

# 5. Report significant pathways
print("\n=== Hallmark Pathways ===")
h_results |> filter(|r| r.fdr < 0.05) |> each(|r|
    print(r.term + " (overlap=" + str(r.overlap) + ", FDR=" + str(r.fdr) + ")")
)

print("\n=== KEGG Pathways ===")
k_results |> filter(|r| r.fdr < 0.05) |> each(|r|
    print(r.term + " (overlap=" + str(r.overlap) + ", FDR=" + str(r.fdr) + ")")
)
```

## Builtin Reference

### PDB

| Function | Args | Returns | Description |
|---|---|---|---|
| `pdb_entry(id)` | Str | Record | Fetch PDB entry metadata |
| `pdb_search(query)` | Str | List[Str] | Full-text search, returns PDB IDs |
| `pdb_entity(id, entity_id)` | Str, Int | Record | Get specific polymer entity |
| `pdb_sequence(id, entity_id)` | Str, Int | Protein | Get entity sequence as Protein value |
| `pdb_chains(id)` | Str | List[Record] | Get all polymer entities for a structure |

### PubMed

| Function | Args | Returns | Description |
|---|---|---|---|
| `pubmed_search(query, [max])` | Str, [Int] | Record{count, ids} | Search PubMed articles |
| `pubmed_abstract(pmid)` | Str | Str | Fetch abstract text |

### Enrichment

| Function | Args | Returns | Description |
|---|---|---|---|
| `read_gmt(path)` | Str | Map{name -> List[Str]} | Parse GMT gene set file |
| `enrich(genes, sets, bg)` | List, Map, Int | Table | ORA with hypergeometric test + BH FDR |
| `ora(genes, sets, bg)` | List, Map, Int | Table | Alias for `enrich()` |
| `gsea(ranked, sets)` | Table, Map | Table | GSEA with permutation test + BH FDR |
