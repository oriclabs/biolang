# Day 9: Line-of-Code Comparison

## Task: Querying biological databases and combining results programmatically

| Metric | BioLang | Python | R |
|--------|---------|--------|---|
| Total lines | 112 | 198 | 212 |
| Import/setup | 0 | 8 | 6 |
| NCBI gene lookup | 4 | 12 | 8 |
| NCBI PubMed search | 4 | 6 | 5 |
| Ensembl gene model | 4 | 8 | 8 |
| Ensembl protein sequence | 4 | 8 | 8 |
| UniProt entry lookup | 5 | 18 | 18 |
| UniProt features/domains | 6 | 8 | 10 |
| KEGG find + link | 6 | 10 | 14 |
| PDB entry + search | 6 | 20 | 18 |
| STRING interactions | 8 | 10 | 12 |
| Gene Ontology | 6 | 12 | 12 |
| Reactome pathways | 4 | 10 | 12 |
| Combined gene profile | 18 | 30 | 35 |
| Batch query with table | 8 | 16 | 18 |
| Dependencies | 0 (built-in) | 2 (biopython, requests) | 3 (rentrez, httr, jsonlite) |

## Key Differences

### NCBI Gene Lookup
```
BioLang:  let gene = ncbi_gene("BRCA1")
          print(f"Chromosome: {gene.chromosome}")

Python:   handle = Entrez.esearch(db="gene", term="BRCA1[Gene Name]...", retmax=1)
          record = Entrez.read(handle)
          handle.close()
          gene_id = record["IdList"][0]
          handle = Entrez.esummary(db="gene", id=gene_id)
          summary = Entrez.read(handle)
          handle.close()
          doc = summary["DocumentSummarySet"]["DocumentSummary"][0]
          print(f"Chromosome: {doc.get('Chromosome', '?')}")

R:        search_result <- entrez_search(db="gene", term="BRCA1[Gene Name]...", retmax=1)
          gene_summary <- entrez_summary(db="gene", id=search_result$ids[1])
          cat(sprintf("Chromosome: %s\n", gene_summary$chromosome))
```

### Ensembl Gene Lookup
```
BioLang:  let gene = ensembl_symbol("homo_sapiens", "BRCA1")
          print(f"Ensembl ID: {gene.id}")

Python:   resp = requests.get("https://rest.ensembl.org/lookup/symbol/homo_sapiens/BRCA1",
                              headers={"Content-Type": "application/json"})
          resp.raise_for_status()
          ens = resp.json()
          print(f"Ensembl ID: {ens['id']}")

R:        resp <- GET("https://rest.ensembl.org/lookup/symbol/homo_sapiens/BRCA1",
                      content_type_json())
          ens <- content(resp, as="parsed")
          cat(sprintf("Ensembl ID: %s\n", ens$id))
```

### UniProt Function
```
BioLang:  let entry = uniprot_entry("P38398")
          print(f"Function: {entry.function}")

Python:   resp = requests.get("https://rest.uniprot.org/uniprotkb/P38398.json")
          up = resp.json()
          function_text = ""
          for comment in up.get("comments", []):
              if comment.get("commentType") == "FUNCTION":
                  texts = comment.get("texts", [])
                  if texts:
                      function_text = texts[0].get("value", "")
                      break
          print(f"Function: {function_text}")

R:        resp <- GET("https://rest.uniprot.org/uniprotkb/P38398.json")
          up <- content(resp, as="parsed")
          function_text <- ""
          for (comment in up$comments) {
              if (comment$commentType == "FUNCTION") {
                  function_text <- comment$texts[[1]]$value
                  break
              }
          }
          cat(sprintf("Function: %s\n", function_text))
```

### STRING Interactions
```
BioLang:  let network = string_network(["BRCA1"], 9606)
          let top = network |> sort_by(|n| n.score) |> reverse() |> take(5)

Python:   resp = requests.get("https://string-db.org/api/json/network",
                              params={"identifiers": "BRCA1", "species": 9606})
          interactions = resp.json()
          sorted_interactions = sorted(interactions, key=lambda x: x.get("score", 0), reverse=True)[:5]

R:        resp <- GET("https://string-db.org/api/json/network",
                      query=list(identifiers="BRCA1", species=9606))
          interactions <- content(resp, as="parsed")
          scores <- sapply(interactions, function(x) x$score)
          top_idx <- head(order(scores, decreasing=TRUE), 5)
```

### Combining Multiple Databases
```
BioLang:  let gene = ncbi_gene(symbol)
          let ens = ensembl_symbol("homo_sapiens", symbol)
          let network = string_network([symbol], 9606)
          let pathways = reactome_pathways(symbol)
          # All return structured records — no JSON parsing needed

Python:   # 4 separate HTTP calls with different auth, headers, URL patterns
          # 4 different JSON response structures to parse
          # ~30 lines of boilerplate per combined query

R:        # Mix of rentrez functions and raw httr calls
          # Different parsing strategies per API
          # ~35 lines of boilerplate per combined query
```

## Summary

BioLang's built-in API clients eliminate the two biggest sources of boilerplate in database queries: (1) HTTP request construction (URLs, headers, authentication) and (2) response parsing (navigating nested JSON to extract the fields you need). In Python and R, each database requires its own client library or raw HTTP calls, each with a different response format. The UniProt JSON structure alone requires 8-10 lines of nested dictionary traversal to extract the function field. In BioLang, it is `entry.function`.

For this day's tasks, BioLang uses 43% fewer lines than Python and 47% fewer than R. The gap widens with more databases: each new API adds ~4 lines in BioLang versus ~12-18 in Python/R. The combined gene profile function (querying 5 databases for a single gene) is 18 lines in BioLang, 30 in Python, and 35 in R --- with the difference coming entirely from eliminated boilerplate, not from shorter variable names or compressed logic.
