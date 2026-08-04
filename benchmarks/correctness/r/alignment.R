# Global alignment of sequence pairs. Output JSON.
library(Biostrings)
# pairwiseAlignment moved from Biostrings to pwalign in Bioconductor 3.19.
library(pwalign)
library(jsonlite)

seqs <- readDNAStringSet("data/sequences.fa")
names(seqs) <- sub(" .*$", "", names(seqs))
n <- min(4, length(seqs))
subs <- sapply(1:n, function(i) substr(as.character(seqs[[i]]), 1, 120))

results <- list()
for (i in 1:(n - 1)) {
    # nucleotideSubstitutionMatrix gives match 1 / mismatch -1; gapOpening 0
    # with gapExtension 2 is Biostrings' way of writing a flat -2 per gap.
    mat <- pwalign::nucleotideSubstitutionMatrix(match = 1, mismatch = -1, baseOnly = FALSE)
    al <- pwalign::pairwiseAlignment(subs[i], subs[i + 1], type = "global",
                            substitutionMatrix = mat,
                            gapOpening = 0, gapExtension = 2)
    results[[i]] <- list(a = names(seqs)[i], b = names(seqs)[i + 1],
                         score = as.integer(pwalign::score(al)))
}

cat(toJSON(list(alignments = results), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
