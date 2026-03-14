#!/usr/bin/env Rscript
# Day 9: Biological Databases and APIs — R equivalent
#
# Requires: install.packages(c("rentrez", "httr", "jsonlite"))
# Optional: Sys.setenv(NCBI_API_KEY = "your_key_here")
#
# Note: biomaRt is optional and can be slow to load.
# This script uses direct REST calls for most APIs.

library(rentrez)
library(httr)
library(jsonlite)

cat(strrep("=", 60), "\n")
cat("Day 9: Biological Databases and APIs (R)\n")
cat(strrep("=", 60), "\n")

# Configure NCBI API key if available
ncbi_key <- Sys.getenv("NCBI_API_KEY", unset = "")
if (nchar(ncbi_key) > 0) {
  set_entrez_key(ncbi_key)
}

# ----------------------------------------------------------
# 1. NCBI — Gene Lookup
# ----------------------------------------------------------
cat("\n--- 1. NCBI Gene Lookup ---\n")

search_result <- entrez_search(db = "gene", term = "BRCA1[Gene Name] AND Homo sapiens[Organism]", retmax = 1)
gene_id <- search_result$ids[1]

gene_summary <- entrez_summary(db = "gene", id = gene_id)
cat(sprintf("Symbol: %s\n", gene_summary$nomenclaturesymbol))
cat(sprintf("Name: %s\n", gene_summary$nomenclaturename))
cat(sprintf("Description: %s\n", gene_summary$description))
cat(sprintf("Chromosome: %s\n", gene_summary$chromosome))
cat(sprintf("Location: %s\n", gene_summary$maplocation))

Sys.sleep(0.5)

# ----------------------------------------------------------
# 2. NCBI — PubMed Search
# ----------------------------------------------------------
cat("\n--- 2. NCBI PubMed Search ---\n")

pubmed_search <- entrez_search(db = "pubmed", term = "BRCA1 breast cancer", retmax = 5)
cat(sprintf("PubMed articles found: %d\n", length(pubmed_search$ids)))
for (pmid in pubmed_search$ids) {
  cat(sprintf("  PMID: %s\n", pmid))
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 3. Ensembl — Gene Model
# ----------------------------------------------------------
cat("\n--- 3. Ensembl Gene Model ---\n")

resp <- GET(
  "https://rest.ensembl.org/lookup/symbol/homo_sapiens/BRCA1",
  content_type_json()
)
ens <- content(resp, as = "parsed")
cat(sprintf("Ensembl ID: %s\n", ens$id))
cat(sprintf("Biotype: %s\n", ens$biotype))
cat(sprintf("Position: chr%s:%d-%d\n", ens$seq_region_name, ens$start, ens$end))
cat(sprintf("Strand: %d\n", ens$strand))

Sys.sleep(0.5)

# ----------------------------------------------------------
# 4. Ensembl — Protein Sequence
# ----------------------------------------------------------
cat("\n--- 4. Ensembl Protein Sequence ---\n")

resp <- GET(
  sprintf("https://rest.ensembl.org/sequence/id/%s?type=protein", ens$id),
  content_type_json()
)
prot <- content(resp, as = "parsed")
seq <- prot$seq
cat(sprintf("Protein length: %d amino acids\n", nchar(seq)))
cat(sprintf("First 60 aa: %s\n", substr(seq, 1, 60)))
est_mw <- nchar(seq) * 110
cat(sprintf("Estimated MW: ~%d Da (%.0f kDa)\n", est_mw, est_mw / 1000))

Sys.sleep(0.5)

# ----------------------------------------------------------
# 5. UniProt — Protein Function
# ----------------------------------------------------------
cat("\n--- 5. UniProt Protein Function ---\n")

resp <- GET(
  "https://rest.uniprot.org/uniprotkb/P38398.json",
  add_headers(Accept = "application/json")
)
up <- content(resp, as = "parsed")
protein_name <- up$proteinDescription$recommendedName$fullName$value
organism <- up$organism$scientificName
seq_len <- up$sequence$length
gene_names <- sapply(up$genes, function(g) g$geneName$value)
# Extract function
function_text <- ""
for (comment in up$comments) {
  if (comment$commentType == "FUNCTION" && length(comment$texts) > 0) {
    function_text <- comment$texts[[1]]$value
    break
  }
}
cat(sprintf("Name: %s\n", protein_name))
cat(sprintf("Organism: %s\n", organism))
cat(sprintf("Length: %d aa\n", seq_len))
cat(sprintf("Gene names: %s\n", paste(gene_names, collapse = ", ")))
cat(sprintf("Function: %s...\n", substr(function_text, 1, 120)))

Sys.sleep(0.5)

# ----------------------------------------------------------
# 6. UniProt — Features and Domains
# ----------------------------------------------------------
cat("\n--- 6. UniProt Features ---\n")

features <- up$features
cat(sprintf("Total features: %d\n", length(features)))
domains <- Filter(function(f) f$type == "Domain", features)
cat(sprintf("Domains: %d\n", length(domains)))
for (d in domains) {
  desc <- d$description
  loc_start <- d$location$start$value
  loc_end <- d$location$end$value
  cat(sprintf("  %s (%d..%d)\n", desc, loc_start, loc_end))
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 7. KEGG — Pathway Links
# ----------------------------------------------------------
cat("\n--- 7. KEGG Pathways ---\n")

resp <- GET("https://rest.kegg.jp/find/genes/BRCA1")
kegg_text <- content(resp, as = "text", encoding = "UTF-8")
kegg_lines <- strsplit(trimws(kegg_text), "\n")[[1]]
kegg_lines <- kegg_lines[nchar(kegg_lines) > 0]
cat(sprintf("KEGG gene hits: %d\n", length(kegg_lines)))

resp <- GET("https://rest.kegg.jp/link/pathway/hsa:672")
link_text <- content(resp, as = "text", encoding = "UTF-8")
link_lines <- strsplit(trimws(link_text), "\n")[[1]]
link_lines <- link_lines[nchar(link_lines) > 0]
cat(sprintf("Pathways involving BRCA1: %d\n", length(link_lines)))
for (line in head(link_lines, 5)) {
  parts <- strsplit(line, "\t")[[1]]
  if (length(parts) >= 2) {
    cat(sprintf("  %s\n", parts[2]))
  }
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 8. PDB — 3D Structures
# ----------------------------------------------------------
cat("\n--- 8. PDB Structures ---\n")

resp <- GET("https://data.rcsb.org/rest/v1/core/entry/1JM7")
pdb <- content(resp, as = "parsed")
cat(sprintf("Title: %s\n", pdb$struct$title))
cat(sprintf("Method: %s\n", pdb$exptl[[1]]$method))
resolution <- if (length(pdb$refine) > 0) pdb$refine[[1]]$ls_d_res_high else "N/A"
cat(sprintf("Resolution: %s\n", resolution))

# Search for BRCA1 structures
search_payload <- list(
  query = list(
    type = "terminal",
    service = "full_text",
    parameters = list(value = "BRCA1")
  ),
  return_type = "entry"
)
resp <- POST(
  "https://search.rcsb.org/rcsbsearch/v2/query",
  body = toJSON(search_payload, auto_unbox = TRUE),
  content_type_json()
)
if (status_code(resp) == 200) {
  results <- content(resp, as = "parsed")
  cat(sprintf("Total BRCA1 structures in PDB: %d\n", results$total_count))
} else {
  cat("PDB search failed\n")
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 9. STRING — Protein Interactions
# ----------------------------------------------------------
cat("\n--- 9. STRING Interactions ---\n")

resp <- GET(
  "https://string-db.org/api/json/network",
  query = list(identifiers = "BRCA1", species = 9606)
)
interactions <- content(resp, as = "parsed")
cat(sprintf("Interaction partners: %d\n", length(interactions)))

scores <- sapply(interactions, function(x) x$score)
sorted_idx <- order(scores, decreasing = TRUE)
cat("Top 5 interactors:\n")
for (i in head(sorted_idx, 5)) {
  int <- interactions[[i]]
  cat(sprintf("  %s <-> %s: score=%s\n",
              int$preferredName_A, int$preferredName_B, int$score))
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 10. Gene Ontology
# ----------------------------------------------------------
cat("\n--- 10. Gene Ontology ---\n")

resp <- GET("https://www.ebi.ac.uk/QuickGO/services/ontology/go/terms/GO:0006281")
go_data <- content(resp, as = "parsed")
if (length(go_data$results) > 0) {
  term <- go_data$results[[1]]
  cat(sprintf("GO term: %s (%s)\n", term$name, term$id))
  cat(sprintf("Aspect: %s\n", term$aspect))
}

resp <- GET(
  "https://www.ebi.ac.uk/QuickGO/services/annotation/search",
  query = list(geneProductId = "P38398", limit = 10),
  add_headers(Accept = "application/json")
)
ann_data <- content(resp, as = "parsed")
annotations <- ann_data$results
cat(sprintf("GO annotations for BRCA1: %d\n", length(annotations)))
for (a in head(annotations, 5)) {
  cat(sprintf("  %s: %s (%s)\n", a$goId, a$goName, a$goAspect))
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 11. Reactome Pathways
# ----------------------------------------------------------
cat("\n--- 11. Reactome Pathways ---\n")

resp <- GET(
  "https://reactome.org/ContentService/data/mapping/UniProt/P38398/pathways",
  add_headers(Accept = "application/json")
)
if (status_code(resp) == 200) {
  pathways <- content(resp, as = "parsed")
  human_pathways <- Filter(function(p) {
    !is.null(p$species) && p$species$displayName == "Homo sapiens"
  }, pathways)
  cat(sprintf("Reactome pathways: %d\n", length(human_pathways)))
  for (p in head(human_pathways, 5)) {
    cat(sprintf("  %s: %s\n", p$stId, p$displayName))
  }
} else {
  cat("Reactome query failed\n")
}

Sys.sleep(0.5)

# ----------------------------------------------------------
# 12. Batch Gene Table
# ----------------------------------------------------------
cat("\n--- 12. Batch Gene Table ---\n")

genes <- c("BRCA1", "TP53", "EGFR", "KRAS", "MYC")
rows <- data.frame(gene = character(), chrom = character(), desc = character(),
                   stringsAsFactors = FALSE)
for (symbol in genes) {
  search_result <- entrez_search(db = "gene",
                                  term = paste0(symbol, "[Gene Name] AND Homo sapiens[Organism]"),
                                  retmax = 1)
  if (length(search_result$ids) > 0) {
    gene_summary <- entrez_summary(db = "gene", id = search_result$ids[1])
    rows <- rbind(rows, data.frame(
      gene = symbol,
      chrom = gene_summary$chromosome,
      desc = gene_summary$description,
      stringsAsFactors = FALSE
    ))
  }
  Sys.sleep(0.5)
}

print(rows)

cat("\n")
cat(strrep("=", 60), "\n")
cat("Day 9 complete! (R)\n")
cat(strrep("=", 60), "\n")
