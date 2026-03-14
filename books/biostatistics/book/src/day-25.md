# Day 25: Statistical Visualization — Plots That Tell the Truth

<div class="day-meta">
<span class="badge">Day 25 of 30</span>
<span class="badge">Prerequisites: All previous days</span>
<span class="badge">~60 min reading</span>
<span class="badge">Data Visualization</span>
</div>

## The Problem

You have spent weeks analyzing a clinical dataset. The results are solid: a survival benefit, a clear dose-response relationship, differentially expressed genes with strong effect sizes. You write up the manuscript, generate figures, and submit.

Three weeks later, the reviewer's comments arrive. "Figure 2: bar charts with error bars hide the data distribution. Replace with violin plots or beeswarm plots showing individual data points. Figure 4: the y-axis does not start at zero, exaggerating the effect. Figure 6: red-green color scheme is inaccessible to the 8% of males with color vision deficiency. Major revision."

The statistics were correct. The visualization was not. And in modern publishing, visualization is not decoration — it is evidence. A misleading plot can sink an otherwise excellent paper. A well-designed figure can convey complex results in seconds.

Today is your visualization reference guide. We will cover every major plot type you have encountered in this book, when to use each, how to read them, common mistakes, and how to produce publication-ready versions in BioLang.

## The "Which Plot?" Decision Guide

Choosing the right plot starts with two questions: What type of data do you have? What relationship are you showing?

| Data situation | Recommended plot(s) |
|---|---|
| Distribution of one continuous variable | `histogram`, `density_plot` |
| Compare distributions across groups | `boxplot`, `violin_plot` |
| Two continuous variables | `scatter` |
| Trend over time or ordered variable | `line_plot` |
| Counts or proportions by category | `bar_chart` |
| Matrix of values (e.g., expression) | `heatmap` |
| Differential expression (fold change + significance) | `volcano` |
| Genome-wide association results | `manhattan_plot` |
| Observed vs expected p-values | `qq_plot` |
| Meta-analysis effect sizes | `forest_plot` |
| Method agreement | `bland_altman` |
| Publication bias assessment | `funnel_plot` |
| Survival curves | `kaplan_meier_plot` |
| Classifier performance | `roc_curve` |
| PCA scores + loadings | `pca_biplot` |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="420" viewBox="0 0 680 420" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">"Which Plot?" Decision Guide</text>
  <!-- Headers -->
  <rect x="20" y="45" width="220" height="30" rx="4" fill="#2563eb"/>
  <text x="130" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="white">Data Situation</text>
  <rect x="245" y="45" width="200" height="30" rx="4" fill="#2563eb"/>
  <text x="345" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="white">Recommended Plot</text>
  <rect x="450" y="45" width="215" height="30" rx="4" fill="#2563eb"/>
  <text x="558" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="white">Visual Example</text>
  <!-- Row 1: One continuous variable -->
  <rect x="20" y="80" width="220" height="38" rx="3" fill="#f0f9ff"/>
  <text x="130" y="103" text-anchor="middle" font-size="11" fill="#1e293b">One continuous variable</text>
  <rect x="245" y="80" width="200" height="38" rx="3" fill="#f0f9ff"/>
  <text x="345" y="103" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Histogram / Density</text>
  <g transform="translate(480, 85)">
    <rect x="0" y="18" width="10" height="10" fill="#3b82f6" opacity="0.6"/><rect x="12" y="12" width="10" height="16" fill="#3b82f6" opacity="0.6"/>
    <rect x="24" y="5" width="10" height="23" fill="#3b82f6" opacity="0.7"/><rect x="36" y="0" width="10" height="28" fill="#3b82f6" opacity="0.8"/>
    <rect x="48" y="4" width="10" height="24" fill="#3b82f6" opacity="0.7"/><rect x="60" y="10" width="10" height="18" fill="#3b82f6" opacity="0.6"/>
    <rect x="72" y="16" width="10" height="12" fill="#3b82f6" opacity="0.5"/><rect x="84" y="22" width="10" height="6" fill="#3b82f6" opacity="0.4"/>
  </g>
  <!-- Row 2: Compare groups -->
  <rect x="20" y="123" width="220" height="38" rx="3" fill="white"/>
  <text x="130" y="146" text-anchor="middle" font-size="11" fill="#1e293b">Compare groups (continuous)</text>
  <rect x="245" y="123" width="200" height="38" rx="3" fill="white"/>
  <text x="345" y="146" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Boxplot / Violin</text>
  <g transform="translate(495, 128)">
    <rect x="0" y="8" width="18" height="16" fill="none" stroke="#3b82f6" stroke-width="1.5"/>
    <line x1="0" y1="16" x2="18" y2="16" stroke="#3b82f6" stroke-width="2"/>
    <line x1="9" y1="0" x2="9" y2="8" stroke="#3b82f6" stroke-width="1"/>
    <line x1="9" y1="24" x2="9" y2="30" stroke="#3b82f6" stroke-width="1"/>
    <rect x="28" y="5" width="18" height="20" fill="none" stroke="#dc2626" stroke-width="1.5"/>
    <line x1="28" y1="15" x2="46" y2="15" stroke="#dc2626" stroke-width="2"/>
    <line x1="37" y1="0" x2="37" y2="5" stroke="#dc2626" stroke-width="1"/>
    <line x1="37" y1="25" x2="37" y2="30" stroke="#dc2626" stroke-width="1"/>
    <rect x="56" y="10" width="18" height="14" fill="none" stroke="#16a34a" stroke-width="1.5"/>
    <line x1="56" y1="17" x2="74" y2="17" stroke="#16a34a" stroke-width="2"/>
    <line x1="65" y1="2" x2="65" y2="10" stroke="#16a34a" stroke-width="1"/>
    <line x1="65" y1="24" x2="65" y2="30" stroke="#16a34a" stroke-width="1"/>
  </g>
  <!-- Row 3: Two continuous -->
  <rect x="20" y="166" width="220" height="38" rx="3" fill="#f0f9ff"/>
  <text x="130" y="189" text-anchor="middle" font-size="11" fill="#1e293b">Two continuous variables</text>
  <rect x="245" y="166" width="200" height="38" rx="3" fill="#f0f9ff"/>
  <text x="345" y="189" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Scatter Plot</text>
  <g transform="translate(498, 172)">
    <circle cx="5" cy="22" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="15" cy="18" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="25" cy="15" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="35" cy="10" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="45" cy="8" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="55" cy="5" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="20" cy="20" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="40" cy="12" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="50" cy="6" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="30" cy="18" r="2.5" fill="#3b82f6" opacity="0.7"/>
    <line x1="0" y1="25" x2="62" y2="0" stroke="#dc2626" stroke-width="1" stroke-dasharray="3,2"/>
  </g>
  <!-- Row 4: Trend / time -->
  <rect x="20" y="209" width="220" height="38" rx="3" fill="white"/>
  <text x="130" y="232" text-anchor="middle" font-size="11" fill="#1e293b">Trend over time / ordered</text>
  <rect x="245" y="209" width="200" height="38" rx="3" fill="white"/>
  <text x="345" y="232" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Line Plot</text>
  <g transform="translate(498, 215)">
    <polyline points="0,22 15,18 30,12 45,8 60,3" fill="none" stroke="#3b82f6" stroke-width="2"/>
    <polyline points="0,20 15,22 30,24 45,26 60,28" fill="none" stroke="#dc2626" stroke-width="2"/>
  </g>
  <!-- Row 5: Categories -->
  <rect x="20" y="252" width="220" height="38" rx="3" fill="#f0f9ff"/>
  <text x="130" y="275" text-anchor="middle" font-size="11" fill="#1e293b">Counts by category</text>
  <rect x="245" y="252" width="200" height="38" rx="3" fill="#f0f9ff"/>
  <text x="345" y="275" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Bar Chart</text>
  <g transform="translate(498, 255)">
    <rect x="0" y="8" width="14" height="22" rx="1" fill="#3b82f6" opacity="0.7"/>
    <rect x="18" y="0" width="14" height="30" rx="1" fill="#3b82f6" opacity="0.7"/>
    <rect x="36" y="12" width="14" height="18" rx="1" fill="#3b82f6" opacity="0.7"/>
    <rect x="54" y="5" width="14" height="25" rx="1" fill="#3b82f6" opacity="0.7"/>
  </g>
  <!-- Row 6: Matrix -->
  <rect x="20" y="295" width="220" height="38" rx="3" fill="white"/>
  <text x="130" y="318" text-anchor="middle" font-size="11" fill="#1e293b">Expression matrix</text>
  <rect x="245" y="295" width="200" height="38" rx="3" fill="white"/>
  <text x="345" y="318" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Heatmap</text>
  <g transform="translate(498, 298)">
    <rect x="0" y="0" width="10" height="10" fill="#dc2626" opacity="0.8"/>
    <rect x="12" y="0" width="10" height="10" fill="#ef4444" opacity="0.5"/>
    <rect x="24" y="0" width="10" height="10" fill="#93c5fd"/>
    <rect x="36" y="0" width="10" height="10" fill="#2563eb"/>
    <rect x="0" y="12" width="10" height="10" fill="#93c5fd"/>
    <rect x="12" y="12" width="10" height="10" fill="#2563eb"/>
    <rect x="24" y="12" width="10" height="10" fill="#dc2626" opacity="0.7"/>
    <rect x="36" y="12" width="10" height="10" fill="#ef4444" opacity="0.4"/>
    <rect x="0" y="24" width="10" height="10" fill="#2563eb" opacity="0.6"/>
    <rect x="12" y="24" width="10" height="10" fill="#dc2626" opacity="0.9"/>
    <rect x="24" y="24" width="10" height="10" fill="#ef4444" opacity="0.3"/>
    <rect x="36" y="24" width="10" height="10" fill="#93c5fd"/>
  </g>
  <!-- Row 7: DE / Volcano -->
  <rect x="20" y="338" width="220" height="38" rx="3" fill="#f0f9ff"/>
  <text x="130" y="361" text-anchor="middle" font-size="11" fill="#1e293b">Fold change + significance</text>
  <rect x="245" y="338" width="200" height="38" rx="3" fill="#f0f9ff"/>
  <text x="345" y="361" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Volcano Plot</text>
  <g transform="translate(498, 340)">
    <circle cx="5" cy="25" r="2" fill="#9ca3af" opacity="0.5"/>
    <circle cx="15" cy="22" r="2" fill="#9ca3af" opacity="0.5"/>
    <circle cx="25" cy="20" r="2" fill="#9ca3af" opacity="0.5"/>
    <circle cx="35" cy="18" r="2" fill="#9ca3af" opacity="0.5"/>
    <circle cx="30" cy="24" r="2" fill="#9ca3af" opacity="0.5"/>
    <circle cx="8" cy="8" r="2.5" fill="#3b82f6"/>
    <circle cx="12" cy="5" r="2.5" fill="#3b82f6"/>
    <circle cx="52" cy="6" r="2.5" fill="#dc2626"/>
    <circle cx="58" cy="3" r="2.5" fill="#dc2626"/>
    <circle cx="55" cy="10" r="2.5" fill="#dc2626"/>
    <line x1="30" y1="0" x2="30" y2="30" stroke="#9ca3af" stroke-width="0.5" stroke-dasharray="2,2"/>
  </g>
  <!-- Footer -->
  <rect x="20" y="386" width="645" height="25" rx="4" fill="#f1f5f9"/>
  <text x="340" y="403" text-anchor="middle" font-size="11" fill="#6b7280">Also: Manhattan plot (GWAS), QQ plot (p-value QC), Forest plot (meta-analysis), ROC curve (classifier), Kaplan-Meier (survival)</text>
</svg>
</div>

## Histogram

**When**: Visualize the shape of a single continuous distribution.

**How to read**: Each bar represents a bin; the height shows how many observations fall in that range. Look for: center (peak), spread (width), shape (symmetric? skewed?), modes (one peak or multiple?), outliers (isolated bars far from the center).

**Mistakes**: Too few bins (hides structure), too many bins (shows noise), not specifying bin count (default may be misleading).

```bio
set_seed(42)
# Quality score distribution
let scores = concat(rnorm(5000, 35, 8), rnorm(1000, 20, 3))

histogram(scores, {bins: 50,
  title: "Sequencing Quality Score Distribution",
  xlabel: "Phred Quality Score",
  ylabel: "Count"})
```

**Publication tip**: Always state the bin width or number of bins in the figure legend. Use 30-50 bins for most datasets.

## Density Plot

**When**: Smooth estimate of the probability distribution. Better than histograms for comparing multiple groups on the same axes.

**How to read**: The curve shows estimated probability density. The area under the curve equals 1. Higher curves mean more observations at that value.

```bio
set_seed(42)
# Compare two treatment groups
let group_a = rnorm(200, 50, 12)
let group_b = rnorm(200, 62, 15)

density(group_a, {title: "Expression Level Distribution by Group",
  xlabel: "Expression (TPM)", label: "Control"})
density(group_b, {label: "Treatment"})
```

**Publication tip**: Use semi-transparent fills when overlapping multiple densities. State the bandwidth if non-default.

## Box Plot

**When**: Compare distributions across groups. The standard for multi-group comparisons in biology.

**How to read**: The box spans Q1 to Q3 (the IQR, containing the middle 50%). The line inside is the median. Whiskers extend to the most extreme point within 1.5 x IQR. Points beyond whiskers are outliers.

**Mistakes**: Bar charts with error bars hide distribution shape — use box plots instead. Forgetting to show sample size.

```bio
set_seed(42)
let control = rnorm(30, 5.0, 1.2)
let low_dose = rnorm(30, 6.5, 1.5)
let high_dose = rnorm(30, 8.2, 1.8)

let bp_table = table({"Control": control, "Low Dose": low_dose, "High Dose": high_dose})
boxplot(bp_table, {title: "Tumor Response by Treatment Arm"})
```

**Publication tip**: Always overlay individual data points (jittered) on box plots when n < 50. Report n per group in the axis label or legend.

## Violin Plot

**When**: Like a box plot but shows the full distribution shape. Best for revealing multimodality that box plots miss.

**How to read**: The width of the "violin" at any value shows the density of observations there. A violin with two bulges indicates a bimodal distribution.

```bio
violin([control, low_dose, high_dose],
  {labels: ["Control", "Low Dose", "High Dose"],
  title: "Response Distribution — Full Shape",
  ylabel: "Response Score"})
```

**Publication tip**: Include a miniature box plot inside the violin for reference. Violin plots are increasingly preferred by reviewers over bar charts.

## Scatter Plot

**When**: Show the relationship between two continuous variables. The most versatile plot in statistics.

**How to read**: Each point is one observation. Look for: direction (positive/negative), form (linear/curved), strength (tight/scattered), outliers (isolated points), clusters.

```bio
set_seed(42)
let gene_expr = rnorm(100, 50, 15)
let drug_response = gene_expr |> map(|x| -0.8 * x + 90 + rnorm(1, 0, 8)[0])

scatter(gene_expr, drug_response,
  {xlabel: "Gene Expression (TPM)",
  ylabel: "Drug Response (%)",
  title: "Gene Expression vs Drug Response"})
```

**Publication tip**: Add a trend line with confidence band for linear relationships. Use transparency (alpha) when points overlap. Report the correlation coefficient and p-value in the figure legend or panel.

## Line Plot

**When**: Data has a natural order (time series, dose-response, ordered categories).

**How to read**: The line connects sequential observations. Look for trends, cycles, sudden changes.

**Mistakes**: Connecting unrelated categorical data with lines (implies ordering that does not exist).

```bio
let days = seq(0, 28, 7)
let tumor_vol_drug = [450, 380, 290, 210, 175]
let tumor_vol_ctrl = [450, 510, 580, 650, 720]

# Build a table with both series for plotting
let drug_rows = zip(days, tumor_vol_drug) |> map(|p| {day: p[0], volume: p[1], group: "Drug X"})
let ctrl_rows = zip(days, tumor_vol_ctrl) |> map(|p| {day: p[0], volume: p[1], group: "Control"})
let tbl = concat(drug_rows, ctrl_rows) |> to_table()

plot(tbl, {type: "line", x: "day", y: "volume", color: "group",
  xlabel: "Days Since Randomization",
  ylabel: "Tumor Volume (mm^3)",
  title: "Tumor Growth Over Time"})
```

## Bar Chart

**When**: Compare counts or proportions across categories. NOT for continuous distributions (use box/violin plots).

**How to read**: Bar height represents the value. Bars should start at zero.

**Mistakes**: Using bar charts for continuous data (hides distribution). Not starting at zero (exaggerates differences). Using 3D bars (distorts perception).

```bio
let categories = ["Complete Response", "Partial Response", "Stable Disease", "Progressive"]
let counts = [18, 35, 28, 19]

bar_chart(categories, counts,
  {title: "RECIST Response Categories",
  ylabel: "Number of Patients"})
```

> **Common pitfall:** Bar charts with error bars (dynamite plots) are the most criticized visualization in biostatistics. They show only the mean and one measure of spread, hiding the actual data distribution. A sample with a bimodal distribution and a sample with a normal distribution can produce identical bar charts. Always prefer box plots, violin plots, or strip charts for continuous data.

## Heatmap

**When**: Visualize a matrix of values (gene expression across samples, correlation matrices, p-value matrices).

**How to read**: Color intensity represents the value at each cell. Rows and columns are often clustered to group similar patterns.

```bio
# Expression heatmap of top 50 DE genes
heatmap(top_50_expression,
  {cluster_rows: true,
  cluster_cols: true,
  color_scale: "red_blue",
  title: "Top 50 Differentially Expressed Genes"})
```

**Publication tip**: State the color scale, clustering method, and distance metric in the legend. Use diverging color scales (red-blue) for data centered on zero; sequential scales (white-red) for non-negative data.

## Volcano Plot

**When**: Differential expression analysis — simultaneously display fold change (effect size) and statistical significance.

**How to read**: x-axis is log2 fold change, y-axis is -log10(p-value). Points in the upper corners are genes with large, significant changes. Upper-left: significantly downregulated. Upper-right: significantly upregulated. Bottom center: not significant.

```bio
volcano(de_results,
  {fc_threshold: 1.0,       # log2 FC cutoff
  p_threshold: 0.05,        # adjusted p-value cutoff
  title: "Tumor vs Normal — Differential Expression",
  xlabel: "log2 Fold Change",
  ylabel: "-log10(adjusted p-value)"})
```

**Publication tip**: Use adjusted p-values (FDR), not raw p-values. Label the most significant genes. Include dashed lines at your FC and p-value thresholds.

## Manhattan Plot

**When**: Genome-wide association results — display p-values for hundreds of thousands of SNPs across chromosomes.

**How to read**: x-axis is genomic position (chromosomes colored alternately), y-axis is -log10(p-value). Peaks above the genome-wide significance line (p < 5 x 10^-8) are associated loci.

```bio
manhattan(gwas_results,
  {significance_line: 5e-8,
  suggestive_line: 1e-5,
  title: "GWAS — Type 2 Diabetes"})
```

**Publication tip**: Use alternating colors for chromosomes. Draw both the genome-wide significance line (5 x 10^-8) and the suggestive line (1 x 10^-5). Label top hits with the nearest gene name.

## Q-Q Plot

**When**: Check whether observed p-values follow the expected uniform distribution under the null. Essential for GWAS quality control.

**How to read**: Points should follow the diagonal if there is no systematic inflation. Deviation at the upper end indicates true signal. Deviation along the entire line indicates systematic inflation (population structure, technical artifacts).

```bio
qq_plot(p_values,
  {title: "Q-Q Plot — Observed vs Expected p-values",
  ci: true})
```

**Publication tip**: Report the genomic inflation factor (lambda) in the plot or legend. Lambda close to 1.0 is good; lambda > 1.1 suggests confounding.

## Forest Plot

**When**: Meta-analysis or subgroup analysis — display effect estimates and confidence intervals from multiple studies or subgroups.

**How to read**: Each row is a study/subgroup. The square is the point estimate (size proportional to weight), horizontal line is the CI. The diamond at the bottom is the pooled estimate. If the CI crosses the null (vertical line at 0 or 1), the result is not statistically significant.

```bio
let meta_tbl = [
  {study: "Smith 2019", estimate: 0.72, ci_lower: 0.55, ci_upper: 0.93, weight: 25},
  {study: "Jones 2020", estimate: 0.85, ci_lower: 0.70, ci_upper: 1.03, weight: 30},
  {study: "Lee 2021", estimate: 0.68, ci_lower: 0.48, ci_upper: 0.96, weight: 15},
  {study: "Garcia 2022", estimate: 0.91, ci_lower: 0.75, ci_upper: 1.10, weight: 30},
  {study: "Pooled", estimate: 0.78, ci_lower: 0.68, ci_upper: 0.89, weight: 100}
] |> to_table()

forest_plot(meta_tbl,
  {null_value: 1.0,
  title: "Meta-Analysis — Hazard Ratios for Overall Survival",
  xlabel: "Hazard Ratio (95% CI)"})
```

## Bland-Altman Plot

**When**: Assess agreement between two measurement methods. NOT the same as correlation — two methods can be highly correlated but systematically biased.

**How to read**: x-axis is the mean of two measurements, y-axis is their difference. Points should scatter randomly around zero (no bias). The limits of agreement (mean difference +/- 1.96 SD) show the expected range of disagreement.

```bio
let method_a = [5.2, 6.1, 4.8, 7.3, 5.5, 6.8, 4.2, 5.9, 7.1, 6.3]
let method_b = [5.0, 6.3, 4.5, 7.0, 5.8, 6.5, 4.4, 5.7, 7.4, 6.1]

# Bland-Altman: plot difference vs mean
let means = zip(method_a, method_b) |> map(|p| (p[0] + p[1]) / 2)
let diffs = zip(method_a, method_b) |> map(|p| p[0] - p[1])
let mean_diff = mean(diffs)
let sd_diff = sd(diffs)

scatter(means, diffs,
  {title: "Bland-Altman — qPCR vs RNA-seq Expression",
  xlabel: "Mean of Two Methods",
  ylabel: "Difference (qPCR - RNA-seq)"})
print("Mean difference: " + str(round(mean_diff, 3)))
print("Limits of agreement: [" +
  str(round(mean_diff - 1.96 * sd_diff, 3)) + ", " +
  str(round(mean_diff + 1.96 * sd_diff, 3)) + "]")
```

## Funnel Plot

**When**: Assess publication bias in meta-analysis. Small studies with extreme results may indicate selective publishing.

**How to read**: x-axis is effect size, y-axis is study precision (1/SE or sample size). Large, precise studies cluster near the true effect (top). Small, imprecise studies scatter widely (bottom). Asymmetry suggests publication bias — if small negative studies are missing, the funnel is lopsided.

```bio
# Funnel plot: scatter of effect size vs precision
scatter(effect_sizes, standard_errors,
  {title: "Funnel Plot — Publication Bias Assessment",
  xlabel: "Effect Size (log OR)",
  ylabel: "Standard Error"})
```

## Kaplan-Meier Plot

**When**: Survival analysis — display probability of survival over time, comparing treatment arms.

**How to read**: The curve starts at 1.0 (all alive) and drops with each event. Censored observations (patients lost to follow-up) are marked with tick marks. The median survival is where the curve crosses 0.5.

```bio
# Build a Kaplan-Meier survival table and plot
let km_tbl = zip(time, event, group) |> map(|r| {
  time: r[0], event: r[1], group: r[2]
}) |> to_table()

plot(km_tbl, {type: "line", x: "time", y: "survival",
  color: "group",
  title: "Overall Survival — Drug X vs Standard of Care",
  xlabel: "Months",
  ylabel: "Survival Probability"})
```

**Publication tip**: Include a risk table below the plot showing the number at risk at regular intervals. Report the log-rank p-value and hazard ratio with CI.

## ROC Curve

**When**: Evaluate a binary classifier — plot sensitivity (true positive rate) against 1-specificity (false positive rate) at all thresholds.

**How to read**: The curve bows toward the upper-left corner for good classifiers. The AUC (area under the curve) summarizes performance: 0.5 = random guessing, 1.0 = perfect classification. AUC > 0.8 is generally considered good.

```bio
roc_curve(roc_tbl,
  {title: "ROC — Gene Expression Classifier for Response",
  auc_label: true})
```

## PCA Biplot

**When**: Display PCA scores (samples) and loadings (variables) simultaneously.

**How to read**: Points are samples — clusters indicate groups. Arrows are variable loadings — direction and length show each variable's contribution to the PCs.

```bio
let result = pca(expression_matrix)
pca_plot(result,
  {title: "PCA Biplot — Gene Expression",
  xlabel: "PC1 (" + str(round(result.variance_explained[0] * 100, 1)) + "%)",
  ylabel: "PC2 (" + str(round(result.variance_explained[1] * 100, 1)) + "%)"})
```

## The Grammar of Honest Visualization

### Rule 1: Start axes at zero (usually)

Truncating the y-axis can make a 2% difference look like a 200% difference. Always start bar charts at zero. For scatter plots and line plots, starting at zero is less critical but should be considered.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Truncated Y-Axis: Misleading vs Honest</text>
  <!-- BAD chart: truncated axis -->
  <rect x="20" y="48" width="300" height="235" rx="6" fill="white" stroke="#dc2626" stroke-width="1.5"/>
  <text x="170" y="68" text-anchor="middle" font-size="13" font-weight="bold" fill="#dc2626">Misleading</text>
  <text x="170" y="84" text-anchor="middle" font-size="10" fill="#dc2626">(Y-axis starts at 95)</text>
  <!-- Y-axis -->
  <line x1="70" y1="100" x2="70" y2="255" stroke="#9ca3af" stroke-width="1"/>
  <text x="62" y="255" text-anchor="end" font-size="10" fill="#9ca3af">95</text>
  <text x="62" y="216" text-anchor="end" font-size="10" fill="#9ca3af">97</text>
  <text x="62" y="177" text-anchor="end" font-size="10" fill="#9ca3af">99</text>
  <text x="62" y="138" text-anchor="end" font-size="10" fill="#9ca3af">101</text>
  <text x="62" y="100" text-anchor="end" font-size="10" fill="#9ca3af">103</text>
  <!-- X-axis -->
  <line x1="70" y1="255" x2="290" y2="255" stroke="#9ca3af" stroke-width="1"/>
  <!-- Bars (values: 100, 102, 101, 103 -- but axis starts at 95, making differences look huge) -->
  <rect x="90" y="157" width="40" height="98" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="145" y="118" width="40" height="137" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="200" y="138" width="40" height="117" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="255" y="98" width="40" height="157" rx="2" fill="#3b82f6" opacity="0.8"/>
  <text x="110" y="270" text-anchor="middle" font-size="10" fill="#6b7280">A</text>
  <text x="165" y="270" text-anchor="middle" font-size="10" fill="#6b7280">B</text>
  <text x="220" y="270" text-anchor="middle" font-size="10" fill="#6b7280">C</text>
  <text x="275" y="270" text-anchor="middle" font-size="10" fill="#6b7280">D</text>
  <!-- Zigzag break indicator -->
  <polyline points="65,250 67,244 73,248 70,242" fill="none" stroke="#dc2626" stroke-width="1.5"/>

  <!-- GOOD chart: axis from zero -->
  <rect x="360" y="48" width="300" height="235" rx="6" fill="white" stroke="#16a34a" stroke-width="1.5"/>
  <text x="510" y="68" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">Honest</text>
  <text x="510" y="84" text-anchor="middle" font-size="10" fill="#16a34a">(Y-axis starts at 0)</text>
  <!-- Y-axis -->
  <line x1="410" y1="100" x2="410" y2="255" stroke="#9ca3af" stroke-width="1"/>
  <text x="402" y="255" text-anchor="end" font-size="10" fill="#9ca3af">0</text>
  <text x="402" y="216" text-anchor="end" font-size="10" fill="#9ca3af">25</text>
  <text x="402" y="177" text-anchor="end" font-size="10" fill="#9ca3af">50</text>
  <text x="402" y="138" text-anchor="end" font-size="10" fill="#9ca3af">75</text>
  <text x="402" y="100" text-anchor="end" font-size="10" fill="#9ca3af">100</text>
  <!-- X-axis -->
  <line x1="410" y1="255" x2="630" y2="255" stroke="#9ca3af" stroke-width="1"/>
  <!-- Bars (same values: 100, 102, 101, 103 -- but now from 0, differences are tiny) -->
  <rect x="430" y="100" width="40" height="155" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="485" y="97" width="40" height="158" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="540" y="98" width="40" height="157" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="595" y="95" width="40" height="160" rx="2" fill="#3b82f6" opacity="0.8"/>
  <text x="450" y="270" text-anchor="middle" font-size="10" fill="#6b7280">A</text>
  <text x="505" y="270" text-anchor="middle" font-size="10" fill="#6b7280">B</text>
  <text x="560" y="270" text-anchor="middle" font-size="10" fill="#6b7280">C</text>
  <text x="615" y="270" text-anchor="middle" font-size="10" fill="#6b7280">D</text>

  <!-- Bottom note -->
  <rect x="20" y="290" width="640" height="22" rx="4" fill="#f1f5f9"/>
  <text x="340" y="305" text-anchor="middle" font-size="11" fill="#6b7280">Same data (100, 102, 101, 103). The truncated axis makes a 3% difference look like a 60% difference.</text>
</svg>
</div>

### Rule 2: No 3D charts

3D bar charts, 3D pie charts, and 3D scatter plots distort perception. The human visual system poorly judges depth in 2D projections. Use 2D alternatives always.

### Rule 3: Use colorblind-safe palettes

Approximately 8% of males and 0.5% of females have red-green color vision deficiency. Never use red-green as the only distinguishing feature. Safe alternatives:

| Palette | Colors |
|---|---|
| Blue-orange | Good contrast, colorblind safe |
| Viridis | Perceptually uniform, prints well in grayscale |
| Blue-red diverging | Good for centered data (use with colorblind check) |
| Categorical (Set2) | Up to 8 distinguishable, colorblind safe |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="200" viewBox="0 0 680 200" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Colorblind-Safe Palette (8 Distinguishable Colors)</text>
  <text x="340" y="48" text-anchor="middle" font-size="11" fill="#6b7280">Wong (2011) palette -- safe for protanopia, deuteranopia, and tritanopia</text>
  <!-- Color swatches -->
  <g transform="translate(60, 70)">
    <!-- Black -->
    <rect x="0" y="0" width="60" height="50" rx="6" fill="#000000"/>
    <text x="30" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Black</text>
    <text x="30" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#000000</text>
    <!-- Orange -->
    <rect x="75" y="0" width="60" height="50" rx="6" fill="#E69F00"/>
    <text x="105" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Orange</text>
    <text x="105" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#E69F00</text>
    <!-- Sky Blue -->
    <rect x="150" y="0" width="60" height="50" rx="6" fill="#56B4E9"/>
    <text x="180" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Sky Blue</text>
    <text x="180" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#56B4E9</text>
    <!-- Bluish Green -->
    <rect x="225" y="0" width="60" height="50" rx="6" fill="#009E73"/>
    <text x="255" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Green</text>
    <text x="255" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#009E73</text>
    <!-- Yellow -->
    <rect x="300" y="0" width="60" height="50" rx="6" fill="#F0E442" stroke="#e5e7eb"/>
    <text x="330" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Yellow</text>
    <text x="330" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#F0E442</text>
    <!-- Blue -->
    <rect x="375" y="0" width="60" height="50" rx="6" fill="#0072B2"/>
    <text x="405" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Blue</text>
    <text x="405" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#0072B2</text>
    <!-- Vermillion -->
    <rect x="450" y="0" width="60" height="50" rx="6" fill="#D55E00"/>
    <text x="480" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Vermillion</text>
    <text x="480" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#D55E00</text>
    <!-- Reddish Purple -->
    <rect x="525" y="0" width="60" height="50" rx="6" fill="#CC79A7"/>
    <text x="555" y="70" text-anchor="middle" font-size="10" fill="#1e293b">Rose</text>
    <text x="555" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">#CC79A7</text>
  </g>
  <!-- Bottom note -->
  <rect x="20" y="165" width="640" height="25" rx="4" fill="#f1f5f9"/>
  <text x="340" y="182" text-anchor="middle" font-size="11" fill="#6b7280">Avoid red-green as the only distinction. ~8% of males have red-green color vision deficiency. Use shape + color together.</text>
</svg>
</div>

### Rule 4: Show the data

Whenever possible, show individual data points. Summary statistics (means, medians) are important but insufficient. A bimodal distribution and a unimodal distribution can have identical means and standard deviations.

### Rule 5: Label everything

Every figure needs: title, axis labels with units, legend if multiple groups, sample sizes, and any statistical annotations.

## Visualization Sins That Lie

### The truncated axis

A bar chart showing revenue of $100M vs $102M with the y-axis starting at $99M makes a 2% difference look enormous. In biology, this commonly appears in gene expression plots where the y-axis starts at a non-zero value.

### The dual y-axis

Two different scales on the same plot can make unrelated trends appear correlated. By choosing the scales appropriately, you can make any two lines appear to track each other.

### The cherry-picked time window

Showing survival curves from month 6 to month 24 (where the drug looks good) but omitting month 0 to 6 (where there is no difference) or month 24 to 48 (where the effect fades) is misleading.

### The pie chart

Pie charts are universally criticized by statisticians. Humans are poor at judging angles and areas. A bar chart conveys the same information more accurately.

> **Key insight:** The goal of visualization is not to make your results look impressive — it is to accurately represent the data so that the reader can draw correct conclusions. A figure that misleads, even unintentionally, undermines the entire paper.

## Exercises

1. **Box vs violin vs bar.** Generate three datasets: (a) normal, (b) bimodal, (c) heavily skewed. For each, create a bar chart with error bars, a box plot with points, and a violin plot. Which plot type reveals the distribution shape most honestly?

```bio
set_seed(42)
let normal = rnorm(100, 50, 10)
let bimodal = concat(rnorm(50, 30, 5), rnorm(50, 70, 5))
let skewed = rnorm(100, 0, 1) |> map(|x| abs(x) * 10)
# Your code: create all three plot types for each dataset
```

2. **Publication figure.** Take the clinical trial data from Day 17 (survival analysis) and create a publication-ready Kaplan-Meier plot with: risk table, log-rank p-value, hazard ratio annotation, median survival lines, and proper axis labels. Save as SVG.

```bio
# Your code: kaplan_meier_plot with all publication elements
```

3. **Volcano and heatmap.** From the DE analysis on Day 12, create (a) a volcano plot with the top 10 genes labeled and threshold lines, and (b) a heatmap of the top 50 DE genes clustered by sample and gene.

```bio
# Your code: volcano + heatmap, publication quality
```

4. **Color blindness check.** Create a scatter plot with 4 groups using (a) a red-green palette and (b) a colorblind-safe palette. Describe why the first is problematic.

```bio
# Your code: two versions of the same scatter, different palettes
```

5. **Visualization critique.** The following figure specifications describe common visualization sins. For each, explain the problem and generate the correct version:
   - (a) Bar chart of gene expression levels with y-axis from 95 to 105
   - (b) 3D pie chart of mutation types
   - (c) Scatter plot with no axis labels

```bio
# Your code: create the corrected versions
```

## Key Takeaways

- Choose the plot type based on your data type and the relationship you want to show — the decision guide above covers the most common situations.
- Box plots and violin plots show distribution shape; bar charts with error bars do not — prefer the former for continuous data.
- Volcano plots combine fold change and significance for differential expression; Manhattan plots display genome-wide results by chromosomal position.
- Forest plots are the standard for meta-analysis and subgroup results; Bland-Altman plots assess method agreement; funnel plots check for publication bias.
- Honest visualization requires: axes starting at zero (for bar charts), no 3D effects, colorblind-safe palettes, individual data points when feasible, and complete labeling.
- Every publication figure should include: descriptive title, axis labels with units, legend, sample sizes, statistical annotations (p-values, effect sizes), and a mention of the color scale for heatmaps.
- The goal of visualization is truth, not beauty. A technically impressive plot that misleads is worse than a simple plot that communicates honestly.

## What's Next

Individual studies provide individual estimates. But what happens when five different labs study the same question and get five different answers? Tomorrow, we tackle meta-analysis — the formal framework for combining results across studies to get a single, more precise estimate. You will build forest plots, assess heterogeneity, check for publication bias, and learn when pooling studies is appropriate and when it is dangerously misleading.
