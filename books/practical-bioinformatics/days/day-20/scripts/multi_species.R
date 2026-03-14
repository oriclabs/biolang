#!/usr/bin/env Rscript
# Day 20: Multi-Species Comparison — R equivalent using biomaRt + httr

library(httr)
library(jsonlite)

ENSEMBL_REST <- "https://rest.ensembl.org"

ensembl_symbol <- function(species, symbol) {
  url <- paste0(ENSEMBL_REST, "/lookup/symbol/", species, "/", symbol)
  resp <- GET(url, content_type_json())
  stop_for_status(resp)
  content(resp, "parsed")
}

ensembl_sequence <- function(gene_id, type = "protein") {
  url <- paste0(ENSEMBL_REST, "/sequence/id/", gene_id, "?type=", type)
  resp <- GET(url, content_type_json())
  stop_for_status(resp)
  content(resp, "parsed")
}

gc_content <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  sum(chars %in% c("G", "C")) / length(chars)
}

kmers <- function(seq, k) {
  n <- nchar(seq)
  if (n < k) return(character(0))
  sapply(1:(n - k + 1), function(i) substr(seq, i, i + k - 1))
}

kmer_jaccard <- function(seq1, seq2, k) {
  k1 <- unique(kmers(seq1, k))
  k2 <- unique(kmers(seq2, k))
  shared <- length(intersect(k1, k2))
  total <- length(union(k1, k2))
  if (total > 0) round(shared / total, 3) else 0.0
}

aa_composition <- function(seq) {
  residues <- strsplit(seq, "")[[1]]
  total <- length(residues)
  hydrophobic <- sum(residues %in% strsplit("AVLIMFWP", "")[[1]])
  polar <- sum(residues %in% strsplit("STNQYC", "")[[1]])
  charged <- sum(residues %in% strsplit("DEKRH", "")[[1]])
  list(
    hydrophobic = round(hydrophobic / total * 100, 1),
    polar = round(polar / total * 100, 1),
    charged = round(charged / total * 100, 1)
  )
}

cat(strrep("=", 60), "\n")
cat("Day 20: Multi-Species Comparison (R)\n")
cat(strrep("=", 60), "\n")

species <- list(
  list(name = "Human", id = "homo_sapiens"),
  list(name = "Mouse", id = "mus_musculus"),
  list(name = "Chicken", id = "gallus_gallus"),
  list(name = "Zebrafish", id = "danio_rerio")
)

# -- Fetch orthologs --
cat("\n-- Fetching BRCA1 Orthologs --\n\n")
results <- list()
for (sp in species) {
  tryCatch({
    gene <- ensembl_symbol(sp$id, "BRCA1")
    protein <- ensembl_sequence(gene$id, "protein")
    cds <- ensembl_sequence(gene$id, "cdna")
    results[[length(results) + 1]] <- list(
      species = sp$name,
      gene_id = gene$id,
      protein_len = nchar(protein$seq),
      protein_seq = protein$seq,
      cds_len = nchar(cds$seq),
      cds_seq = cds$seq,
      gc = round(gc_content(cds$seq) * 100, 1)
    )
    cat(sprintf("  %s: %s (%d aa)\n", sp$name, gene$id, nchar(protein$seq)))
  }, error = function(e) {
    cat(sprintf("  %s: not found (%s)\n", sp$name, e$message))
  })
}

# -- Comparison table --
cat("\n-- Cross-Species Comparison --\n\n")
df <- data.frame(
  species = sapply(results, `[[`, "species"),
  protein_len = sapply(results, `[[`, "protein_len"),
  cds_len = sapply(results, `[[`, "cds_len"),
  gc_percent = sapply(results, `[[`, "gc"),
  cds_protein_ratio = round(sapply(results, `[[`, "cds_len") / sapply(results, `[[`, "protein_len"), 1)
)
print(df)

# -- K-mer similarity --
cat("\n-- K-mer Jaccard Similarity (k=5) --\n\n")
n <- length(results)
for (i in 1:(n - 1)) {
  for (j in (i + 1):n) {
    sim <- kmer_jaccard(results[[i]]$cds_seq, results[[j]]$cds_seq, 5)
    cat(sprintf("  %s vs %s: %s\n", results[[i]]$species, results[[j]]$species, sim))
  }
}

# -- Amino acid composition --
cat("\n-- Amino Acid Composition --\n\n")
for (r in results) {
  comp <- aa_composition(r$protein_seq)
  cat(sprintf("  %s: hydrophobic=%.1f%%, polar=%.1f%%, charged=%.1f%%\n",
              r$species, comp$hydrophobic, comp$polar, comp$charged))
}

# -- Multi-gene comparison --
cat("\n-- Multi-Gene Comparison --\n\n")
genes <- c("TP53", "BRCA1", "EGFR")
all_results <- data.frame(gene = character(), species = character(), length = integer(),
                          stringsAsFactors = FALSE)
for (gene_symbol in genes) {
  for (sp in species) {
    tryCatch({
      gene <- ensembl_symbol(sp$id, gene_symbol)
      prot <- ensembl_sequence(gene$id, "protein")
      all_results <- rbind(all_results, data.frame(
        gene = gene_symbol,
        species = sp$name,
        length = nchar(prot$seq),
        stringsAsFactors = FALSE
      ))
    }, error = function(e) {
      # Gene may not exist in all species
    })
  }
}
print(all_results)

# -- Export --
dir.create("results", showWarnings = FALSE)
write.csv(df, "results/species_comparison.csv", row.names = FALSE)

fasta_lines <- c()
for (r in results) {
  fasta_lines <- c(fasta_lines,
    paste0(">", r$species, "_BRCA1"),
    r$protein_seq
  )
}
writeLines(fasta_lines, "results/brca1_orthologs.fasta")

cat("\nExported results/species_comparison.csv\n")
cat("Exported results/brca1_orthologs.fasta\n")

cat("\n", strrep("=", 60), "\n")
cat("Done!\n")
cat(strrep("=", 60), "\n")
