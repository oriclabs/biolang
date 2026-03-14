#!/usr/bin/env Rscript
# Day 17: Protein Analysis — R equivalent
#
# Requires: install.packages(c("httr2", "jsonlite", "bio3d"))

library(httr2)
library(jsonlite)

# ── Step 1: Protein Sequence Basics ──────────────────────────────────

cat("=== Step 1: Protein Sequence Basics ===\n\n")

p53_seq <- "MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDDIEQWFTEDPGPDEAPRMPEAAPPVAPAPAAPTPAAPAPAPSWPLSSSVPSQKTYPQGLNGTVNLPGRNSFEV"
cat(sprintf("Length: %d amino acids\n", nchar(p53_seq)))
cat(sprintf("Type: character\n"))

# ── Step 2: UniProt Lookup ───────────────────────────────────────────

cat("\n=== Step 2: UniProt Lookup ===\n\n")

uniprot_entry <- function(accession) {
    url <- sprintf("https://rest.uniprot.org/uniprotkb/%s.json", accession)
    resp <- request(url) |> req_perform()
    data <- resp_body_json(resp)
    list(
        accession = accession,
        name = data$proteinDescription$recommendedName$fullName$value,
        organism = data$organism$scientificName,
        gene_names = sapply(data$genes, function(g) g$geneName$value),
        sequence_length = data$sequence$length
    )
}

entry <- uniprot_entry("P04637")
cat(sprintf("Protein: %s\n", entry$name))
cat(sprintf("Gene: %s\n", paste(entry$gene_names, collapse = ", ")))
cat(sprintf("Organism: %s\n", entry$organism))
cat(sprintf("Length: %d aa\n", entry$sequence_length))

# Get FASTA sequence
uniprot_fasta <- function(accession) {
    url <- sprintf("https://rest.uniprot.org/uniprotkb/%s.fasta", accession)
    resp <- request(url) |> req_perform()
    lines <- strsplit(resp_body_string(resp), "\n")[[1]]
    paste(lines[-1], collapse = "")
}

fasta <- uniprot_fasta("P04637")
cat(sprintf("\nFirst 60 residues: %s\n", substr(fasta, 1, 60)))
cat(sprintf("Full length: %d aa\n", nchar(fasta)))

# ── Step 3: Amino Acid Composition ───────────────────────────────────

cat("\n=== Step 3: Amino Acid Composition ===\n\n")

seq <- "MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDD"
residues <- strsplit(seq, "")[[1]]
counts <- table(residues)
cat(sprintf("Amino acid counts: %s\n", paste(sprintf("%s:%d", names(counts), counts), collapse = ", ")))

classify_aa <- function(aa) {
    if (aa %in% c("A","V","L","I","M","F","W","P")) return("hydrophobic")
    if (aa %in% c("S","T","N","Q","Y","C")) return("polar")
    if (aa %in% c("K","R","H")) return("positive")
    if (aa %in% c("D","E")) return("negative")
    return("other")
}

groups <- table(sapply(residues, classify_aa))
cat(sprintf("Property distribution: %s\n",
    paste(sprintf("%s:%d", names(groups), groups), collapse = ", ")))

total <- length(residues)
for (group in c("hydrophobic", "polar", "negative", "positive")) {
    count <- ifelse(group %in% names(groups), groups[[group]], 0)
    pct <- round(count / total * 100, 1)
    cat(sprintf("  %s: %d/%d (%.1f%%)\n", group, count, total, pct))
}

# ── Step 4: K-mer Analysis ───────────────────────────────────────────

cat("\n=== Step 4: K-mer Analysis ===\n\n")

seq <- "MEEPQSDPSVEPPLSQETFSDLWKLL"
n <- nchar(seq)
trimers <- sapply(1:(n-2), function(i) substr(seq, i, i+2))
cat(sprintf("Protein 3-mers: %d\n", length(trimers)))
cat(sprintf("First 5 trimers: %s\n", paste(trimers[1:5], collapse = ", ")))

dipeptides <- sapply(1:(n-1), function(i) substr(seq, i, i+1))
dp_counts <- sort(table(dipeptides), decreasing = TRUE)
cat("\nDipeptide counts (top 10):\n")
for (i in 1:min(10, length(dp_counts))) {
    cat(sprintf("  %s: %d\n", names(dp_counts)[i], dp_counts[i]))
}

# ── Step 5: PDB Structure ────────────────────────────────────────────

cat("\n=== Step 5: PDB Structure ===\n\n")

pdb_entry <- function(pdb_id) {
    url <- sprintf("https://data.rcsb.org/rest/v1/core/entry/%s", pdb_id)
    resp <- request(url) |> req_perform()
    data <- resp_body_json(resp)
    list(
        id = pdb_id,
        title = data$struct$title,
        method = data$exptl[[1]]$method,
        resolution = data$rcsb_entry_info$resolution_combined[[1]]
    )
}

structure <- pdb_entry("1TUP")
cat(sprintf("Title: %s\n", structure$title))
cat(sprintf("Resolution: %s angstrom\n", structure$resolution))
cat(sprintf("Method: %s\n", structure$method))

cat("\n=== Done ===\n")
