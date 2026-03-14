#!/usr/bin/env Rscript
# Day 24: Programmatic Database Access - R equivalent

library(httr2)
library(jsonlite)
library(rentrez)

ENSEMBL_BASE <- "https://rest.ensembl.org"
UNIPROT_BASE <- "https://rest.uniprot.org"
REACTOME_BASE <- "https://reactome.org/ContentService"
QUICKGO_BASE <- "https://www.ebi.ac.uk/QuickGO/services"
STRING_BASE <- "https://string-db.org/api"

ncbi_gene_search <- function(symbol) {
  tryCatch({
    result <- entrez_search(db = "gene",
                            term = paste0(symbol, "[Gene Name] AND Homo sapiens[ORGN]"))
    if (result$count > 0) {
      return(list(id = result$ids[1], found = TRUE))
    }
    return(NULL)
  }, error = function(e) NULL)
}

ensembl_symbol_lookup <- function(species, symbol) {
  tryCatch({
    resp <- request(paste0(ENSEMBL_BASE, "/lookup/symbol/", species, "/", symbol)) |>
      req_headers("Content-Type" = "application/json") |>
      req_timeout(10) |>
      req_perform()
    if (resp_status(resp) == 200) {
      data <- resp_body_json(resp)
      return(data$id)
    }
    return(NULL)
  }, error = function(e) NULL)
}

uniprot_search_fn <- function(query) {
  tryCatch({
    resp <- request(paste0(UNIPROT_BASE, "/uniprotkb/search")) |>
      req_url_query(query = query, format = "json", size = 1) |>
      req_timeout(10) |>
      req_perform()
    if (resp_status(resp) == 200) {
      data <- resp_body_json(resp)
      if (length(data$results) > 0) {
        return(data$results[[1]]$primaryAccession)
      }
    }
    return(NULL)
  }, error = function(e) NULL)
}

reactome_pathways_fn <- function(symbol) {
  tryCatch({
    resp <- request(paste0(REACTOME_BASE, "/search/query")) |>
      req_url_query(query = symbol, species = "Homo sapiens", types = "Pathway") |>
      req_headers(Accept = "application/json") |>
      req_timeout(10) |>
      req_perform()
    if (resp_status(resp) == 200) {
      data <- resp_body_json(resp)
      pathways <- c()
      for (group in data$results) {
        for (entry in group$entries) {
          pathways <- c(pathways, entry$name)
        }
      }
      return(pathways)
    }
    return(character(0))
  }, error = function(e) character(0))
}

go_annotations_fn <- function(symbol) {
  tryCatch({
    resp <- request(paste0(QUICKGO_BASE, "/annotation/search")) |>
      req_url_query(geneProductId = symbol, taxonId = "9606", limit = 50) |>
      req_timeout(10) |>
      req_perform()
    if (resp_status(resp) == 200) {
      data <- resp_body_json(resp)
      return(data$results)
    }
    return(list())
  }, error = function(e) list())
}

string_network_fn <- function(identifiers) {
  tryCatch({
    resp <- request(paste0(STRING_BASE, "/json/network")) |>
      req_url_query(
        identifiers = paste(identifiers, collapse = "%0d"),
        species = 9606
      ) |>
      req_timeout(10) |>
      req_perform()
    if (resp_status(resp) == 200) {
      return(resp_body_json(resp))
    }
    return(NULL)
  }, error = function(e) NULL)
}

annotate_gene <- function(symbol) {
  ncbi <- ncbi_gene_search(symbol)
  Sys.sleep(0.2)

  ensembl_id <- ensembl_symbol_lookup("homo_sapiens", symbol)
  Sys.sleep(0.2)

  uniprot_acc <- uniprot_search_fn(paste0("gene:", symbol, " AND organism_id:9606"))
  Sys.sleep(0.2)

  pathways <- reactome_pathways_fn(symbol)
  Sys.sleep(0.2)

  go <- go_annotations_fn(symbol)
  Sys.sleep(0.2)

  data.frame(
    symbol = symbol,
    ncbi_found = !is.null(ncbi),
    ensembl_id = ifelse(!is.null(ensembl_id), ensembl_id, "N/A"),
    uniprot_found = !is.null(uniprot_acc),
    pathway_count = length(pathways),
    go_term_count = length(go),
    stringsAsFactors = FALSE
  )
}

main <- function() {
  genes <- read.csv("data/gene_list.csv", stringsAsFactors = FALSE)
  symbols <- genes$symbol

  annotations <- do.call(rbind, lapply(symbols, annotate_gene))

  write.table(annotations, "data/annotations.tsv",
              sep = "\t", row.names = FALSE, quote = FALSE)

  up_genes <- genes$symbol[genes$direction == "up"]
  down_genes <- genes$symbol[genes$direction == "down"]

  up_pathways <- c()
  for (g in up_genes) {
    p <- reactome_pathways_fn(g)
    up_pathways <- c(up_pathways, p)
    Sys.sleep(0.2)
  }

  down_pathways <- c()
  for (g in down_genes) {
    p <- reactome_pathways_fn(g)
    down_pathways <- c(down_pathways, p)
    Sys.sleep(0.2)
  }

  sorted_symbols <- sort(symbols)
  top5 <- sorted_symbols[1:5]
  network <- string_network_fn(top5)

  summary_data <- list(
    total_genes = length(symbols),
    upregulated = length(up_genes),
    downregulated = length(down_genes),
    annotations_complete = nrow(annotations),
    up_pathway_hits = length(up_pathways),
    down_pathway_hits = length(down_pathways),
    network_found = !is.null(network)
  )

  write_json(summary_data, "data/summary.json", pretty = TRUE, auto_unbox = TRUE)
}

main()
