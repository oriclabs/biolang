# Translate first 3 FASTA sequences (trimmed to multiple of 3). Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/sequences.fa")
# readDNAStringSet keeps the entire header line; BioLang and BioPython use the
# identifier up to the first space.
names(seqs) <- sub(" .*$", "", names(seqs))
n <- min(3, length(seqs))
results <- list()
for (i in seq_len(n)) {
    s <- seqs[[i]]
    # seqs[[i]] is a DNAString, and width() is a DNAStringSet method.
    trim_len <- (nchar(s) %/% 3) * 3
    trimmed <- subseq(s, 1, trim_len)
    protein <- as.character(translate(trimmed))
    # BioLang's translate() ends at the first stop codon; Biostrings emits '*'
    # and keeps going. Split on a literal '*' rather than a regex, because '*'
    # needs escaping in a pattern and Biostrings masks base::strsplit.
    protein <- unlist(base::strsplit(protein, "*", fixed = TRUE))[1]
    if (is.na(protein)) protein <- ""
    results[[names(seqs)[i]]] <- protein
}

cat(toJSON(list(translations = results), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
