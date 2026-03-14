#!/usr/bin/env Rscript
# Day 18: Genomic Coordinates and Intervals (R version)
# Uses GenomicRanges and IRanges from Bioconductor

library(GenomicRanges)
library(IRanges)
library(rtracklayer)

# ── 1. Creating intervals ─────────────────────────────────────────────

cat("=== Creating Intervals ===\n\n")

brca1 <- GRanges(seqnames = "chr17", ranges = IRanges(start = 43044295, end = 43125483))
tp53 <- GRanges(seqnames = "chr17", ranges = IRanges(start = 7668402, end = 7687550))

cat(sprintf("BRCA1: chr17:%d-%d\n", start(brca1), end(brca1)))
cat(sprintf("  Length: %d bp\n", width(brca1)))
cat(sprintf("\nTP53: chr17:%d-%d\n", start(tp53), end(tp53)))
cat(sprintf("  Length: %d bp\n", width(tp53)))

# ── 2. Reading BED files ──────────────────────────────────────────────

cat("\n=== Reading BED Files ===\n\n")

exons <- import("data/exons.bed", format = "BED")
cat(sprintf("Exon regions: %d\n", length(exons)))

total_bp <- sum(width(exons))
cat(sprintf("Total exonic bases: %d\n", total_bp))

cat("\nFirst 5 exons:\n")
for (i in 1:5) {
  cat(sprintf("  %s: %s:%d-%d (%d bp)\n",
              exons$name[i],
              as.character(seqnames(exons)[i]),
              start(exons)[i], end(exons)[i],
              width(exons)[i]))
}

# ── 3. Interval trees and overlap queries ─────────────────────────────

cat("\n=== Interval Trees ===\n\n")

regions <- GRanges(
  seqnames = rep("chr17", 4),
  ranges = IRanges(
    start = c(43044295, 43060000, 43080000, 43100000),
    end = c(43050000, 43070000, 43090000, 43125483)
  )
)

query <- GRanges(seqnames = "chr17", ranges = IRanges(start = 43065000, end = 43085000))
hits <- findOverlaps(query, regions)
cat(sprintf("Query: chr17:43065000-43085000\n"))
cat(sprintf("Overlapping regions: %d\n", length(hits)))

# Bulk overlaps
cat("\nBulk overlap queries:\n")
bulk_queries <- GRanges(
  seqnames = rep("chr17", 3),
  ranges = IRanges(
    start = c(43045000, 43065000, 43095000),
    end = c(43046000, 43066000, 43096000)
  )
)
for (i in 1:3) {
  q <- bulk_queries[i]
  n <- length(findOverlaps(q, regions))
  cat(sprintf("  Query %d: %d overlaps\n", i - 1, n))
}

# Nearest
lonely <- GRanges(seqnames = "chr17", ranges = IRanges(start = 43055000, end = 43056000))
nearest_idx <- nearest(lonely, regions)
cat(sprintf("\nNearest to chr17:43055000-43056000: chr17:%d-%d\n",
            start(regions)[nearest_idx], end(regions)[nearest_idx]))

# ── 4. Variant-in-region filtering ────────────────────────────────────

cat("\n=== Variant-in-Region Filtering ===\n\n")

# Read VCF manually
vcf_lines <- readLines("data/variants.vcf")
vcf_lines <- vcf_lines[!grepl("^#", vcf_lines)]
vcf_df <- read.delim(textConnection(vcf_lines), header = FALSE, sep = "\t")
colnames(vcf_df) <- c("CHROM", "POS", "ID", "REF", "ALT", "QUAL", "FILTER", "INFO", "FORMAT", "SAMPLE1")

variants_gr <- GRanges(
  seqnames = vcf_df$CHROM,
  ranges = IRanges(start = vcf_df$POS, end = vcf_df$POS)
)

overlaps <- findOverlaps(variants_gr, exons)
exonic_idx <- unique(queryHits(overlaps))
exonic_n <- length(exonic_idx)
total_v <- length(variants_gr)

cat(sprintf("Total variants: %d\n", total_v))
cat(sprintf("Exonic variants: %d\n", exonic_n))
cat(sprintf("Intronic/intergenic: %d\n", total_v - exonic_n))

# ── 5. Coordinate conversion ──────────────────────────────────────────

cat("\n=== Coordinate Conversion ===\n\n")

bed_start <- 43044294
bed_end <- 43044295
vcf_pos <- bed_start + 1
cat(sprintf("BED %d-%d -> VCF pos %d\n", bed_start, bed_end, vcf_pos))

vcf_p <- 43044295
bed_s <- vcf_p - 1
bed_e <- vcf_p
cat(sprintf("VCF pos %d -> BED %d-%d\n", vcf_p, bed_s, bed_e))

roundtrip <- bed_s + 1
cat(sprintf("Round-trip VCF pos: %d (should be %d)\n", roundtrip, vcf_p))

# ── 6. Writing BED files ──────────────────────────────────────────────

cat("\n=== Writing BED Files ===\n\n")

high_cov <- GRanges(
  seqnames = rep("chr17", 2),
  ranges = IRanges(start = c(43044295, 43100000), end = c(43050000, 43125483))
)
export(high_cov, "results/high_coverage.bed", format = "BED")
cat("Wrote high-coverage regions to BED file\n")

# ── 7. Exome coverage report ──────────────────────────────────────────

cat("\n=== Exome Coverage Report ===\n\n")

targets <- exons
total_target_bp <- sum(width(targets))
cat(sprintf("Target regions: %d\n", length(targets)))
cat(sprintf("Total target bases: %d\n", total_target_bp))

on_target_hits <- findOverlaps(variants_gr, targets)
on_target_idx <- unique(queryHits(on_target_hits))
on_target_n <- length(on_target_idx)
off_target <- total_v - on_target_n
on_rate <- round(on_target_n / total_v * 100, 1)

cat(sprintf("\nVariant classification:\n"))
cat(sprintf("  On-target:  %d\n", on_target_n))
cat(sprintf("  Off-target: %d\n", off_target))
cat(sprintf("  On-target rate: %.1f%%\n", on_rate))

on_target_df <- vcf_df[on_target_idx, ]
write.csv(on_target_df, "results/on_target_variants.csv", row.names = FALSE)
cat("\nResults saved\n")
cat("\n=== Report complete ===\n")
