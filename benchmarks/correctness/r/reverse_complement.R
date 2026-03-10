# Reverse complement all sequences from FASTA. Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/contigs.fa")
results <- sapply(names(seqs), function(nm) {
    as.character(reverseComplement(seqs[[nm]]))
})

cat(toJSON(list(sequences = as.list(results)), auto_unbox = TRUE, pretty = TRUE))
cat("\n")
