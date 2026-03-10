# Compute GC content per sequence from a FASTA file. Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/contigs.fa")
gc_per_seq <- sapply(names(seqs), function(nm) {
    s <- as.character(seqs[[nm]])
    nchar_g <- nchar(gsub("[^G]", "", s))
    nchar_c <- nchar(gsub("[^C]", "", s))
    round((nchar_g + nchar_c) / nchar(s), 6)
})

result <- list(
    gc_per_sequence = as.list(gc_per_seq),
    n_sequences = length(seqs)
)
cat(toJSON(result, auto_unbox = TRUE, pretty = TRUE))
cat("\n")
