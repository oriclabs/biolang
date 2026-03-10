# Translate first 3 FASTA sequences (trimmed to multiple of 3). Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/contigs.fa")
n <- min(3, length(seqs))
results <- list()
for (i in seq_len(n)) {
    s <- seqs[[i]]
    trim_len <- (width(s) %/% 3) * 3
    trimmed <- subseq(s, 1, trim_len)
    protein <- as.character(translate(trimmed))
    results[[names(seqs)[i]]] <- protein
}

cat(toJSON(list(translations = results), auto_unbox = TRUE, pretty = TRUE))
cat("\n")
