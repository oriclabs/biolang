library(Biostrings)
library(jsonlite)
seqs <- readDNAStringSet("real_data/yeast_genome.fa")
results <- list()
for (i in 1:min(3, length(seqs))) {
  sub_seq <- subseq(seqs[[i]], 1, min(99, width(seqs[[i]])))
  dna_str <- as.character(sub_seq)
  protein <- as.character(translate(sub_seq))
  id <- sub("\\s.*", "", names(seqs)[i])
  results[[i]] <- list(id = id, dna = toupper(dna_str), protein = protein)
}
cat(toJSON(list(translations = results), auto_unbox = TRUE))
cat("\n")
