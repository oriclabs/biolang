# Independent base-R oracle for renderer-independent plot geometry.
# Explicit breaks isolate endpoint/closure/count semantics from R's `pretty()`
# choice, which is a separate conformance question.

json_numbers <- function(values) {
  paste(format(values, digits = 17, scientific = FALSE, trim = TRUE), collapse = ",")
}

json_array <- function(values) {
  paste0("[", json_numbers(values), "]")
}

edge_values <- c(0, 1, 1.5, 2, 3)
edge_breaks <- c(0, 1, 2, 3)
edge_right <- hist(
  edge_values,
  breaks = edge_breaks,
  right = TRUE,
  include.lowest = TRUE,
  plot = FALSE
)
edge_left <- hist(
  edge_values,
  breaks = edge_breaks,
  right = FALSE,
  include.lowest = TRUE,
  plot = FALSE
)
# hist() deliberately errors when include.lowest = FALSE leaves an outer
# endpoint uncounted. Use the same base-R interval primitive directly so the
# excluded endpoint and density denominator remain measurable.
open_geometry <- function(right) {
  membership <- cut(
    edge_values,
    breaks = edge_breaks,
    labels = FALSE,
    right = right,
    include.lowest = FALSE
  )
  counts <- tabulate(membership, nbins = length(edge_breaks) - 1)
  list(
    breaks = edge_breaks,
    counts = counts,
    density = counts / (sum(counts) * diff(edge_breaks))
  )
}
edge_right_open <- open_geometry(TRUE)
edge_left_open <- open_geometry(FALSE)

air <- read.csv("packages/statistics/validation/results/airquality.csv")
air_breaks <- c(0, 25, 50, 75, 100, 125, 150, 175)
air_histogram <- hist(
  air$ozone,
  breaks = air_breaks,
  right = TRUE,
  include.lowest = TRUE,
  plot = FALSE
)

# Geometry fixtures beyond histograms. These compare values that a renderer
# consumes, never pixels. Type-7 boxes match BioLang's documented summary
# convention; the second box uses base R's boxplot.stats/Tukey hinges.
shape_values <- c(1, 2, 2, 3, 4, 100)
box_geometry <- function(values, method) {
  if (method == "tukey") {
    stats <- boxplot.stats(values, coef = 1.5)
    return(list(
      summary = stats$stats,
      outliers = stats$out
    ))
  }
  quartiles <- quantile(values, c(0.25, 0.5, 0.75), type = 7, names = FALSE)
  fences <- c(quartiles[1] - 1.5 * (quartiles[3] - quartiles[1]),
              quartiles[3] + 1.5 * (quartiles[3] - quartiles[1]))
  inside <- values[values >= fences[1] & values <= fences[2]]
  list(
    summary = c(min(inside), quartiles, max(inside)),
    outliers = values[values < min(inside) | values > max(inside)]
  )
}
box_type7 <- box_geometry(shape_values, "type7")
box_tukey <- box_geometry(shape_values, "tukey")

ecdf_counts <- as.numeric(table(shape_values))
ecdf_x <- as.numeric(names(table(shape_values)))
ecdf_cumulative <- cumsum(ecdf_counts)
ecdf_fraction <- ecdf_cumulative / length(shape_values)

qq <- qqnorm(shape_values, plot.it = FALSE)
qq_quartiles <- quantile(shape_values, c(0.25, 0.75), type = 7, names = FALSE)
qq_theoretical <- qnorm(c(0.25, 0.75))
qq_slope <- diff(qq_quartiles) / diff(qq_theoretical)
qq_intercept <- qq_quartiles[1] - qq_slope * qq_theoretical[1]

violin_bandwidth <- bw.nrd0(shape_values)
violin_x <- seq(
  min(shape_values) - 3 * violin_bandwidth,
  max(shape_values) + 3 * violin_bandwidth,
  length.out = 256
)
violin_density <- vapply(
  violin_x,
  function(x) mean(dnorm((x - shape_values) / violin_bandwidth)) / violin_bandwidth,
  numeric(1)
)
violin_scaled <- violin_density / max(violin_density)

# A real, skewed environmental measurement exercises the same geometry away
# from the small controlled edge case. Missing Ozone rows were removed when
# this pinned fixture was exported by reference.R.
air_box_type7 <- box_geometry(air$ozone, "type7")
air_box_tukey <- box_geometry(air$ozone, "tukey")
air_ecdf_counts <- as.numeric(table(air$ozone))
air_ecdf_x <- as.numeric(names(table(air$ozone)))
air_ecdf_cumulative <- cumsum(air_ecdf_counts)
air_ecdf_fraction <- air_ecdf_cumulative / length(air$ozone)
air_qq_sample <- sort(air$ozone)
air_qq <- list(
  x = qnorm(ppoints(length(air_qq_sample))),
  y = air_qq_sample
)
air_qq_quartiles <- quantile(air$ozone, c(0.25, 0.75), type = 7, names = FALSE)
air_qq_slope <- diff(air_qq_quartiles) / diff(qq_theoretical)
air_qq_intercept <- air_qq_quartiles[1] - air_qq_slope * qq_theoretical[1]
air_violin_bandwidth <- bw.nrd0(air$ozone)
air_violin_x <- seq(
  min(air$ozone) - 3 * air_violin_bandwidth,
  max(air$ozone) + 3 * air_violin_bandwidth,
  length.out = 256
)
air_violin_density <- vapply(
  air_violin_x,
  function(x) mean(dnorm((x - air$ozone) / air_violin_bandwidth)) / air_violin_bandwidth,
  numeric(1)
)
air_violin_scaled <- air_violin_density / max(air_violin_density)

air_fit <- lm(ozone ~ month, data = air)
air_fit_at <- sort(unique(air$month))
air_fit_confidence <- predict(
  air_fit,
  newdata = data.frame(month = air_fit_at),
  interval = "confidence",
  level = 0.95
)
air_fit_prediction <- predict(
  air_fit,
  newdata = data.frame(month = air_fit_at),
  interval = "prediction",
  level = 0.95
)

# Clinical plot geometry. `survfit` is the independent product-limit and
# Greenwood oracle. The ROC AUC uses the equivalent Mann-Whitney definition,
# with half credit for tied positive/negative scores.
clinical_survival_data <- data.frame(
  time = c(1, 2, 2, 1, 3, 4),
  event = c(1, 0, 1, 0, 1, 0),
  arm = c("control", "control", "control", "treated", "treated", "treated")
)
clinical_survival <- summary(
  survival::survfit(
    survival::Surv(time, event) ~ arm,
    data = clinical_survival_data
  ),
  censored = TRUE
)
clinical_scores <- c(0.9, 0.8, 0.8, 0.1)
clinical_labels <- c(1, 1, 0, 0)
positive_scores <- clinical_scores[clinical_labels == 1]
negative_scores <- clinical_scores[clinical_labels == 0]
clinical_auc <- (
  sum(outer(positive_scores, negative_scores, ">")) +
    0.5 * sum(outer(positive_scores, negative_scores, "=="))
) / (length(positive_scores) * length(negative_scores))
clinical_forest <- data.frame(
  estimate = c(0.72, 1.18),
  lower = c(0.51, 0.94),
  upper = c(1.01, 1.49),
  weight = c(4, 9)
)

# Genomic-association geometry. These formulas are deliberately independent of
# BioLang: first-observed chromosome offsets are assembled with base-R loops,
# Q-Q envelopes use qbeta order statistics, and rainfall distances use an
# explicit stable chromosome/position/source-row ordering.
association <- data.frame(
  chrom = c("chr2", "chr1", "chr2"),
  pos = c(200, 100, 50),
  pvalue = c(0.01, 1e-9, 0.5),
  stringsAsFactors = FALSE
)
association_order <- unique(association$chrom)
association_lengths <- vapply(
  association_order,
  function(chromosome) max(association$pos[association$chrom == chromosome]),
  numeric(1)
)
association_offsets <- numeric(length(association_order))
if (length(association_order) > 1) {
  for (index in 2:length(association_order)) {
    association_offsets[index] <- association_offsets[index - 1] +
      association_lengths[index - 1] * 1.02
  }
}
association_chromosome_index <- match(association$chrom, association_order) - 1
association_genome_position <- association$pos +
  association_offsets[association_chromosome_index + 1]

genetic_qq_p <- sort(c(0.5, 0.01, 1e-5, 0.2))
genetic_qq_rank <- seq_along(genetic_qq_p)
genetic_qq_expected_p <- (genetic_qq_rank - 0.5) / length(genetic_qq_p)
genetic_qq_tail <- (1 - 0.95) / 2
genetic_qq_lower_p <- qbeta(
  genetic_qq_tail,
  genetic_qq_rank,
  length(genetic_qq_p) - genetic_qq_rank + 1
)
genetic_qq_upper_p <- qbeta(
  1 - genetic_qq_tail,
  genetic_qq_rank,
  length(genetic_qq_p) - genetic_qq_rank + 1
)
genetic_qq_lambda <- median(qchisq(1 - genetic_qq_p, df = 1)) /
  qchisq(0.5, df = 1)

rainfall_input <- data.frame(
  chrom = c("chr2", "chr1", "chr2", "chr2", "chr1"),
  pos = c(100, 10, 100, 400, 30),
  source_row = 0:4,
  stringsAsFactors = FALSE
)
rainfall_chromosome_order <- unique(rainfall_input$chrom)
rainfall_sorted <- rainfall_input[
  order(
    match(rainfall_input$chrom, rainfall_chromosome_order),
    rainfall_input$pos,
    rainfall_input$source_row
  ),
]
rainfall_keep <- c(FALSE, rainfall_sorted$chrom[-1] == rainfall_sorted$chrom[-nrow(rainfall_sorted)])
rainfall_distance <- c(NA_real_, diff(rainfall_sorted$pos))[rainfall_keep]
rainfall_rows <- rainfall_sorted[rainfall_keep,]
rainfall_previous <- rainfall_rows$pos - rainfall_distance
rainfall_plotted <- pmax(rainfall_distance, 1)

# Genomic-track geometry. Cytobands and CNV segments use stable first-observed
# chromosome order. Coverage intervals use half-open overlap and are clipped to
# the requested region rather than selected by midpoint.
ideogram_input <- data.frame(
  chrom = c("chr2", "chr1", "chr2"),
  start = c(50, 0, 0),
  end = c(100, 200, 50),
  source_row = 0:2,
  stringsAsFactors = FALSE
)
ideogram_chromosome_order <- unique(ideogram_input$chrom)
ideogram_chromosome_index <- match(ideogram_input$chrom, ideogram_chromosome_order) - 1
ideogram_lengths <- vapply(
  ideogram_chromosome_order,
  function(chromosome) max(ideogram_input$end[ideogram_input$chrom == chromosome]),
  numeric(1)
)
ideogram_order <- order(
  ideogram_chromosome_index,
  ideogram_input$start,
  ideogram_input$end,
  ideogram_input$source_row
)
ideogram_sorted <- ideogram_input[ideogram_order,]
ideogram_sorted_index <- ideogram_chromosome_index[ideogram_order]

cnv_input <- data.frame(
  chrom = c("chr2", "chr1", "chr2"),
  start = c(100, 0, 0),
  end = c(200, 50, 100),
  log2ratio = c(0.7, -0.4, 0.05),
  source_row = 0:2,
  stringsAsFactors = FALSE
)
cnv_chromosome_order <- unique(cnv_input$chrom)
cnv_chromosome_index <- match(cnv_input$chrom, cnv_chromosome_order) - 1
cnv_lengths <- vapply(
  cnv_chromosome_order,
  function(chromosome) max(cnv_input$end[cnv_input$chrom == chromosome]),
  numeric(1)
)
cnv_offsets <- numeric(length(cnv_chromosome_order))
if (length(cnv_chromosome_order) > 1) {
  for (index in 2:length(cnv_chromosome_order)) {
    cnv_offsets[index] <- cnv_offsets[index - 1] + cnv_lengths[index - 1] * 1.02
  }
}
cnv_order <- order(
  cnv_chromosome_index,
  cnv_input$start,
  cnv_input$end,
  cnv_input$source_row
)
cnv_sorted <- cnv_input[cnv_order,]
cnv_sorted_index <- cnv_chromosome_index[cnv_order]
cnv_genome_start <- cnv_sorted$start + cnv_offsets[cnv_sorted_index + 1]
cnv_genome_end <- cnv_sorted$end + cnv_offsets[cnv_sorted_index + 1]
cnv_state <- ifelse(
  cnv_sorted$log2ratio > 0.25,
  1,
  ifelse(cnv_sorted$log2ratio < -0.25, -1, 0)
)

coverage_input <- data.frame(
  start = c(20, 0, 40),
  end = c(40, 20, 80),
  value = c(8, 3, 5),
  source_row = 0:2
)
coverage_region_start <- 10
coverage_region_end <- 60
coverage_keep <- coverage_input$end > coverage_region_start &
  coverage_input$start < coverage_region_end
coverage_clipped <- coverage_input[coverage_keep,]
coverage_clipped$clipped_start <- pmax(coverage_clipped$start, coverage_region_start)
coverage_clipped$clipped_end <- pmin(coverage_clipped$end, coverage_region_end)
coverage_order <- order(
  coverage_clipped$clipped_start,
  coverage_clipped$clipped_end,
  coverage_clipped$source_row
)
coverage_clipped <- coverage_clipped[coverage_order,]

# Regional annotation and splice geometry is derived independently of the SVG
# renderer. Half-open intervals use the first available non-overlapping lane.
assign_lanes <- function(start, end) {
  lane_ends <- numeric(0)
  lanes <- numeric(length(start))
  for (index in seq_along(start)) {
    available <- which(lane_ends <= start[index])
    if (length(available) == 0) {
      lane <- length(lane_ends) + 1
      lane_ends <- c(lane_ends, -Inf)
    } else {
      lane <- available[1]
    }
    lane_ends[lane] <- end[index]
    lanes[index] <- lane - 1
  }
  lanes
}

genome_input <- data.frame(
  start = c(100, 50, 200),
  end = c(200, 120, 250),
  source_row = 0:2
)
genome_region_start <- 75
genome_region_end <- 225
genome_keep <- genome_input$end > genome_region_start &
  genome_input$start < genome_region_end
genome_sorted <- genome_input[genome_keep,]
genome_sorted$clipped_start <- pmax(genome_sorted$start, genome_region_start)
genome_sorted$clipped_end <- pmin(genome_sorted$end, genome_region_end)
genome_sorted <- genome_sorted[order(
  genome_sorted$clipped_start,
  genome_sorted$clipped_end,
  genome_sorted$source_row
),]
genome_lanes <- assign_lanes(genome_sorted$clipped_start, genome_sorted$clipped_end)

lollipop_input <- data.frame(
  position = c(300, 100, 200),
  height = c(4, 9, 1),
  source_row = 0:2
)
lollipop_sorted <- lollipop_input[order(
  lollipop_input$position,
  lollipop_input$source_row
),]
lollipop_domain <- c(0, 500)

sashimi_coverage <- data.frame(
  position = c(250, 100, 400),
  depth = c(4, 1, 8),
  source_row = 0:2
)
sashimi_coverage <- sashimi_coverage[order(
  sashimi_coverage$position,
  sashimi_coverage$source_row
),]
sashimi_junctions <- data.frame(
  start = c(200, 100, 450),
  end = c(450, 300, 500),
  count = c(25, 100, 4),
  source_row = 0:2
)
sashimi_junctions <- sashimi_junctions[order(
  sashimi_junctions$start,
  sashimi_junctions$end,
  sashimi_junctions$source_row
),]
sashimi_lanes <- assign_lanes(sashimi_junctions$start, sashimi_junctions$end)
sashimi_max_count <- max(1, sashimi_junctions$count)
sashimi_strength <- sqrt(sashimi_junctions$count / sashimi_max_count)
sashimi_arc_fraction <- 0.35 + 0.65 * sashimi_strength
sashimi_stroke_width <- 1 + round(sashimi_strength * 12) / 4

# Circular-genome geometry is computed independently from the same public
# coordinate contract used by circos-style tools: chromosome arc length is
# proportional to chromosome length, with a fixed angular gap between arcs.
circos_segments <- data.frame(
  chromosome_index = 0:1,
  source_row = 0:1,
  start = c(0, 0),
  end = c(100, 50)
)
circos_segments$size <- circos_segments$end - circos_segments$start
circos_gap <- 3 * pi / 180
circos_start_angle <- -pi / 2
circos_available <- 2 * pi - circos_gap * nrow(circos_segments)
circos_segments$angle_start <- numeric(nrow(circos_segments))
circos_segments$angle_end <- numeric(nrow(circos_segments))
circos_angle <- circos_start_angle
for (index in seq_len(nrow(circos_segments))) {
  circos_segments$angle_start[index] <- circos_angle
  circos_segments$angle_end[index] <- circos_angle +
    circos_segments$size[index] / sum(circos_segments$size) * circos_available
  circos_angle <- circos_segments$angle_end[index] + circos_gap
}
circos_angle_at <- function(chromosome_index, position) {
  row <- circos_segments[chromosome_index + 1,]
  row$angle_start + (position - row$start) / row$size *
    (row$angle_end - row$angle_start)
}

circos_track_band <- 0.075
circos_track_gap <- 0.018
circos_track <- data.frame(
  track_index = c(0, 0, 0, 1, 1),
  point_index = c(0, 1, 2, 0, 1),
  source_row = c(0, 1, 2, 0, 1),
  chromosome_index = c(0, 0, 1, 0, 1),
  start = c(10, 50, 20, 0, 5),
  end = c(10, 50, 20, 30, 40),
  value = c(2, 9, 4, -0.6, 0.8)
)
circos_track$angle_start <- mapply(
  circos_angle_at, circos_track$chromosome_index, circos_track$start
)
circos_track$angle_end <- mapply(
  circos_angle_at, circos_track$chromosome_index, circos_track$end
)
circos_track$radial_inner <- c(rep(0.86 - circos_track_band, 3),
  rep(0.86 - (circos_track_band + circos_track_gap) - circos_track_band, 2))
circos_line_normalized <- (circos_track$value[1:3] - 2) / (9 - 2)
circos_track$radial_outer <- c(
  circos_track$radial_inner[1:3] + circos_line_normalized * circos_track_band,
  rep(0.86 - (circos_track_band + circos_track_gap), 2)
)

circos_links <- data.frame(
  link_index = 0,
  source_row = 0,
  source_chromosome_index = 0,
  source_start = 10,
  source_end = 20,
  target_chromosome_index = 1,
  target_start = 30,
  target_end = 35,
  weight = 16
)
circos_links$source_angle_start <- circos_angle_at(0, circos_links$source_start)
circos_links$source_angle_end <- circos_angle_at(0, circos_links$source_end)
circos_links$target_angle_start <- circos_angle_at(1, circos_links$target_start)
circos_links$target_angle_end <- circos_angle_at(1, circos_links$target_end)
circos_links$stroke_width <- 0.75 + round(sqrt(circos_links$weight /
  max(circos_links$weight)) * 3 * 4) / 4

result <- paste0(
  "{",
  '"edge_right":{',
  '"left":', json_array(head(edge_right$breaks, -1)), ",",
  '"right":', json_array(tail(edge_right$breaks, -1)), ",",
  '"counts":', json_array(edge_right$counts), ",",
  '"density":', json_array(edge_right$density),
  "},",
  '"edge_left":{',
  '"left":', json_array(head(edge_left$breaks, -1)), ",",
  '"right":', json_array(tail(edge_left$breaks, -1)), ",",
  '"counts":', json_array(edge_left$counts), ",",
  '"density":', json_array(edge_left$density),
  "},",
  '"edge_right_open":{',
  '"left":', json_array(head(edge_right_open$breaks, -1)), ",",
  '"right":', json_array(tail(edge_right_open$breaks, -1)), ",",
  '"counts":', json_array(edge_right_open$counts), ",",
  '"density":', json_array(edge_right_open$density),
  "},",
  '"edge_left_open":{',
  '"left":', json_array(head(edge_left_open$breaks, -1)), ",",
  '"right":', json_array(tail(edge_left_open$breaks, -1)), ",",
  '"counts":', json_array(edge_left_open$counts), ",",
  '"density":', json_array(edge_left_open$density),
  "},",
  '"airquality":{',
  '"left":', json_array(head(air_histogram$breaks, -1)), ",",
  '"right":', json_array(tail(air_histogram$breaks, -1)), ",",
  '"counts":', json_array(air_histogram$counts), ",",
  '"density":', json_array(air_histogram$density),
  "},",
  '"box_type7":{',
  '"summary":', json_array(box_type7$summary), ",",
  '"outliers":', json_array(box_type7$outliers),
  "},",
  '"box_tukey":{',
  '"summary":', json_array(box_tukey$summary), ",",
  '"outliers":', json_array(box_tukey$outliers),
  "},",
  '"ecdf":{',
  '"x":', json_array(ecdf_x), ",",
  '"counts":', json_array(ecdf_counts), ",",
  '"cumulative":', json_array(ecdf_cumulative), ",",
  '"fraction":', json_array(ecdf_fraction),
  "},",
  '"normal_qq":{',
  '"theoretical":', json_array(qq$x), ",",
  '"sample":', json_array(qq$y), ",",
  '"line":', json_array(c(qq_intercept, qq_slope)),
  "},",
  '"violin":{',
  '"bandwidth":', format(violin_bandwidth, digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"x":', json_array(violin_x), ",",
  '"density":', json_array(violin_density), ",",
  '"scaled":', json_array(violin_scaled),
  "},",
  '"air_box_type7":{',
  '"summary":', json_array(air_box_type7$summary), ",",
  '"outliers":', json_array(air_box_type7$outliers),
  "},",
  '"air_box_tukey":{',
  '"summary":', json_array(air_box_tukey$summary), ",",
  '"outliers":', json_array(air_box_tukey$outliers),
  "},",
  '"air_ecdf":{',
  '"x":', json_array(air_ecdf_x), ",",
  '"counts":', json_array(air_ecdf_counts), ",",
  '"cumulative":', json_array(air_ecdf_cumulative), ",",
  '"fraction":', json_array(air_ecdf_fraction),
  "},",
  '"air_normal_qq":{',
  '"theoretical":', json_array(air_qq$x), ",",
  '"sample":', json_array(air_qq$y), ",",
  '"line":', json_array(c(air_qq_intercept, air_qq_slope)),
  "},",
  '"air_violin":{',
  '"bandwidth":', format(air_violin_bandwidth, digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"x":', json_array(air_violin_x), ",",
  '"density":', json_array(air_violin_density), ",",
  '"scaled":', json_array(air_violin_scaled),
  "},",
  '"linear_fit_air":{',
  '"slope":', format(coef(air_fit)[2], digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"intercept":', format(coef(air_fit)[1], digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"residual_mse":', format(deviance(air_fit) / df.residual(air_fit), digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"x":', json_array(air_fit_at), ",",
  '"fitted":', json_array(air_fit_confidence[, "fit"]), ",",
  '"confidence_lower":', json_array(air_fit_confidence[, "lwr"]), ",",
  '"confidence_upper":', json_array(air_fit_confidence[, "upr"]), ",",
  '"prediction_lower":', json_array(air_fit_prediction[, "lwr"]), ",",
  '"prediction_upper":', json_array(air_fit_prediction[, "upr"]),
  "},",
  '"clinical_survival":{',
  '"time":', json_array(clinical_survival$time), ",",
  '"n_risk":', json_array(clinical_survival$n.risk), ",",
  '"n_event":', json_array(clinical_survival$n.event), ",",
  '"n_censor":', json_array(clinical_survival$n.censor), ",",
  '"survival":', json_array(clinical_survival$surv), ",",
  '"std_error":', json_array(clinical_survival$std.err),
  "},",
  '"clinical_roc":{',
  '"auc":', format(clinical_auc, digits = 17, scientific = FALSE, trim = TRUE),
  "},",
  '"clinical_forest":{',
  '"estimate":', json_array(clinical_forest$estimate), ",",
  '"lower":', json_array(clinical_forest$lower), ",",
  '"upper":', json_array(clinical_forest$upper), ",",
  '"weight":', json_array(clinical_forest$weight),
  "},",
  '"genomic_manhattan":{',
  '"chromosome_index":', json_array(association_chromosome_index), ",",
  '"offset":', json_array(association_offsets), ",",
  '"genome_position":', json_array(association_genome_position), ",",
  '"neg_log10_p":', json_array(-log10(association$pvalue)), ",",
  '"significant":', json_array(as.numeric(association$pvalue <= 5e-8)),
  "},",
  '"genetic_qq":{',
  '"rank":', json_array(genetic_qq_rank), ",",
  '"p_value":', json_array(genetic_qq_p), ",",
  '"expected_p":', json_array(genetic_qq_expected_p), ",",
  '"expected_neg_log10_p":', json_array(-log10(genetic_qq_expected_p)), ",",
  '"observed_neg_log10_p":', json_array(-log10(genetic_qq_p)), ",",
  '"envelope_lower":', json_array(-log10(genetic_qq_upper_p)), ",",
  '"envelope_upper":', json_array(-log10(genetic_qq_lower_p)), ",",
  '"lambda_gc":', format(genetic_qq_lambda, digits = 17, scientific = FALSE, trim = TRUE),
  "},",
  '"genomic_rainfall":{',
  '"source_row":', json_array(rainfall_rows$source_row), ",",
  '"position":', json_array(rainfall_rows$pos), ",",
  '"previous_position":', json_array(rainfall_previous), ",",
  '"distance":', json_array(rainfall_distance), ",",
  '"plotted_distance":', json_array(rainfall_plotted), ",",
  '"log10_distance":', json_array(log10(rainfall_plotted)), ",",
  '"duplicate_position":', json_array(as.numeric(rainfall_distance == 0)),
  "},",
  '"genomic_ideogram":{',
  '"chromosome_length":', json_array(ideogram_lengths), ",",
  '"source_row":', json_array(ideogram_sorted$source_row), ",",
  '"chromosome_index":', json_array(ideogram_sorted_index), ",",
  '"start":', json_array(ideogram_sorted$start), ",",
  '"end":', json_array(ideogram_sorted$end), ",",
  '"length":', json_array(ideogram_sorted$end - ideogram_sorted$start),
  "},",
  '"genomic_cnv":{',
  '"chromosome_offset":', json_array(cnv_offsets), ",",
  '"source_row":', json_array(cnv_sorted$source_row), ",",
  '"chromosome_index":', json_array(cnv_sorted_index), ",",
  '"start":', json_array(cnv_sorted$start), ",",
  '"end":', json_array(cnv_sorted$end), ",",
  '"genome_start":', json_array(cnv_genome_start), ",",
  '"genome_end":', json_array(cnv_genome_end), ",",
  '"genome_midpoint":', json_array((cnv_genome_start + cnv_genome_end) / 2), ",",
  '"log2ratio":', json_array(cnv_sorted$log2ratio), ",",
  '"state":', json_array(cnv_state),
  "},",
  '"genomic_coverage":{',
  '"source_row":', json_array(coverage_clipped$source_row), ",",
  '"original_start":', json_array(coverage_clipped$start), ",",
  '"original_end":', json_array(coverage_clipped$end), ",",
  '"start":', json_array(coverage_clipped$clipped_start), ",",
  '"end":', json_array(coverage_clipped$clipped_end), ",",
  '"position":', json_array((coverage_clipped$clipped_start + coverage_clipped$clipped_end) / 2), ",",
  '"value":', json_array(coverage_clipped$value), ",",
  '"clipped":', json_array(as.numeric(
    coverage_clipped$start != coverage_clipped$clipped_start |
      coverage_clipped$end != coverage_clipped$clipped_end
  )),
  "},",
  '"regional_genome":{',
  '"source_row":', json_array(genome_sorted$source_row), ",",
  '"original_start":', json_array(genome_sorted$start), ",",
  '"original_end":', json_array(genome_sorted$end), ",",
  '"start":', json_array(genome_sorted$clipped_start), ",",
  '"end":', json_array(genome_sorted$clipped_end), ",",
  '"length":', json_array(genome_sorted$clipped_end - genome_sorted$clipped_start), ",",
  '"lane":', json_array(genome_lanes), ",",
  '"clipped":', json_array(as.numeric(
    genome_sorted$start != genome_sorted$clipped_start |
      genome_sorted$end != genome_sorted$clipped_end
  )),
  "},",
  '"regional_lollipop":{',
  '"source_row":', json_array(lollipop_sorted$source_row), ",",
  '"position":', json_array(lollipop_sorted$position), ",",
  '"height":', json_array(lollipop_sorted$height), ",",
  '"domain":', json_array(lollipop_domain), ",",
  '"y_max":', format(max(1, lollipop_sorted$height), digits = 17, scientific = FALSE, trim = TRUE),
  "},",
  '"regional_sashimi":{',
  '"coverage_source_row":', json_array(sashimi_coverage$source_row), ",",
  '"coverage_position":', json_array(sashimi_coverage$position), ",",
  '"coverage_depth":', json_array(sashimi_coverage$depth), ",",
  '"junction_source_row":', json_array(sashimi_junctions$source_row), ",",
  '"junction_start":', json_array(sashimi_junctions$start), ",",
  '"junction_end":', json_array(sashimi_junctions$end), ",",
  '"junction_span":', json_array(sashimi_junctions$end - sashimi_junctions$start), ",",
  '"junction_count":', json_array(sashimi_junctions$count), ",",
  '"junction_lane":', json_array(sashimi_lanes), ",",
  '"arc_fraction":', json_array(sashimi_arc_fraction), ",",
  '"stroke_width":', json_array(sashimi_stroke_width), ",",
  '"max_count":', format(sashimi_max_count, digits = 17, scientific = FALSE, trim = TRUE), ",",
  '"max_depth":', format(max(1, sashimi_coverage$depth), digits = 17, scientific = FALSE, trim = TRUE),
  "},",
  '"circular_circos":{',
  '"segment_chromosome_index":', json_array(circos_segments$chromosome_index), ",",
  '"segment_source_row":', json_array(circos_segments$source_row), ",",
  '"segment_start":', json_array(circos_segments$start), ",",
  '"segment_end":', json_array(circos_segments$end), ",",
  '"segment_size":', json_array(circos_segments$size), ",",
  '"segment_angle_start":', json_array(circos_segments$angle_start), ",",
  '"segment_angle_end":', json_array(circos_segments$angle_end), ",",
  '"track_index":', json_array(circos_track$track_index), ",",
  '"track_point_index":', json_array(circos_track$point_index), ",",
  '"track_source_row":', json_array(circos_track$source_row), ",",
  '"track_chromosome_index":', json_array(circos_track$chromosome_index), ",",
  '"track_start":', json_array(circos_track$start), ",",
  '"track_end":', json_array(circos_track$end), ",",
  '"track_value":', json_array(circos_track$value), ",",
  '"track_angle_start":', json_array(circos_track$angle_start), ",",
  '"track_angle_end":', json_array(circos_track$angle_end), ",",
  '"track_radial_inner":', json_array(circos_track$radial_inner), ",",
  '"track_radial_outer":', json_array(circos_track$radial_outer), ",",
  '"link_index":', json_array(circos_links$link_index), ",",
  '"link_source_row":', json_array(circos_links$source_row), ",",
  '"link_source_chromosome_index":', json_array(circos_links$source_chromosome_index), ",",
  '"link_source_start":', json_array(circos_links$source_start), ",",
  '"link_source_end":', json_array(circos_links$source_end), ",",
  '"link_target_chromosome_index":', json_array(circos_links$target_chromosome_index), ",",
  '"link_target_start":', json_array(circos_links$target_start), ",",
  '"link_target_end":', json_array(circos_links$target_end), ",",
  '"link_source_angle_start":', json_array(circos_links$source_angle_start), ",",
  '"link_source_angle_end":', json_array(circos_links$source_angle_end), ",",
  '"link_target_angle_start":', json_array(circos_links$target_angle_start), ",",
  '"link_target_angle_end":', json_array(circos_links$target_angle_end), ",",
  '"link_weight":', json_array(circos_links$weight), ",",
  '"link_stroke_width":', json_array(circos_links$stroke_width),
  "}",
  "}"
)

writeLines(
  result,
  "packages/statistics/validation/results/r-plot-reference.json",
  useBytes = TRUE
)
