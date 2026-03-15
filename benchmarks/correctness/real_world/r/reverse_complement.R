library(Biostrings)
library(jsonlite)
seqs <- readDNAStringSet("real_data/yeast_genome.fa")
results <- list()
for (i in 1:min(5, length(seqs))) {
  sub_seq <- subseq(seqs[[i]], 1, min(200, width(seqs[[i]])))
  original <- as.character(sub_seq)
  rc <- as.character(reverseComplement(sub_seq))
  id <- sub("\\s.*", "", names(seqs)[i])
  results[[i]] <- list(id = id, original = toupper(original), revcomp = toupper(rc))
}
cat(toJSON(list(sequences = results), auto_unbox = TRUE))
cat("\n")
