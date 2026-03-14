# Day 12: Finding Variants in Genomes — R equivalent
# Uses VariantAnnotation / vcfR or base R for VCF parsing

# ── Step 1: Load and Explore VCF ────────────────────────────────────────

cat("=== Step 1: Loading VCF ===\n")

# Simple VCF parser using base R
lines <- readLines("data/variants.vcf")
data_lines <- lines[!grepl("^#", lines) & nchar(lines) > 0]

variants <- do.call(rbind, lapply(data_lines, function(line) {
  fields <- strsplit(line, "\t")[[1]]
  fmt_fields <- strsplit(fields[9], ":")[[1]]
  sample_fields <- strsplit(fields[10], ":")[[1]]
  gt_idx <- which(fmt_fields == "GT")
  gt <- if (length(gt_idx) > 0) sample_fields[gt_idx] else "."
  data.frame(
    chrom = fields[1], pos = as.integer(fields[2]), id = fields[3],
    ref = fields[4], alt = fields[5],
    qual = as.numeric(ifelse(fields[6] == ".", 0, fields[6])),
    filter = fields[7], info = fields[8], gt = gt,
    stringsAsFactors = FALSE
  )
}))

cat(sprintf("Total variants loaded: %d\n", nrow(variants)))

v <- variants[1, ]
cat("First variant:\n")
cat(sprintf("  Chrom: %s\n", v$chrom))
cat(sprintf("  Position: %d\n", v$pos))
cat(sprintf("  ID: %s\n", v$id))
cat(sprintf("  Ref: %s, Alt: %s\n", v$ref, v$alt))
cat(sprintf("  Quality: %s\n", v$qual))
cat(sprintf("  Filter: %s\n", v$filter))
cat("\n")

# ── Step 2: Variant Classification ──────────────────────────────────────

cat("=== Step 2: Variant Classification ===\n")

variant_type <- function(ref_a, alt_a) {
  if (nchar(ref_a) == 1 && nchar(alt_a) == 1) return("Snp")
  if (nchar(ref_a) != nchar(alt_a)) return("Indel")
  return("Mnp")
}

variants$type <- mapply(variant_type, variants$ref, variants$alt)

snps <- variants[variants$type == "Snp", ]
indels <- variants[variants$type == "Indel", ]
cat(sprintf("SNPs: %d\n", nrow(snps)))
cat(sprintf("Indels: %d\n", nrow(indels)))

cat("First 10 variants with types:\n")
for (i in 1:min(10, nrow(variants))) {
  v <- variants[i, ]
  cat(sprintf("  %s:%d %s>%s (%s)\n", v$chrom, v$pos, v$ref, v$alt, v$type))
}
cat("\n")

# ── Step 3: Transition/Transversion Ratio ───────────────────────────────

cat("=== Step 3: Ts/Tv Ratio ===\n")

transitions <- c("AG", "GA", "CT", "TC")

is_transition <- function(ref_a, alt_a) {
  paste0(ref_a, alt_a) %in% transitions
}

ts_count <- sum(mapply(is_transition, snps$ref, snps$alt))
tv_count <- nrow(snps) - ts_count
ratio <- ifelse(tv_count > 0, ts_count / tv_count, 0)
cat(sprintf("Ts/Tv ratio: %s\n", round(ratio, 2)))
cat("Expected ~2.0 for whole genome sequencing\n")
cat(sprintf("Transitions: %d\n", ts_count))
cat(sprintf("Transversions: %d\n", tv_count))
cat("\n")

# ── Step 4: Quality Filtering ───────────────────────────────────────────

cat("=== Step 4: Quality Filtering ===\n")

passed <- variants[variants$filter == "PASS", ]
cat(sprintf("PASS variants: %d / %d\n", nrow(passed), nrow(variants)))

high_qual <- variants[variants$filter == "PASS" & variants$qual >= 30, ]
cat(sprintf("PASS + quality >= 30: %d\n", nrow(high_qual)))

low_qual <- variants[variants$filter != "PASS", ]
cat(sprintf("Filtered out (non-PASS): %d\n", nrow(low_qual)))
for (i in seq_len(nrow(low_qual))) {
  v <- low_qual[i, ]
  cat(sprintf("  %s:%d %s>%s qual=%s filter=%s\n",
              v$chrom, v$pos, v$ref, v$alt, v$qual, v$filter))
}
cat("\n")

# ── Step 5: Variant Summary ────────────────────────────────────────────

cat("=== Step 5: Variant Summary ===\n")

cat(sprintf("Total alleles: %d\n", nrow(variants)))
cat(sprintf("  SNPs: %d\n", nrow(snps)))
cat(sprintf("  Indels: %d\n", nrow(indels)))
cat(sprintf("  MNPs: %d\n", sum(variants$type == "Mnp")))
cat(sprintf("  Transitions: %d\n", ts_count))
cat(sprintf("  Transversions: %d\n", tv_count))
cat(sprintf("  Ts/Tv ratio: %s\n", round(ratio, 2)))
cat(sprintf("  Multiallelic: %d\n", sum(grepl(",", variants$alt))))
cat("\n")

# ── Step 6: Chromosome Distribution ────────────────────────────────────

cat("=== Step 6: Chromosome Distribution ===\n")

chrom_counts <- table(variants$chrom)
for (ch in names(sort(chrom_counts))) {
  cat(sprintf("  %s: %d\n", ch, chrom_counts[ch]))
}
cat("\n")

# ── Step 7: Het/Hom Ratio ──────────────────────────────────────────────

cat("=== Step 7: Het/Hom Ratio ===\n")

is_het <- function(gt) {
  sep <- ifelse(grepl("\\|", gt), "\\|", "/")
  alleles <- unlist(strsplit(gt, sep))
  alleles <- alleles[alleles != "."]
  length(alleles) >= 2 && length(unique(alleles)) > 1
}

is_hom_alt <- function(gt) {
  sep <- ifelse(grepl("\\|", gt), "\\|", "/")
  alleles <- unlist(strsplit(gt, sep))
  alleles <- alleles[alleles != "."]
  length(alleles) >= 2 && length(unique(alleles)) == 1 && alleles[1] != "0"
}

het_count <- sum(sapply(variants$gt, is_het))
hom_count <- sum(sapply(variants$gt, is_hom_alt))
hh_ratio <- ifelse(hom_count > 0, het_count / hom_count, 0)
cat(sprintf("Het/Hom ratio: %s\n", round(hh_ratio, 2)))
cat("Expected ~1.5-2.0 for diploid organisms\n")
cat(sprintf("Heterozygous: %d\n", het_count))
cat(sprintf("Homozygous alt: %d\n", hom_count))
cat("\n")

# ── Step 8: VEP Annotation ─────────────────────────────────────────────

cat("=== Step 8: VEP Annotation (requires internet) ===\n")
cat("  Skipping in R version (use httr + Ensembl REST API)\n")
cat("\n")

# ── Step 9: Complete Filtering Pipeline ─────────────────────────────────

cat("=== Step 9: Complete Filtering Pipeline ===\n")

pipeline_results <- high_qual[, c("chrom", "pos", "id", "ref", "alt", "qual", "type")]

cat(sprintf("Variants after filtering: %d\n", nrow(pipeline_results)))
cat(sprintf("  Filtered SNPs: %d\n", sum(pipeline_results$type == "Snp")))
cat(sprintf("  Filtered Indels: %d\n", sum(pipeline_results$type == "Indel")))

dir.create("results", showWarnings = FALSE)
write.csv(pipeline_results, "results/classified_variants.csv", row.names = FALSE)
cat("Results saved to results/classified_variants.csv\n")
cat("\n")

cat("=== Analysis Complete ===\n")
