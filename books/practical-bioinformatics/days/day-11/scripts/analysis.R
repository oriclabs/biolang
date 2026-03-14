# Day 11: Sequence Comparison — R equivalent
# Uses Biostrings and seqinr for sequence operations

library(Biostrings)

# ── Step 1: Base Composition Analysis ─────────────────────────────────

cat("=== Base Composition Analysis ===\n")

seqs <- list(
  list(name = "E. coli",  seq = "GCGCATCGATCGATCGCG"),
  list(name = "Human",    seq = "ATATCGATCGATATATAT"),
  list(name = "Thermus",  seq = "GCGCGCGCGCGCGCGCGC")
)

for (s in seqs) {
  dna <- DNAString(s$seq)
  freq <- alphabetFrequency(dna)
  gc <- round((freq["G"] + freq["C"]) / nchar(s$seq) * 100, 1)
  cat(sprintf("%s: GC=%s%%, A=%d, T=%d, G=%d, C=%d\n",
              s$name, gc, freq["A"], freq["T"], freq["G"], freq["C"]))
}
cat("\n")

# ── Step 2: K-mer Extraction ──────────────────────────────────────────

cat("=== K-mer Analysis ===\n")

seq_str <- "ATCGATCGATCG"
k <- 3
kmers_list <- sapply(1:(nchar(seq_str) - k + 1), function(i) substr(seq_str, i, i + k - 1))
cat(sprintf("Sequence: %s\n", seq_str))
cat(sprintf("3-mers: %s\n", paste(kmers_list, collapse = ", ")))
cat("\n")

# K-mer frequency
freq_table <- sort(table(kmers_list), decreasing = TRUE)
cat("3-mer frequencies:\n")
print(freq_table)
cat("\n")

# ── Step 3: Alignment-Free Similarity ─────────────────────────────────

cat("=== K-mer Similarity (Jaccard) ===\n")

seq1 <- "ATCGATCGATCGATCG"
seq2 <- "ATCGATCGTTTTGATCG"
k <- 5

get_kmers <- function(s, k) {
  sapply(1:(nchar(s) - k + 1), function(i) substr(s, i, i + k - 1))
}

k1 <- unique(get_kmers(seq1, k))
k2 <- unique(get_kmers(seq2, k))

shared <- intersect(k1, k2)
total <- union(k1, k2)
jaccard <- length(shared) / length(total)

cat(sprintf("Seq1: %s\n", seq1))
cat(sprintf("Seq2: %s\n", seq2))
cat(sprintf("Shared 5-mers: %d\n", length(shared)))
cat(sprintf("Total unique 5-mers: %d\n", length(total)))
cat(sprintf("K-mer similarity: %s%%\n", round(jaccard * 100, 1)))
cat("\n")

# ── Step 4: Motif Finding ─────────────────────────────────────────────

cat("=== Motif Finding ===\n")

motif_seq <- DNAString("ATGATCGATGATCGATGATCG")
atg_matches <- matchPattern("ATG", motif_seq)
cat(sprintf("Sequence: %s\n", as.character(motif_seq)))
cat(sprintf("ATG positions: %s\n", paste(start(atg_matches) - 1, collapse = ", ")))

re_seq <- DNAString("ATCGGAATTCGATCGGGATCCATCG")
ecori <- matchPattern("GAATTC", re_seq)
bamhi <- matchPattern("GGATCC", re_seq)
cat(sprintf("EcoRI sites: %s\n", paste(start(ecori) - 1, collapse = ", ")))
cat(sprintf("BamHI sites: %s\n", paste(start(bamhi) - 1, collapse = ", ")))
cat("\n")

# ── Step 5: Reverse Complement ────────────────────────────────────────

cat("=== Reverse Complement ===\n")

forward <- DNAString("ATGCGATCGATCG")
revcomp <- reverseComplement(forward)
cat(sprintf("Forward:  5'-%s-3'\n", as.character(forward)))
cat(sprintf("RevComp:  5'-%s-3'\n", as.character(revcomp)))

strand_seq <- DNAString("ATCGGAATTCGATCG")
fwd_hits <- matchPattern("GAATTC", strand_seq)
rev_hits <- matchPattern("GAATTC", reverseComplement(strand_seq))
cat(sprintf("Forward strand GAATTC hits: %s\n", paste(start(fwd_hits) - 1, collapse = ", ")))
cat(sprintf("Reverse strand GAATTC hits: %s\n", paste(start(rev_hits) - 1, collapse = ", ")))
cat("\n")

# ── Step 6: Codon Analysis ────────────────────────────────────────────

cat("=== Codon Usage ===\n")

codon_usage <- function(seq_str) {
  codons <- sapply(seq(1, nchar(seq_str) - 2, by = 3), function(i) substr(seq_str, i, i + 2))
  table(codons)
}

human_gene <- "ATGGCTGCTTCTGATAAATGA"
ecoli_gene <- "ATGGCAGCGAGCGATAAATGA"
cat("Human codons:\n")
print(codon_usage(human_gene))
cat("E. coli codons:\n")
print(codon_usage(ecoli_gene))
cat("\n")

# ── Step 7: Similarity Matrix ─────────────────────────────────────────

cat("=== Pairwise Similarity Matrix ===\n")

sequences <- list(
  list(name = "seq1", seq = "ATCGATCGATCGATCG"),
  list(name = "seq2", seq = "ATCGATCGTTTTGATCG"),
  list(name = "seq3", seq = "GCGCGCGCGCGCGCGC")
)

k <- 5
results <- data.frame(seq1 = character(), seq2 = character(), similarity = numeric(),
                       stringsAsFactors = FALSE)

for (s1 in sequences) {
  for (s2 in sequences) {
    k1 <- unique(get_kmers(s1$seq, k))
    k2 <- unique(get_kmers(s2$seq, k))
    shared <- length(intersect(k1, k2))
    total <- length(union(k1, k2))
    sim <- ifelse(total > 0, round(shared / total, 3), 0.0)
    results <- rbind(results, data.frame(seq1 = s1$name, seq2 = s2$name,
                                          similarity = sim, stringsAsFactors = FALSE))
  }
}
print(results)
cat("\n")

cat("=== Analysis Complete ===\n")
