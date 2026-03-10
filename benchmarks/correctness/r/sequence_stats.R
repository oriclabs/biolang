# Compute N50, total length, sequence count, mean GC from FASTA. Output JSON.
library(Biostrings)
library(jsonlite)

seqs <- readDNAStringSet("data/contigs.fa")
lengths <- width(seqs)
total_len <- sum(lengths)

# GC content per sequence
gc_vals <- sapply(seq_along(seqs), function(i) {
    s <- as.character(seqs[[i]])
    nchar_g <- nchar(gsub("[^G]", "", s))
    nchar_c <- nchar(gsub("[^C]", "", s))
    (nchar_g + nchar_c) / nchar(s)
})
mean_gc <- round(mean(gc_vals), 6)

# N50
sorted_lens <- sort(lengths, decreasing = TRUE)
cumsum_lens <- cumsum(sorted_lens)
n50 <- sorted_lens[which(cumsum_lens >= total_len / 2)[1]]

result <- list(
    n_sequences = length(seqs),
    total_length = total_len,
    n50 = n50,
    min_length = min(lengths),
    max_length = max(lengths),
    mean_gc = mean_gc
)
cat(toJSON(result, auto_unbox = TRUE, pretty = TRUE))
cat("\n")
