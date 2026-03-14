#!/usr/bin/env Rscript
# Day 3: Biology Crash Course for Developers — R equivalent
# Uses Biostrings for sequence operations
# Install: Rscript r/install.R

suppressPackageStartupMessages({
  library(Biostrings)
})

cat(strrep("=", 60), "\n")
cat("Day 3: Biology Crash Course for Developers\n")
cat(strrep("=", 60), "\n")

# ── Section 1: DNA — The Source Code ─────────────────────────

cat("\n--- DNA: The Source Code ---\n")

coding <- DNAString("ATGCGATCG")
comp <- complement(coding)
rc <- reverseComplement(coding)
cat(sprintf("Coding:     5'-%s-3'\n", as.character(coding)))
cat(sprintf("Complement: 3'-%s-5'\n", as.character(comp)))
cat(sprintf("RevComp:    5'-%s-3'\n", as.character(rc)))

# ── Section 2: The Central Dogma ─────────────────────────────

cat("\n--- The Central Dogma: DNA -> RNA -> Protein ---\n")

seq <- DNAString("ATGGCTAACTGA")
rna <- RNAString(gsub("T", "U", as.character(seq)))
protein <- translate(seq)
# Remove trailing stop
protein_str <- sub("\\*$", "", as.character(protein))
cat(sprintf("DNA:     %s\n", as.character(seq)))
cat(sprintf("RNA:     %s\n", as.character(rna)))
cat(sprintf("Protein: %s\n", protein_str))
cat("M = Methionine (start), A = Alanine, N = Asparagine\n")

# ── Section 3: Codon Usage ───────────────────────────────────

cat("\n--- Codon Usage ---\n")

gene_str <- "ATGGCTGCTTCTGATTGA"
gene <- DNAString(gene_str)
codons <- sapply(seq(1, nchar(gene_str), 3), function(i) substr(gene_str, i, i+2))
usage <- table(codons)
cat(sprintf("Sequence: %s\n", gene_str))
cat("Usage:    ")
cat(paste(names(usage), usage, sep=": ", collapse=", "), "\n")

# ── Section 4: Mutations ────────────────────────────────────

cat("\n--- Mutations: Bugs in the Code ---\n")

normal <- DNAString("ATGGCTAACTGA")
mutant <- DNAString("ATGGCTGACTGA")  # A->G at position 7

normal_protein <- sub("\\*$", "", as.character(translate(normal)))
mutant_protein <- sub("\\*$", "", as.character(translate(mutant)))

cat(sprintf("Normal DNA:     %s\n", as.character(normal)))
cat(sprintf("Mutant DNA:     %s\n", as.character(mutant)))
cat(sprintf("Normal protein: %s\n", normal_protein))
cat(sprintf("Mutant protein: %s\n", mutant_protein))
cat(sprintf("Changed:        %s\n", normal_protein != mutant_protein))
cat("One base change (A->G) changed Asparagine (N) to Aspartate (D)\n")

# ── Section 5: Wobble Position Experiment ────────────────────

cat("\n--- Wobble Position Experiment ---\n")
cat("Mutating each position of codon GCT (Alanine):\n")

translate_no_stop <- function(dna_str) {
  sub("\\*$", "", as.character(translate(DNAString(dna_str))))
}

cat(sprintf("Original (GCT): %s\n", translate_no_stop("ATGGCTTGA")))
cat(sprintf("Pos1 mut (TCT): %s\n", translate_no_stop("ATGTCTTGA")))
cat(sprintf("Pos2 mut (GAT): %s\n", translate_no_stop("ATGGATTGA")))
cat(sprintf("Pos3 mut (GCA): %s\n", translate_no_stop("ATGGCATGA")))
cat("Position 3 (wobble) is most tolerant — GCA still encodes Alanine\n")

# ── Section 6: Genomic Intervals ────────────────────────────

cat("\n--- Genomic Intervals ---\n")

# R uses GRanges for genomic intervals (GenomicRanges package)
# Here we use simple lists for illustration
brca1 <- list(chrom="chr17", start=43044295, end=43125483)
tp53 <- list(chrom="chr17", start=7668402, end=7687550)

cat(sprintf("BRCA1: %s:%d-%d\n", brca1$chrom, brca1$start, brca1$end))
cat(sprintf("TP53:  %s:%d-%d\n", tp53$chrom, tp53$start, tp53$end))
cat(sprintf("Same chromosome: %s\n", brca1$chrom == tp53$chrom))

egfr <- list(chrom="chr7", start=55019017, end=55211628)
braf <- list(chrom="chr7", start=140719327, end=140924929)

same_chrom <- egfr$chrom == braf$chrom
overlaps <- same_chrom && egfr$start < braf$end && braf$start < egfr$end

cat(sprintf("EGFR: %s:%d-%d\n", egfr$chrom, egfr$start, egfr$end))
cat(sprintf("BRAF: %s:%d-%d\n", braf$chrom, braf$start, braf$end))
cat(sprintf("Same chromosome: %s\n", same_chrom))
cat(sprintf("Overlap: %s\n", overlaps))

# ── Section 7: TP53 — The Guardian ──────────────────────────

cat("\n--- TP53: The Guardian of the Genome ---\n")

normal_tp53 <- DNAString("ATGGAGGAGCCGCAGTCAGATCCTAGC")
tp53_prot <- sub("\\*$", "", as.character(translate(normal_tp53)))
gc <- letterFrequency(normal_tp53, letters=c("G", "C"))
gc_content <- sum(gc) / nchar(normal_tp53)
cat(sprintf("TP53 coding start: %s\n", as.character(normal_tp53)))
cat(sprintf("Protein begins:    %s\n", tp53_prot))
cat(sprintf("GC content:        %f\n", gc_content))
cat("TP53 is mutated in >50% of all human cancers\n")

# ── Section 8: Exercise 1 — Hand Translation ────────────────

cat("\n--- Exercise 1: Hand Translation ---\n")

ex1 <- DNAString("ATGAAAGCTTGA")
cat(sprintf("Sequence: %s\n", as.character(ex1)))
cat("Codons:   ATG | AAA | GCT | TGA\n")
cat("Expected: M     K     A    Stop\n")
cat(sprintf("Result:   %s\n", sub("\\*$", "", as.character(translate(ex1)))))

cat("\n")
cat(strrep("=", 60), "\n")
cat("Day 3 complete!\n")
cat(strrep("=", 60), "\n")
