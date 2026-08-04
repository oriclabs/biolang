library(Biostrings)
library(jsonlite)
seqs <- readDNAStringSet("real_data/yeast_genome.fa")
names(seqs) <- sub(" .*$", "", names(seqs))
results <- list()
for (i in 1:min(3, length(seqs))) {
  sub_seq <- subseq(seqs[[i]], 1, min(99, nchar(seqs[[i]])))
  dna_str <- as.character(sub_seq)
  protein <- as.character(translate(sub_seq))
  # BioLang's translate() ends at the first stop codon; Biostrings emits '*'
  # and continues. Split on a literal '*' — Biostrings masks base::strsplit.
  protein <- unlist(base::strsplit(protein, "*", fixed = TRUE))[1]
  if (is.na(protein)) protein <- ""
  id <- sub("\\s.*", "", names(seqs)[i])
  results[[i]] <- list(id = id, dna = toupper(dna_str), protein = protein)
}
cat(toJSON(list(translations = results), auto_unbox = TRUE, digits = 10))
cat("\n")
