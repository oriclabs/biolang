# Independent base-R oracle for renderer-independent histogram geometry.
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
  "}",
  "}"
)

writeLines(
  result,
  "packages/statistics/validation/results/r-plot-reference.json",
  useBytes = TRUE
)
