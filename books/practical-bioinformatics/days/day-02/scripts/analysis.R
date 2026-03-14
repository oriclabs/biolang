# Day 2: Your First Language — BioLang (R equivalent)
# Complete analysis script covering all chapter concepts

library(Biostrings)

# --- Variables and Types ---
cat("=== Variables and Types ===\n")
name <- "BRCA1"
length_val <- 81189
gc <- 0.423
is_oncogene <- FALSE
seq <- DNAString("ATGCGATCG")

cat(sprintf("Gene: %s, Length: %d, GC: %s\n", name, length_val, gc))
cat(sprintf("Types: name=%s, length=%s, gc=%s, seq=%s\n",
    class(name), class(length_val), class(gc), class(seq)))

# --- The Pipe Operator ---
cat("\n=== Pipe Operator ===\n")

# R has |> (base pipe, R 4.1+) or %>% (magrittr)
gc_nested <- round(letterFrequency(DNAString("ATCGATCGATCG"),
    letters="GC", as.prob=TRUE), 3)
cat(sprintf("Nested: %s\n", gc_nested))

# Using base pipe (R 4.1+)
gc_piped <- DNAString("ATCGATCGATCG") |>
    letterFrequency(letters="GC", as.prob=TRUE) |>
    round(3)
cat(sprintf("Piped:  %s\n", gc_piped))

# Central dogma
cat("Central dogma:\n")
dna_seq <- DNAString("ATGAAACCCGGG")
rna_seq <- RNAString(gsub("T", "U", as.character(dna_seq)))
protein <- translate(DNAString("ATGAAACCCGGG"))
cat(as.character(protein), "\n")

# Motif search
find_motif <- function(seq_str, motif) {
    positions <- c()
    seq_str <- as.character(seq_str)
    for (i in 1:(nchar(seq_str) - nchar(motif) + 1)) {
        if (substr(seq_str, i, i + nchar(motif) - 1) == motif) {
            positions <- c(positions, i - 1)  # 0-based
        }
    }
    positions
}

positions <- find_motif("ATGATGCCGATG", "ATG")
cat(sprintf("Start codon positions: [%s]\n", paste(positions, collapse=", ")))
cat(sprintf("Found %d start codons\n", length(positions)))

# --- Lists and Records ---
cat("\n=== Lists and Records ===\n")

genes <- c("BRCA1", "TP53", "EGFR", "KRAS")
cat(sprintf("Genes: [%s]\n", paste(genes, collapse=", ")))
cat(sprintf("Count: %d\n", length(genes)))
cat(sprintf("First: %s\n", genes[1]))
cat(sprintf("Last:  %s\n", genes[length(genes)]))

gene <- list(
    name = "TP53",
    chromosome = "17",
    length = 19149,
    is_tumor_suppressor = TRUE
)
cat(sprintf("Gene record: %s on chr%s\n", gene$name, gene$chromosome))

# --- Functions ---
cat("\n=== Functions ===\n")

gc_rich <- function(seq) {
    gc <- letterFrequency(seq, letters="GC", as.prob=TRUE)
    gc > 0.6
}

cat(sprintf("GCGCGCGCATGC is GC-rich: %s\n",
    tolower(gc_rich(DNAString("GCGCGCGCATGC")))))
cat(sprintf("AAAATTTT is GC-rich: %s\n",
    tolower(gc_rich(DNAString("AAAATTTT")))))

# Lambdas (R uses anonymous functions)
double <- function(x) x * 2
cat(sprintf("double(5) = %d\n", double(5)))

# --- Control Flow ---
cat("\n=== Control Flow ===\n")

gc_val <- 0.65
if (gc_val > 0.6) {
    cat("GC-rich region\n")
} else if (gc_val < 0.4) {
    cat("AT-rich region\n")
} else {
    cat("Balanced composition\n")
}

codons <- c("ATG", "GCT", "TAA")
for (codon in codons) {
    cat(sprintf("Codon: %s\n", codon))
}

base <- "A"
result <- switch(base,
    "A" = , "G" = "Purine",
    "C" = , "T" = "Pyrimidine",
    "Unknown"
)
cat(sprintf("%s is a %s\n", base, result))

# --- Higher-Order Functions ---
cat("\n=== Higher-Order Functions ===\n")

sequences <- DNAStringSet(c("ATCG", "GCGCGC", "ATATAT"))

# map (sapply)
gc_values <- sapply(sequences, function(s) {
    letterFrequency(s, letters="GC", as.prob=TRUE)
})
cat(sprintf("GC values: [%s]\n", paste(gc_values, collapse=", ")))

# filter (which + subsetting)
gc_rich_idx <- which(gc_values > 0.4)
gc_rich_seqs <- sequences[gc_rich_idx]
cat(sprintf("GC-rich count: %d\n", length(gc_rich_seqs)))

# each (for loop or invisible(lapply))
cat("All genes:\n")
invisible(lapply(c("BRCA1", "TP53", "EGFR"), function(g) {
    cat(sprintf("  Gene: %s\n", g))
}))

# reduce (Reduce)
total_length <- Reduce("+", sapply(sequences, nchar))
cat(sprintf("Total bases: %d\n", total_length))

# --- Putting It All Together ---
cat("\n=== Mini-Analysis ===\n")

fragments <- list(
    list(name = "exon1", seq = DNAString("ATGCGATCGATCG")),
    list(name = "exon2", seq = DNAString("GCGCGCATATAT")),
    list(name = "exon3", seq = DNAString("TTTTAAAACCCC"))
)

# Find GC-rich exons
frag_gc_vals <- sapply(fragments, function(f) {
    letterFrequency(f$seq, letters="GC", as.prob=TRUE)
})
gc_rich_names <- sapply(fragments[which(frag_gc_vals > 0.5)], function(f) f$name)
cat(sprintf("GC-rich exons: [%s]\n", paste(gc_rich_names, collapse=", ")))

# Summary statistics
frag_gc_rounded <- round(frag_gc_vals, 3)
cat(sprintf("GC contents: [%s]\n", paste(frag_gc_rounded, collapse=", ")))
cat(sprintf("Mean GC: %s\n", round(mean(frag_gc_rounded), 3)))

# Classify each fragment
classify_gc <- function(gc_val) {
    if (gc_val > 0.6) "GC-rich"
    else if (gc_val < 0.4) "AT-rich"
    else "balanced"
}

invisible(lapply(seq_along(fragments), function(i) {
    f <- fragments[[i]]
    gc_f <- round(frag_gc_vals[i], 3)
    cat(sprintf("%s: GC=%s (%s)\n", f$name, gc_f, classify_gc(gc_f)))
}))

# --- Language Comparison Task ---
cat("\n=== BioLang vs Python vs R (same task) ===\n")
seqs <- DNAStringSet(c("ATCGATCG", "GCGCGCGC", "ATATATAT"))
gc_vals <- letterFrequency(seqs, letters="GC", as.prob=TRUE)
rich_idx <- which(gc_vals > 0.5)
for (i in rich_idx) {
    cat(sprintf("%s: %s\n", as.character(seqs[i]), round(gc_vals[i], 3)))
}
