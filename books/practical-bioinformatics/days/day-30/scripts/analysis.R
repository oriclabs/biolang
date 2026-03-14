library(dplyr)

read_fasta <- function(path) {
  lines <- readLines(path)
  ids <- c()
  seqs <- c()
  current_id <- NULL
  current_seq <- c()
  for (line in lines) {
    if (startsWith(line, ">")) {
      if (!is.null(current_id)) {
        ids <- c(ids, current_id)
        seqs <- c(seqs, paste0(current_seq, collapse = ""))
      }
      current_id <- strsplit(sub("^>", "", line), " ")[[1]][1]
      current_seq <- c()
    } else {
      current_seq <- c(current_seq, line)
    }
  }
  if (!is.null(current_id)) {
    ids <- c(ids, current_id)
    seqs <- c(seqs, paste0(current_seq, collapse = ""))
  }
  data.frame(id = ids, sequence = seqs, stringsAsFactors = FALSE)
}

get_kmers <- function(seq, k) {
  n <- nchar(seq) - k + 1
  if (n <= 0) return(character(0))
  sapply(1:n, function(i) substr(seq, i, i + k - 1))
}

kmer_similarity <- function(seq_a, seq_b, k) {
  ka <- unique(get_kmers(seq_a, k))
  kb <- unique(get_kmers(seq_b, k))
  shared <- length(intersect(ka, kb))
  total <- length(union(ka, kb))
  if (total == 0) return(0)
  round(shared / total, 4)
}

kmer_distance <- function(seq_a, seq_b, k) {
  round(1.0 - kmer_similarity(seq_a, seq_b, k), 4)
}

window_identity <- function(sequences, window_size) {
  ref_seq <- sequences[1]
  ref_len <- nchar(ref_seq)
  n_seqs <- length(sequences)
  ref_chars <- strsplit(ref_seq, "")[[1]]

  results <- data.frame(position = integer(0), conservation = numeric(0))
  for (start in 0:(ref_len - window_size)) {
    end_pos <- start + window_size
    match_scores <- numeric(end_pos - start)
    for (p_idx in seq_len(end_pos - start)) {
      pos <- start + p_idx
      ref_char <- ref_chars[pos]
      match_count <- 0
      for (si in 2:n_seqs) {
        other_chars <- strsplit(sequences[si], "")[[1]]
        if (pos <= length(other_chars) && other_chars[pos] == ref_char) {
          match_count <- match_count + 1
        }
      }
      match_scores[p_idx] <- match_count / (n_seqs - 1)
    }
    results <- rbind(results, data.frame(
      position = start + window_size %/% 2,
      conservation = round(mean(match_scores), 4)
    ))
  }
  results
}

domain_divergence <- function(sequences, species_info, orthologs, d_start, d_end) {
  ref_chars <- strsplit(sequences[1], "")[[1]]
  results <- data.frame(
    species = character(0),
    divergence_mya = numeric(0),
    sub_rate = numeric(0),
    stringsAsFactors = FALSE
  )
  for (i in 2:length(sequences)) {
    other_chars <- strsplit(sequences[i], "")[[1]]
    end_val <- min(d_end, length(ref_chars), length(other_chars))
    positions <- (d_start + 1):end_val
    mismatches <- sum(ref_chars[positions] != other_chars[positions])
    total <- length(positions)
    info <- species_info[species_info$seq_id == orthologs$id[i], ]
    results <- rbind(results, data.frame(
      species = info$common_name[1],
      divergence_mya = as.numeric(info$divergence_mya[1]),
      sub_rate = round(mismatches / total, 4),
      stringsAsFactors = FALSE
    ))
  }
  results
}

dir.create("data/output", recursive = TRUE, showWarnings = FALSE)

orthologs <- read_fasta("data/orthologs.fasta")
species_info <- read.delim("data/species_info.tsv", sep = "\t", stringsAsFactors = FALSE)
domain_annotations <- read.delim("data/domain_annotations.tsv", sep = "\t", stringsAsFactors = FALSE)

species_names <- sapply(orthologs$id, function(sid) {
  species_info$common_name[species_info$seq_id == sid][1]
})
sequences <- orthologs$sequence
n <- length(sequences)

seq_summary <- data.frame(
  species = species_names,
  length_aa = nchar(sequences),
  divergence_mya = sapply(orthologs$id, function(sid) {
    species_info$divergence_mya[species_info$seq_id == sid][1]
  }),
  stringsAsFactors = FALSE
)
write.table(seq_summary, "data/output/sequence_summary.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

human_protein <- sequences[1]
sim_table <- data.frame(
  species = species_names,
  kmer3_sim = sapply(sequences, function(s) kmer_similarity(human_protein, s, 3)),
  kmer5_sim = sapply(sequences, function(s) kmer_similarity(human_protein, s, 5)),
  stringsAsFactors = FALSE
)
sim_table <- sim_table[order(-sim_table$kmer5_sim), ]
write.table(sim_table, "data/output/similarity_table.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

dist_matrix <- matrix(0, n, n)
for (i in 1:n) {
  for (j in 1:n) {
    dist_matrix[i, j] <- kmer_distance(sequences[i], sequences[j], 4)
  }
}
dist_df <- as.data.frame(dist_matrix)
colnames(dist_df) <- species_names
dist_df <- cbind(species = species_names, dist_df)
write.table(dist_df, "data/output/distance_matrix.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

domain_regions <- list(
  list(name = "N-terminal_TAD", start = 0, end = 60),
  list(name = "Proline-rich", start = 60, end = 95),
  list(name = "DNA-binding", start = 95, end = 290),
  list(name = "Tetramerization", start = 320, end = 360),
  list(name = "C-terminal_reg", start = 360, end = 393)
)

vert_mask <- !grepl("fly|worm|yeast", orthologs$id)
vertebrate_seqs <- sequences[vert_mask]

conservation <- window_identity(vertebrate_seqs, 10)

domain_cons <- do.call(rbind, lapply(domain_regions, function(d) {
  region <- conservation[conservation$position >= d$start & conservation$position < d$end, ]
  avg <- if (nrow(region) > 0) mean(region$conservation) else 0
  data.frame(
    domain = d$name,
    start = d$start,
    end_pos = d$end,
    mean_conservation = round(avg, 4),
    stringsAsFactors = FALSE
  )
}))
write.table(domain_cons, "data/output/domain_conservation.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

arch_table <- do.call(rbind, lapply(seq_len(nrow(species_info)), function(i) {
  sp <- species_info[i, ]
  domains <- domain_annotations[domain_annotations$seq_id == sp$seq_id, ]
  data.frame(
    species = sp$common_name,
    n_domains = nrow(domains),
    domains = paste(domains$domain_name, collapse = ", "),
    seq_length = sp$seq_length,
    stringsAsFactors = FALSE
  )
}))
write.table(arch_table, "data/output/domain_architecture.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

dbd <- domain_divergence(sequences, species_info, orthologs, 95, 290)
tad <- domain_divergence(sequences, species_info, orthologs, 0, 60)

evo_rates <- data.frame(
  species = dbd$species,
  divergence_mya = dbd$divergence_mya,
  dbd_rate = dbd$sub_rate,
  tad_rate = tad$sub_rate,
  ratio = round(tad$sub_rate / (dbd$sub_rate + 0.001), 2),
  stringsAsFactors = FALSE
)
write.table(evo_rates, "data/output/evolutionary_rates.tsv", sep = "\t", row.names = FALSE, quote = FALSE)

dbd_val <- domain_cons$mean_conservation[domain_cons$domain == "DNA-binding"]
tet_val <- domain_cons$mean_conservation[domain_cons$domain == "Tetramerization"]
tad_val <- domain_cons$mean_conservation[domain_cons$domain == "N-terminal_TAD"]
mean_ratio <- round(mean(evo_rates$ratio), 2)

summary_lines <- c(
  "=== Multi-Species TP53 Gene Family Analysis ===",
  "",
  paste0("Species analyzed: ", n),
  paste0("Vertebrate orthologs: ", length(vertebrate_seqs)),
  "",
  "Sequence lengths (aa):",
  paste0("  Min: ", min(seq_summary$length_aa)),
  paste0("  Max: ", max(seq_summary$length_aa)),
  paste0("  Mean: ", round(mean(seq_summary$length_aa), 1)),
  "",
  "Domain conservation (vertebrates):",
  paste0("  DNA-binding domain: ", dbd_val),
  paste0("  Tetramerization: ", tet_val),
  paste0("  N-terminal TAD: ", tad_val),
  "",
  "Evolutionary rate ratio (TAD/DBD):",
  paste0("  Mean: ", mean_ratio),
  "  (>1.0 means TAD evolves faster than DBD)",
  "",
  "Output files:",
  "  data/output/sequence_summary.tsv",
  "  data/output/similarity_table.tsv",
  "  data/output/distance_matrix.tsv",
  "  data/output/domain_conservation.tsv",
  "  data/output/domain_architecture.tsv",
  "  data/output/evolutionary_rates.tsv",
  "  data/output/summary.txt"
)

writeLines(summary_lines, "data/output/summary.txt")
