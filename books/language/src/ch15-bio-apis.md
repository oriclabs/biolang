# Chapter 13: Biological Database APIs

Bioinformatics analysis rarely lives in isolation. You query NCBI for gene
annotations, check UniProt for protein function, pull variant consequences from
Ensembl VEP, and look up pathways in KEGG. BioLang provides 16 built-in
database client functions so you can fetch, cross-reference, and integrate
biological data without leaving your script.

All bio API functions are builtins. No imports are needed.

## NCBI

The National Center for Biotechnology Information hosts PubMed, Gene, Nucleotide,
and dozens of other databases. BioLang provides three NCBI functions.

### ncbi_search

Search any NCBI database (PubMed, Gene, Nucleotide, Protein, etc.).

```
# requires: internet connection
# requires: NCBI_API_KEY (optional, increases rate limit)
# Search PubMed for recent CRISPR-Cas9 papers
# ncbi_search(db, query, max_results?)
let ids = ncbi_search("pubmed", "CRISPR-Cas9 delivery 2024", 20)
# ids => ["39012345", "39012346", ...]

# Get summaries for the IDs
let summaries = ncbi_summary(ids, "pubmed")
summaries |> each(|s| {
  print(s.uid)
})
```

### ncbi_gene

Fetch detailed gene information by gene ID or symbol.

```
# requires: internet connection
# requires: NCBI_API_KEY (optional, increases rate limit)
# ncbi_gene(symbol_or_query, max_results?)
let tp53 = ncbi_gene("TP53")
# If a single gene matches, returns a record:
# {id, symbol, name, description, organism, chromosome, location, summary}

print(tp53.name + " on chr" + tp53.chromosome + ": " + tp53.location)
```

### ncbi_sequence

Retrieve nucleotide or protein sequences from NCBI.

```
# requires: internet connection
# requires: NCBI_API_KEY (optional, increases rate limit)
# ncbi_sequence(accession) — returns FASTA text
let fasta = ncbi_sequence("NM_000546.6")
print("FASTA (first 100 chars): " + fasta[0..100])
```

Set the `NCBI_API_KEY` environment variable to get 10 requests/second instead
of the default 3.

## Ensembl

### ensembl_gene

Look up a gene via the Ensembl REST API.

```
# requires: internet connection
# ensembl_symbol(species, symbol) — lookup by symbol
let brca1 = ensembl_symbol("human", "BRCA1")
# Returns: {id, symbol, description, species, biotype, start, end, strand, chromosome}

print("Ensembl ID: " + brca1.id)
print("Location: " + brca1.chromosome + ":" + str(brca1.start) + "-" + str(brca1.end))

# ensembl_gene(ensembl_id) — lookup by Ensembl ID
let same = ensembl_gene("ENSG00000012048")
print("Symbol: " + same.symbol)
```

### ensembl_vep

Predict the functional consequences of variants using the Variant Effect
Predictor.

```
# requires: internet connection
# ensembl_vep(hgvs) — predict variant consequences
let variants = [
  "17:g.43091434C>T",   # BRCA1 splice donor
  "7:g.140753336A>T",   # BRAF V600E
  "12:g.25245350C>A",   # KRAS G12V
]

let predictions = variants |> map(|v| ensembl_vep(v))

# Each result is a list of records with:
# {allele_string, most_severe_consequence, transcript_consequences: [...]}
predictions |> each(|pred| {
  if len(pred) > 0 then {
    let r = pred[0]
    print(r.allele_string + " => " + r.most_severe_consequence)
  }
})
```

## UniProt

### uniprot_search

Search the UniProt protein database.

```
# requires: internet connection
# uniprot_search(query, limit?)
let kinases = uniprot_search("kinase AND organism_id:9606", 50)
# Returns list of records: {accession, name, organism, sequence_length, gene_names, function}

print(str(len(kinases)) + " human kinases found")
kinases |> take(5) |> each(|k| print(k.accession + " " + k.gene_names))
```

### uniprot_entry

Get full details for a single UniProt accession.

```
# requires: internet connection
let entry = uniprot_entry("P04637")  # TP53
# Returns: {accession, name, organism, sequence_length, gene_names, function}

print(entry.name + ": " + str(entry.sequence_length) + " aa")
print("Genes: " + entry.gene_names)
print("Function: " + entry.function)

# Get protein features separately
let features = uniprot_features("P04637")
# Returns list of: {type, location, description}
let domains = features |> filter(|f| f.type == "Domain")
domains |> each(|d| print("  " + d.description + ": " + d.location))
```

## UCSC Genome Browser

### ucsc_sequence

Retrieve genomic sequences from the UCSC Genome Browser.

```
# requires: internet connection
# Get the sequence of the BRCA1 promoter region
# ucsc_sequence(genome, chrom, start, end)
let promoter = ucsc_sequence("hg38", "chr17", 43170245, 43172245)
# Returns DNA sequence as a string

print("BRCA1 promoter length: " + str(len(promoter)) + " bp")
let gc = gc_content(dna(promoter))
print("GC content: " + str(gc))
```

## KEGG

### kegg_find

Search KEGG databases (pathway, enzyme, compound, etc.).

```
# requires: internet connection
# kegg_find(db, query) — search KEGG databases
let pathways = kegg_find("pathway", "apoptosis human")
# Returns list of: {id, description}
```

### kegg_get

Retrieve a specific KEGG entry.

```
# requires: internet connection
# kegg_get(entry_id) — returns raw KEGG text
let apoptosis = kegg_get("hsa04210")
# Returns the KEGG flat-file text for the entry
print("Entry text length: " + str(len(apoptosis)) + " chars")
```

## STRING

### string_network

Query protein-protein interaction networks from the STRING database.

```
# requires: internet connection
# string_network(identifiers, species)
# First argument must be a list of protein names
let network = string_network(["TP53"], 9606)
# Returns list of: {protein_a, protein_b, score}

network |> each(|i| print(i.protein_a + " <-> " + i.protein_b + " (" + str(i.score) + ")"))
```

## PDB

### pdb_entry

Retrieve protein structure metadata from the Protein Data Bank.

```
# requires: internet connection
let structure = pdb_entry("1TUP")  # TP53 DNA-binding domain
# structure => {
#   id: "1TUP",
#   title: "Crystal structure of the p53 core domain...",
#   resolution: 2.2,
#   method: "X-RAY DIFFRACTION",
#   chains: [{id: "A", entity: "Tumor suppressor p53", length: 195}, ...],
#   ligands: [...],
# }

print(structure.title)
print("Resolution: " + str(structure.resolution) + " A")
```

## Reactome

### reactome_pathways

Find pathways associated with a gene or set of genes.

```
# requires: internet connection
# reactome_pathways(gene_or_genes)
let pathways = reactome_pathways("BRCA1")
# Returns pathway records

pathways |> each(|p| print(p))
```

## Gene Ontology

### go_term

Look up a GO term by its identifier.

```
# requires: internet connection
let term = go_term("GO:0006915")
# term => {id: "GO:0006915", name: "apoptotic process",
#           namespace: "biological_process",
#           definition: "A programmed cell death process..."}
```

### go_annotations

Get GO annotations for a gene.

```
# requires: internet connection
# go_annotations(gene_or_accession)
let annotations = go_annotations("TP53")
# Returns list of: {go_id, term, aspect}

annotations |> take(10) |> each(|a| print("  " + a.go_id + " [" + a.aspect + "]: " + a.term))
```

## COSMIC

### cosmic_gene

Query the Catalogue Of Somatic Mutations In Cancer. Requires `COSMIC_API_KEY`.

```
# requires: internet connection
# requires: COSMIC_API_KEY
# cosmic_gene(gene) — requires COSMIC_API_KEY env var
let cosmic = cosmic_gene("BRAF")
# Returns mutation data for the gene

print(cosmic)
```

## NCBI Datasets

### datasets_gene

Use the NCBI Datasets API for gene metadata.

```
# requires: internet connection
# datasets_gene(symbol_or_id)
let info = datasets_gene("EGFR")
print(info)
```

## Environment Variables

Some APIs require or benefit from API keys:

| Variable | Effect |
|---|---|
| `NCBI_API_KEY` | NCBI: 10 req/sec instead of 3 |
| `COSMIC_API_KEY` | Required for COSMIC queries |

Set these in your shell or in a `.env` file before running your script.

## Example: Gene Annotation Pipeline

Fetch gene info from NCBI, cross-reference with UniProt, and pull pathways
from KEGG.

```
# requires: internet connection
# requires: NCBI_API_KEY (optional, increases rate limit)
let gene_list = ["TP53", "BRCA1", "EGFR", "KRAS", "PIK3CA"]

let annotated = gene_list |> map(|symbol| {
  let ncbi = ncbi_gene(symbol)
  let up_hits = uniprot_search("gene:" + symbol + " AND organism_id:9606", 1)
  let prot_len = if len(up_hits) > 0 then up_hits[0].sequence_length else 0

  let pathways = kegg_find("pathway", symbol + " human")
    |> take(3)

  {
    symbol:         symbol,
    name:           ncbi.name,
    chromosome:     ncbi.chromosome,
    location:       ncbi.location,
    protein_length: prot_len,
    pathways:       pathways |> map(|p| p.description),
  }
})

annotated |> each(|g| {
  print(g.symbol + " (" + g.name + ") - chr" + g.chromosome)
  print("  Protein: " + str(g.protein_length) + " aa")
  print("  Pathways: " + str(g.pathways))
})

annotated |> write_json("gene_annotations.json")
```

## Example: Variant Interpretation

Predict variant effects with Ensembl VEP and check for known cancer mutations
in COSMIC.

```
# requires: internet connection
# requires: COSMIC_API_KEY (for cosmic_gene calls)
let variants = tsv("candidate_variants.tsv")
  |> map(|r| r.chrom + ":" + str(r.pos) + ":" + r.ref + ":" + r.alt)

let interpreted = variants |> map(|v| {
  let vep = ensembl_vep(v)

  let worst = vep.consequences
    |> sort_by(|c| c.impact_rank)
    |> first()

  let gene = worst.gene_symbol

  # Check COSMIC if it is a missense or nonsense variant
  let cosmic_info = if worst.consequence == "missense_variant" or
                       worst.consequence == "stop_gained" then
    cosmic_gene(gene)
      |> |cg| cg.mutations
      |> find(|m| m.aa_change == worst.amino_acid_change)
  else
    nil

  {
    variant: v,
    gene: gene,
    consequence: worst.consequence,
    impact: worst.impact,
    sift: worst.sift_prediction,
    polyphen: worst.polyphen_prediction,
    cosmic_count: if cosmic_info != nil then cosmic_info.count else 0,
    cosmic_known: cosmic_info != nil,
  }
})

# Flag high-impact or COSMIC-known variants
let flagged = interpreted
  |> filter(|v| v.impact == "HIGH" or v.cosmic_known)
  |> sort_by(|v| -v.cosmic_count)

print(str(len(flagged)) + " variants flagged for review")
flagged |> write_tsv("flagged_variants.tsv")
```

## Example: Protein Interaction Network

Build a STRING interaction network for a set of differentially expressed genes
and annotate with Reactome pathways.

```
# requires: internet connection
let de_genes = tsv("de_results.tsv")
  |> filter(|r| r.q_value < 0.01 and abs(r.log2fc) > 2.0)
  |> map(|r| r.gene)

# Get STRING interactions for the top 50 DE genes
let top_genes = de_genes |> take(50)
let network = string_network(top_genes, 9606)
# Returns list of: {protein_a, protein_b, score}

print(str(len(network)) + " interactions")

# Find hub genes (most connections)
let all_proteins = network |> map(|i| i.protein_a) + network |> map(|i| i.protein_b)
let unique_proteins = all_proteins |> unique()
let degree = unique_proteins |> map(|p| {
  let edges = network |> filter(|i| i.protein_a == p || i.protein_b == p)
  {gene: p, degree: len(edges)}
})
  |> sort(|a, b| b.degree - a.degree)

print("Hub genes:")
degree |> take(10) |> each(|d| print("  " + d.gene + ": " + str(d.degree) + " interactions"))

# Pathway enrichment for hub genes
let hub_genes = degree |> take(10) |> map(|d| d.gene)
let pathways = reactome_pathways(hub_genes)
  |> filter(|p| p.p_value < 0.05)
  |> sort_by(|p| p.p_value)

print("Enriched pathways:")
pathways |> take(10) |> each(|p| print("  " + p.name + " (p=" + str(p.p_value) + ")"))
```

## Summary

BioLang's built-in bio API functions let you query 12 major biological
databases directly from your scripts. Combine them with pipes, maps, and
filters to build annotation pipelines that cross-reference genes, variants,
proteins, and pathways in a few lines of code. Set API keys via environment
variables to increase rate limits where supported.
