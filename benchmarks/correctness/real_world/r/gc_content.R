library(Biostrings)
library(jsonlite)
seqs <- readDNAStringSet("real_data/yeast_genome.fa")
gc <- sapply(seqs, function(s) {
  freq <- alphabetFrequency(s)
  (freq["G"] + freq["C"]) / sum(freq[c("A","C","G","T")])
})
names(gc) <- names(seqs)
# Use just the ID (first word)
ids <- sub("\\s.*", "", names(gc))
names(gc) <- ids
cat(toJSON(list(gc_per_sequence = as.list(gc)), auto_unbox = TRUE))
cat("\n")
