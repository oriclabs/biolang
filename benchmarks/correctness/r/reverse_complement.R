# Reverse complement all sequences from FASTA. Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/sequences.fa")
# readDNAStringSet keeps the entire header line; BioLang and BioPython
# use the identifier up to the first space.
names(seqs) <- sub(" .*$", "", names(seqs))
results <- sapply(names(seqs), function(nm) {
    as.character(reverseComplement(seqs[[nm]]))
})

cat(toJSON(list(sequences = as.list(results)), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
