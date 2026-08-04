# Levenshtein distance between the first five sequences, pairwise. Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/sequences.fa")
names(seqs) <- sub(" .*$", "", names(seqs))
n <- min(5, length(seqs))
ids <- names(seqs)[1:n]
subs <- sapply(1:n, function(i) substr(as.character(seqs[[i]]), 1, 300))

pairs <- list()
idx <- 1
for (i in 1:(n - 1)) {
    for (j in (i + 1):n) {
        # adist() is base R's edit distance, independent of Biostrings.
        d <- as.integer(adist(subs[i], subs[j])[1, 1])
        pairs[[idx]] <- list(a = ids[i], b = ids[j], distance = d)
        idx <- idx + 1
    }
}

cat(toJSON(list(pairs = pairs), auto_unbox = TRUE, pretty = TRUE, digits = 10))
cat("\n")
