#!/usr/bin/env Rscript
# Day 4: Coding Crash Course for Biologists — R equivalent
# Uses base R and dplyr for data operations
# Install: Rscript r/install.R

library(dplyr)

cat(strrep("=", 60), "\n")
cat("Day 4: Coding Crash Course for Biologists\n")
cat(strrep("=", 60), "\n")

# ── Section 1: Variables ──────────────────────────────────────

cat("\n--- Variables: Labeling Your Tubes ---\n")

sample_name <- "Patient_042"
concentration <- 23.5
is_contaminated <- FALSE
bases_sequenced <- 3200000

cat(sprintf("Sample: %s\n", sample_name))
cat(sprintf("Concentration: %s ng/uL\n", concentration))
cat(sprintf("Clean: %s\n", !is_contaminated))
cat(sprintf("Bases: %d\n", bases_sequenced))

# ── Section 2: Lists (vectors) ───────────────────────────────

cat("\n--- Lists: Your Sample Rack ---\n")

samples <- c("Control_1", "Control_2", "Treated_1", "Treated_2")
cat(sprintf("Number of samples: %d\n", length(samples)))
cat(sprintf("First sample: %s\n", samples[1]))

updated <- c(samples, "Treated_3")
cat(sprintf("After adding: %d samples\n", length(updated)))
cat(sprintf("Contains Control_1: %s\n", "Control_1" %in% samples))
cat(sprintf("Contains Control_9: %s\n", "Control_9" %in% samples))

# ── Section 3: Records (named lists) ─────────────────────────

cat("\n--- Records: Lab Notebook Entries ---\n")

experiment <- list(
  date = "2024-03-15",
  investigator = "Dr. Chen",
  cell_line = "HeLa",
  treatment = "Doxorubicin",
  concentration_uM = 0.5,
  viability_percent = 72.3
)

cat(sprintf("Cell line: %s\n", experiment$cell_line))
cat(sprintf("Viability: %s%%\n", experiment$viability_percent))

# ── Section 4: Loops ─────────────────────────────────────────

cat("\n--- Loops: Processing Every Sample ---\n")

genes <- c("BRCA1", "TP53", "EGFR", "KRAS", "MYC")
for (gene in genes) {
  cat(sprintf("  Analyzing %s...\n", gene))
}

cat("\nGC content per sequence:\n")

gc_content <- function(seq) {
  bases <- strsplit(toupper(seq), "")[[1]]
  gc_count <- sum(bases %in% c("G", "C"))
  gc_count / length(bases)
}

sequences <- c("ATCGATCG", "GCGCGCGC", "AATTAATT")
for (seq in sequences) {
  gc <- gc_content(seq)
  gc_pct <- round(gc * 100.0, 1)
  cat(sprintf("  %s -> GC: %s%%\n", seq, gc_pct))
}

# ── Section 5: Conditions ────────────────────────────────────

cat("\n--- Conditions: QC Decisions ---\n")

qc_samples <- data.frame(
  name = c("S001", "S002", "S003", "S004"),
  reads = c(25000000, 500000, 18000000, 12000000),
  quality = c(35.2, 28.7, 33.1, 22.0),
  stringsAsFactors = FALSE
)

for (i in seq_len(nrow(qc_samples))) {
  s <- qc_samples[i, ]
  if (s$reads < 1000000) {
    cat(sprintf("  %s: FAIL (too few reads: %d)\n", s$name, s$reads))
  } else if (s$quality < 25.0) {
    cat(sprintf("  %s: FAIL (low quality: %s)\n", s$name, s$quality))
  } else {
    cat(sprintf("  %s: PASS\n", s$name))
  }
}

# ── Section 6: Functions ─────────────────────────────────────

cat("\n--- Functions: Reusable Protocols ---\n")

qc_check <- function(reads, min_reads) {
  if (reads < min_reads) "FAIL" else "PASS"
}

fold_change <- function(control, treated) {
  round(treated / control, 2)
}

cat(sprintf("QC 25M reads: %s\n", qc_check(25000000, 1000000)))
cat(sprintf("QC 500K reads: %s\n", qc_check(500000, 1000000)))
cat(sprintf("Fold change 5.2 -> 12.8: %s\n", fold_change(5.2, 12.8)))
cat(sprintf("Fold change 8.1 -> 7.9: %s\n", fold_change(8.1, 7.9)))

# ── Section 7: Pipes (tidyverse) ─────────────────────────────

cat("\n--- Pipes: Connecting Steps ---\n")

dna_seq <- "ATGCGATCGATCGATCGATCGATCG"
gc_result <- round(gc_content(dna_seq), 3)
cat(sprintf("GC content (piped): %s\n", gc_result))

dna_seqs <- c(
  "ATCGATCGATCG",
  "GCGCGCGCGCGC",
  "ATATATATATATAT",
  "GCGCATATAGCGC",
  "TTTTTAAAAACCCCC"
)

gc_values <- sapply(dna_seqs, gc_content)
gc_rich_count <- sum(gc_values > 0.5)

cat(sprintf("%d out of %d sequences are GC-rich\n", gc_rich_count, length(dna_seqs)))

# ── Section 8: Complete Analysis ──────────────────────────────

cat("\n--- Complete Analysis: Gene Expression ---\n")

expr_data <- data.frame(
  gene = c("BRCA1", "TP53", "EGFR", "MYC", "KRAS"),
  control = c(5.2, 8.1, 3.4, 6.7, 4.1),
  treated = c(12.8, 7.9, 15.2, 6.5, 11.3),
  stringsAsFactors = FALSE
)

results <- expr_data %>%
  mutate(
    fold_change = round(treated / control, 2),
    direction = ifelse(treated > control, "UP", "DOWN")
  )

upregulated <- results %>%
  filter(fold_change > 2.0) %>%
  arrange(desc(fold_change))

cat("=== Upregulated Genes (FC > 2.0) ===\n")
for (i in seq_len(nrow(upregulated))) {
  row <- upregulated[i, ]
  cat(sprintf("  %s: %sx %s\n", row$gene, row$fold_change, row$direction))
}
cat(sprintf("\nTotal: %d of %d genes upregulated\n", nrow(upregulated), nrow(results)))

# ── Summary ───────────────────────────────────────────────────

cat("\n")
cat(strrep("=", 60), "\n")
cat("Day 4 complete! You now know:\n")
cat("  - Variables, lists, records\n")
cat("  - Loops, conditions, functions\n")
cat("  - Pipes for chaining analysis steps\n")
cat(strrep("=", 60), "\n")
