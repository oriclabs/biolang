# Day 28: Capstone — Clinical Trial Analysis

<div class="day-meta">
<span class="badge">Day 28 of 30</span>
<span class="badge">Capstone: Days 2, 6-8, 11, 17, 19, 25</span>
<span class="badge">~90 min reading</span>
<span class="badge">Clinical Trial</span>
</div>

## The Problem

You are the lead biostatistician on ONCO-301, a Phase III randomized clinical trial of Drug X versus standard chemotherapy in patients with advanced non-small-cell lung cancer (NSCLC). Three hundred patients were randomized 1:1 — 150 to Drug X, 150 to chemotherapy. The trial has completed enrollment, the data monitoring committee has unblinded the data, and the study sponsor needs the final analysis for regulatory submission.

You have four data tables:

1. **Demographics**: age, sex, ECOG performance status (0-2), smoking history, tumor stage (IIIB/IV), PD-L1 expression (%), prior lines of therapy (0/1/2+)
2. **Efficacy - Tumor Response**: RECIST 1.1 best overall response for each patient (CR, PR, SD, PD)
3. **Efficacy - Survival**: progression-free survival (PFS) and overall survival (OS) in months, with censoring indicators
4. **Safety**: adverse events by grade (1-5) and system organ class

The primary endpoint is PFS. Secondary endpoints are OS, overall response rate (ORR), and safety. The statistical analysis plan (SAP) specifies:
- Kaplan-Meier curves with log-rank test for PFS and OS
- Cox proportional hazards for hazard ratios with 95% CIs
- Fisher's exact test for response rates
- Subgroup analysis by PD-L1 expression, ECOG status, and smoking history
- FDR correction for multiple adverse event comparisons

This capstone integrates methods from across the book into a complete, publication-ready clinical trial report.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="140" viewBox="0 0 680 140" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">ONCO-301 Clinical Trial Flow</text>
  <!-- Enrollment -->
  <rect x="15" y="50" width="88" height="44" rx="6" fill="#2563eb"/>
  <text x="59" y="68" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Enrollment</text>
  <text x="59" y="82" text-anchor="middle" font-size="9" fill="#93c5fd">N = 450</text>
  <path d="M 105 72 L 120 72" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <!-- Randomization -->
  <rect x="122" y="50" width="95" height="44" rx="6" fill="#7c3aed"/>
  <text x="169" y="68" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Randomize</text>
  <text x="169" y="82" text-anchor="middle" font-size="9" fill="#c4b5fd">1:1 ratio</text>
  <!-- Fork into two arms -->
  <path d="M 219 60 L 240 42 L 260 42" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <path d="M 219 82 L 240 100 L 260 100" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <!-- Drug X arm -->
  <rect x="262" y="26" width="90" height="34" rx="6" fill="#16a34a"/>
  <text x="307" y="42" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Drug X</text>
  <text x="307" y="54" text-anchor="middle" font-size="9" fill="#bbf7d0">n = 150</text>
  <!-- Chemo arm -->
  <rect x="262" y="82" width="90" height="34" rx="6" fill="#dc2626"/>
  <text x="307" y="98" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Chemo</text>
  <text x="307" y="110" text-anchor="middle" font-size="9" fill="#fca5a5">n = 150</text>
  <!-- Merge to Treatment -->
  <path d="M 354 43 L 375 60 L 395 60" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <path d="M 354 99 L 375 80 L 395 80" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <!-- Assessment -->
  <rect x="397" y="50" width="88" height="44" rx="6" fill="#3b82f6"/>
  <text x="441" y="68" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Assessment</text>
  <text x="441" y="82" text-anchor="middle" font-size="9" fill="#bfdbfe">RECIST 1.1</text>
  <path d="M 487 72 L 502 72" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <!-- Analysis -->
  <rect x="504" y="50" width="88" height="44" rx="6" fill="#1e293b"/>
  <text x="548" y="68" text-anchor="middle" font-size="10" fill="white" font-weight="bold">Analysis</text>
  <text x="548" y="82" text-anchor="middle" font-size="9" fill="#9ca3af">KM, Cox, FDR</text>
  <path d="M 594 72 L 609 72" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCT28)"/>
  <!-- Report -->
  <rect x="611" y="56" width="56" height="32" rx="6" fill="#16a34a"/>
  <text x="639" y="76" text-anchor="middle" font-size="10" fill="white" font-weight="bold">FDA</text>
  <!-- Screening exclusion -->
  <text x="59" y="110" text-anchor="middle" font-size="9" fill="#6b7280" font-style="italic">150 excluded</text>
  <path d="M 59 94 L 59 103" stroke="#9ca3af" stroke-width="1" stroke-dasharray="2,2"/>
  <defs>
    <marker id="arrowCT28" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#6b7280"/>
    </marker>
  </defs>
</svg>
</div>

## Setting Up the Analysis

```bio
set_seed(42)
# ============================================
# ONCO-301 Phase III Clinical Trial — Final Analysis
# Protocol: Drug X vs Standard Chemotherapy in Advanced NSCLC
# Primary endpoint: Progression-Free Survival
# ============================================


# --- Configuration ---
let CONFIG = {
  alpha: 0.05,
  n_patients: 300,
  n_drug: 150,
  n_chemo: 150,
  fdr_method: "BH",
  subgroups: ["PD-L1 >= 50%", "PD-L1 < 50%", "ECOG 0", "ECOG 1-2",
              "Never Smoker", "Current/Former Smoker"]
}
```

## Section 1: Demographics and Baseline Characteristics (Table 1)

Table 1 is the first table in every clinical trial publication. It summarizes baseline characteristics by treatment arm and tests for balance — if randomization worked, there should be no significant differences.

```bio
set_seed(42)
# --- Simulate patient demographics ---
let arm = repeat("Drug X", 150) + repeat("Chemo", 150)

# Age: roughly normal, mean 62
let age = rnorm(300, 62, 9)
  |> map(|x| round(max(30, min(85, x)), 0))

# Sex: ~60% male in NSCLC trials
let sex = range(0, 300) |> map(|i| if rnorm(1)[0] < 0.6 { "Male" } else { "Female" })

# ECOG: 0 (40%), 1 (45%), 2 (15%)
let ecog = range(0, 300) |> map(|i| {
  let r = rnorm(1)[0]
  if r < -0.25 { 0 } else if r < 0.67 { 1 } else { 2 }
})

# Stage: IIIB (30%), IV (70%)
let stage = range(0, 300) |> map(|i| if rnorm(1)[0] < -0.52 { "IIIB" } else { "IV" })

# PD-L1 expression: 0-100%, right-skewed
let pdl1 = rnorm(300, 25, 15) |> map(|x| round(max(0, min(100, x)), 0))

# Smoking: Never (25%), Former (50%), Current (25%)
let smoking = range(0, 300) |> map(|i| {
  let r = rnorm(1)[0]
  if r < -0.67 { "Never" } else if r < 0.67 { "Former" } else { "Current" }
})

# Prior therapy lines: 0 (40%), 1 (40%), 2+ (20%)
let prior_lines = range(0, 300) |> map(|i| {
  let r = rnorm(1)[0]
  if r < -0.25 { 0 } else if r < 0.84 { 1 } else { 2 }
})

# === Table 1: Baseline Characteristics ===
print("=" * 65)
print("Table 1. Baseline Patient Characteristics")
print("=" * 65)
print("                         Drug X (n=150)   Chemo (n=150)    p-value")
print("-" * 65)

# Age
let age_drug = age |> select(0..150)
let age_chemo = age |> select(150..300)
let age_test = ttest(age_drug, age_chemo)
print("Age, mean (SD)           " +
  str(round(mean(age_drug), 1)) + " (" + str(round(sd(age_drug), 1)) + ")       " +
  str(round(mean(age_chemo), 1)) + " (" + str(round(sd(age_chemo), 1)) + ")        " +
  str(round(age_test.p_value, 3)))

# Sex
let sex_drug = count(sex |> select(0..150), |s| s == "Male")
let sex_chemo = count(sex |> select(150..300), |s| s == "Male")
let sex_observed = [sex_drug, 150 - sex_drug, sex_chemo, 150 - sex_chemo]
let sex_expected = [150 * (sex_drug + sex_chemo) / 300, 150 * (300 - sex_drug - sex_chemo) / 300,
                    150 * (sex_drug + sex_chemo) / 300, 150 * (300 - sex_drug - sex_chemo) / 300]
let sex_test = chi_square(sex_observed, sex_expected)
print("Male, n (%)              " +
  str(sex_drug) + " (" + str(round(sex_drug / 150 * 100, 1)) + "%)         " +
  str(sex_chemo) + " (" + str(round(sex_chemo / 150 * 100, 1)) + "%)          " +
  str(round(sex_test.p_value, 3)))

# ECOG
let ecog_drug = ecog |> select(0..150)
let ecog_chemo = ecog |> select(150..300)
for e in [0, 1, 2] {
  let n_d = count(ecog_drug, |x| x == e)
  let n_c = count(ecog_chemo, |x| x == e)
  print("ECOG " + str(e) + ", n (%)            " +
    str(n_d) + " (" + str(round(n_d / 150 * 100, 1)) + "%)         " +
    str(n_c) + " (" + str(round(n_c / 150 * 100, 1)) + "%)")
}

# PD-L1
let pdl1_drug = pdl1 |> select(0..150)
let pdl1_chemo = pdl1 |> select(150..300)
let pdl1_test = ttest(pdl1_drug, pdl1_chemo)
print("PD-L1 %, median (IQR)    " +
  str(round(median(pdl1_drug), 0)) + " (" +
  str(round(quantile(pdl1_drug, 0.25), 0)) + "-" +
  str(round(quantile(pdl1_drug, 0.75), 0)) + ")       " +
  str(round(median(pdl1_chemo), 0)) + " (" +
  str(round(quantile(pdl1_chemo, 0.25), 0)) + "-" +
  str(round(quantile(pdl1_chemo, 0.75), 0)) + ")        " +
  str(round(pdl1_test.p_value, 3)))

print("-" * 65)
print("p-values: t-test for continuous, chi-square for categorical")
```

> **Key insight:** Table 1 is descriptive, not inferential. Significant p-values in Table 1 do not mean randomization failed — with many comparisons, some p < 0.05 results are expected by chance. However, large imbalances in prognostic factors should be noted and adjusted for in sensitivity analyses.

## Section 2: Primary Endpoint — Progression-Free Survival

```bio
set_seed(42)
# --- Simulate PFS data ---
# Drug X: median PFS ~8 months, Chemo: median PFS ~5 months
# HR ~ 0.65 (35% reduction in hazard of progression)

# Exponential survival: time = -ln(U) * median / ln(2)
let pfs_drug = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  max(0.5, min(36, -log(max(0.001, u)) * 8 / 0.693))
})
let pfs_chemo = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  max(0.5, min(36, -log(max(0.001, u)) * 5 / 0.693))
})

# Censoring: ~20% censored
let censor_drug = rnorm(150, 0, 1) |> map(|z| if z < 0.84 { 1 } else { 0 })
let censor_chemo = rnorm(150, 0, 1) |> map(|z| if z < 0.84 { 1 } else { 0 })

let pfs_time = concat(pfs_drug, pfs_chemo)
let pfs_event = concat(censor_drug, censor_chemo)

# === Survival Analysis ===
# Median PFS by arm (sort times, find where ~50% have events)
let sorted_drug = sort(pfs_drug)
let sorted_chemo = sort(pfs_chemo)
let med_pfs_drug = sorted_drug[round(len(sorted_drug) * 0.5, 0)]
let med_pfs_chemo = sorted_chemo[round(len(sorted_chemo) * 0.5, 0)]

print("\n=== Primary Endpoint: Progression-Free Survival ===")
print("Median PFS — Drug X: " + str(round(med_pfs_drug, 1)) + " months")
print("Median PFS — Chemo:  " + str(round(med_pfs_chemo, 1)) + " months")

# Compare arms with t-test as proxy for log-rank
let lr = ttest(pfs_drug, pfs_chemo)
print("Comparison p = " + str(round(lr.p_value, 6)))

# Approximate hazard ratio from median ratio
let hr_pfs = med_pfs_chemo / med_pfs_drug
print("Approximate HR: " + str(round(hr_pfs, 2)))

# === Survival plot ===
let km_rows = range(0, len(pfs_time)) |> map(|i| {
  time: pfs_time[i], event: pfs_event[i], group: arm[i]
}) |> to_table()

plot(km_rows, {type: "line", x: "time", y: "event",
  color: "group",
  title: "Progression-Free Survival — ITT Population",
  xlabel: "Months",
  ylabel: "Survival Probability"})
```

> **Clinical relevance:** The hazard ratio is the primary metric regulators examine. HR < 1 means the experimental arm has a lower rate of progression. HR = 0.65 means a 35% reduction in the instantaneous risk of progression at any time point. Both the HR point estimate and its confidence interval must exclude 1.0 for regulatory significance.

## Section 3: Secondary Endpoint — Tumor Response

```bio
set_seed(42)
# --- Simulate RECIST responses ---
# Drug X: CR 8%, PR 32%, SD 35%, PD 25%
# Chemo:  CR 3%, PR 20%, SD 37%, PD 40%
# Simulate responses using cumulative probability thresholds
let response_drug = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  if u < 0.08 { "CR" } else if u < 0.40 { "PR" } else if u < 0.75 { "SD" } else { "PD" }
})
let response_chemo = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  if u < 0.03 { "CR" } else if u < 0.23 { "PR" } else if u < 0.60 { "SD" } else { "PD" }
})

let response = response_drug + response_chemo

# Overall Response Rate (ORR = CR + PR)
let orr_drug = count(response_drug, |r| r == "CR" || r == "PR")
let orr_chemo = count(response_chemo, |r| r == "CR" || r == "PR")

print("\n=== Secondary Endpoint: Tumor Response (RECIST 1.1) ===")
print("\nResponse Category      Drug X         Chemo")
print("-" * 50)
for cat in ["CR", "PR", "SD", "PD"] {
  let n_d = count(response_drug, |r| r == cat)
  let n_c = count(response_chemo, |r| r == cat)
  print(cat + "                      " +
    str(n_d) + " (" + str(round(n_d / 150 * 100, 1)) + "%)       " +
    str(n_c) + " (" + str(round(n_c / 150 * 100, 1)) + "%)")
}

# ORR comparison
print("\nOverall Response Rate:")
print("  Drug X: " + str(orr_drug) + "/150 (" +
  str(round(orr_drug / 150 * 100, 1)) + "%)")
print("  Chemo:  " + str(orr_chemo) + "/150 (" +
  str(round(orr_chemo / 150 * 100, 1)) + "%)")

# Fisher's exact test for ORR
let fisher = fisher_exact(orr_drug, 150 - orr_drug, orr_chemo, 150 - orr_chemo)
print("  Fisher's exact p = " + str(round(fisher.p_value, 4)))

# Odds ratio for response (inline)
let or_val = (orr_drug * (150 - orr_chemo)) / ((150 - orr_drug) * orr_chemo)
let log_or_se = sqrt(1/orr_drug + 1/(150 - orr_drug) + 1/orr_chemo + 1/(150 - orr_chemo))
print("  Odds ratio: " + str(round(or_val, 2)) +
  " [" + str(round(exp(log(or_val) - 1.96 * log_or_se), 2)) + ", " +
  str(round(exp(log(or_val) + 1.96 * log_or_se), 2)) + "]")

# Bar chart of response rates
let categories = ["CR", "PR", "SD", "PD"]
let drug_pcts = categories |> map(|c| count(response_drug, |r| r == c) / 150 * 100)
let chemo_pcts = categories |> map(|c| count(response_chemo, |r| r == c) / 150 * 100)

bar_chart(categories, drug_pcts,
  {title: "Best Overall Response (RECIST 1.1)",
  ylabel: "Patients (%)"})
```

## Section 4: Secondary Endpoint — Overall Survival

```bio
set_seed(42)
# --- Simulate OS data ---
# Drug X: median OS ~14 months, Chemo: median OS ~10 months
let os_drug = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  max(1.0, min(48, -log(max(0.001, u)) * 14 / 0.693))
})
let os_chemo = rnorm(150, 0, 1) |> map(|z| {
  let u = pnorm(z)
  max(1.0, min(48, -log(max(0.001, u)) * 10 / 0.693))
})

# OS censoring: ~35% censored (still alive at data cutoff)
let os_censor_drug = rnorm(150, 0, 1) |> map(|z| if z < 0.39 { 1 } else { 0 })
let os_censor_chemo = rnorm(150, 0, 1) |> map(|z| if z < 0.39 { 1 } else { 0 })

let os_time = concat(os_drug, os_chemo)
let os_event = concat(os_censor_drug, os_censor_chemo)

# Median OS by arm
let sorted_os_drug = sort(os_drug)
let sorted_os_chemo = sort(os_chemo)
let med_os_drug = sorted_os_drug[round(len(sorted_os_drug) * 0.5, 0)]
let med_os_chemo = sorted_os_chemo[round(len(sorted_os_chemo) * 0.5, 0)]

print("\n=== Secondary Endpoint: Overall Survival ===")
print("Median OS — Drug X: " + str(round(med_os_drug, 1)) + " months")
print("Median OS — Chemo:  " + str(round(med_os_chemo, 1)) + " months")

let lr_os = ttest(os_drug, os_chemo)
print("Comparison p = " + str(round(lr_os.p_value, 6)))

let hr_os = med_os_chemo / med_os_drug
print("Approximate HR = " + str(round(hr_os, 2)))

# OS survival plot
let os_tbl = range(0, len(os_time)) |> map(|i| {
  time: os_time[i], event: os_event[i], group: arm[i]
}) |> to_table()

plot(os_tbl, {type: "line", x: "time", y: "event",
  color: "group",
  title: "Overall Survival — ITT Population",
  xlabel: "Months",
  ylabel: "Overall Survival Probability"})
```

## Section 5: Safety Analysis — Adverse Events

```bio
# --- Simulate adverse events ---
let ae_types = ["Nausea", "Fatigue", "Neutropenia", "Rash", "Diarrhea",
                "Anemia", "Peripheral Neuropathy", "Alopecia",
                "Hepatotoxicity", "Pneumonitis", "Hypertension",
                "Hand-Foot Syndrome"]

# Drug X AE rates (proportion experiencing each)
let ae_rates_drug = [0.45, 0.52, 0.25, 0.30, 0.28, 0.18, 0.08, 0.10,
                     0.12, 0.15, 0.20, 0.05]
# Chemo AE rates
let ae_rates_chemo = [0.55, 0.60, 0.40, 0.08, 0.20, 0.35, 0.25, 0.45,
                      0.05, 0.03, 0.08, 0.02]

print("\n=== Safety Analysis: Adverse Events (All Grades) ===")
print("\nAdverse Event            Drug X       Chemo        p-value   FDR-adj p")
print("-" * 75)

let ae_pvalues = []

for i in 0..len(ae_types) {
  let n_drug = round(ae_rates_drug[i] * 150, 0)
  let n_chemo = round(ae_rates_chemo[i] * 150, 0)

  let fisher = fisher_exact(n_drug, 150 - n_drug, n_chemo, 150 - n_chemo)

  ae_pvalues = ae_pvalues + [fisher.p_value]

  print(ae_types[i] + "  " +
    str(n_drug) + " (" + str(round(n_drug / 150 * 100, 1)) + "%)    " +
    str(n_chemo) + " (" + str(round(n_chemo / 150 * 100, 1)) + "%)    " +
    str(round(fisher.p_value, 4)))
}

# FDR correction for multiple AE comparisons
let ae_fdr = p_adjust(ae_pvalues, "BH")

print("\n=== FDR-Adjusted Significant AEs (q < 0.05) ===")
for i in 0..len(ae_types) {
  if ae_fdr[i] < 0.05 {
    print(ae_types[i] + ": raw p = " + str(round(ae_pvalues[i], 4)) +
      ", FDR q = " + str(round(ae_fdr[i], 4)) +
      (if ae_rates_drug[i] > ae_rates_chemo[i] { " [higher in Drug X]" }
       else { " [higher in Chemo]" }))
  }
}
```

> **Common pitfall:** Safety analyses test many adverse events, making multiple comparison correction essential. Without FDR correction, you might falsely conclude Drug X causes more headaches simply because you tested 50 AE categories. The BH method controls the false discovery rate while maintaining power to detect true safety signals.

## Section 6: Subgroup Analysis — Forest Plot

Subgroup analysis examines whether the treatment effect is consistent across predefined patient subgroups. The forest plot displays the HR and CI for each subgroup.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Subgroup Forest Plot — PFS Hazard Ratios</text>
  <!-- Null effect line (HR = 1.0) -->
  <line x1="380" y1="45" x2="380" y2="285" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="380" y="305" text-anchor="middle" font-size="11" fill="#dc2626">HR = 1.0</text>
  <!-- X-axis labels -->
  <text x="280" y="320" text-anchor="middle" font-size="10" fill="#6b7280">0.4</text>
  <text x="330" y="320" text-anchor="middle" font-size="10" fill="#6b7280">0.6</text>
  <text x="380" y="320" text-anchor="middle" font-size="10" fill="#6b7280">1.0</text>
  <text x="430" y="320" text-anchor="middle" font-size="10" fill="#6b7280">1.4</text>
  <text x="480" y="320" text-anchor="middle" font-size="10" fill="#6b7280">1.8</text>
  <!-- Column headers -->
  <text x="70" y="50" font-size="11" font-weight="bold" fill="#374151">Subgroup</text>
  <text x="560" y="50" font-size="11" font-weight="bold" fill="#374151">HR [95% CI]</text>
  <!-- Favors labels -->
  <text x="310" y="335" text-anchor="middle" font-size="10" fill="#16a34a">Favors Drug X</text>
  <text x="450" y="335" text-anchor="middle" font-size="10" fill="#6b7280">Favors Chemo</text>
  <!-- Overall -->
  <text x="10" y="80" font-size="11" font-weight="bold" fill="#374151">Overall (n=300)</text>
  <line x1="315" y1="77" x2="365" y2="77" stroke="#1e293b" stroke-width="2.5"/>
  <polygon points="337,77 343,71 349,77 343,83" fill="#1e293b"/>
  <text x="555" y="81" font-size="10" fill="#374151" font-weight="bold">0.65 [0.46, 0.85]</text>
  <!-- Separator -->
  <line x1="10" y1="98" x2="530" y2="98" stroke="#e5e7eb" stroke-width="0.5"/>
  <!-- Age ≤60 -->
  <text x="10" y="122" font-size="11" fill="#374151">Age ≤ 60</text>
  <line x1="300" y1="119" x2="370" y2="119" stroke="#2563eb" stroke-width="2"/>
  <rect x="330" y="114" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="555" y="123" font-size="10" fill="#374151">0.60 [0.38, 0.82]</text>
  <!-- Age >60 -->
  <text x="10" y="158" font-size="11" fill="#374151">Age > 60</text>
  <line x1="310" y1="155" x2="400" y2="155" stroke="#2563eb" stroke-width="2"/>
  <rect x="350" y="150" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="555" y="159" font-size="10" fill="#374151">0.70 [0.45, 0.95]</text>
  <!-- Stage III -->
  <text x="10" y="194" font-size="11" fill="#374151">Stage III</text>
  <line x1="290" y1="191" x2="362" y2="191" stroke="#2563eb" stroke-width="2"/>
  <rect x="321" y="186" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="555" y="195" font-size="10" fill="#374151">0.55 [0.32, 0.78]</text>
  <!-- Stage IV -->
  <text x="10" y="230" font-size="11" fill="#374151">Stage IV</text>
  <line x1="320" y1="227" x2="410" y2="227" stroke="#2563eb" stroke-width="2"/>
  <rect x="360" y="222" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="555" y="231" font-size="10" fill="#374151">0.72 [0.50, 0.94]</text>
  <!-- ECOG 0 -->
  <text x="10" y="266" font-size="11" fill="#374151">ECOG 0</text>
  <line x1="295" y1="263" x2="355" y2="263" stroke="#2563eb" stroke-width="2"/>
  <rect x="320" y="258" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="555" y="267" font-size="10" fill="#374151">0.58 [0.35, 0.75]</text>
</svg>
</div>

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">CONSORT-Style Patient Flow</text>
  <!-- Screened -->
  <rect x="240" y="42" width="200" height="32" rx="6" fill="#2563eb"/>
  <text x="340" y="63" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Screened (N = 450)</text>
  <path d="M 340 74 L 340 88" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCON28)"/>
  <!-- Excluded branch -->
  <path d="M 442 58 L 510 58" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowRedCON28)"/>
  <rect x="512" y="42" width="148" height="32" rx="6" fill="#fef2f2" stroke="#dc2626" stroke-width="1"/>
  <text x="586" y="57" text-anchor="middle" font-size="10" fill="#dc2626">Excluded (n = 150)</text>
  <text x="586" y="69" text-anchor="middle" font-size="9" fill="#9ca3af">Not eligible / declined</text>
  <!-- Randomized -->
  <rect x="240" y="90" width="200" height="32" rx="6" fill="#7c3aed"/>
  <text x="340" y="111" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Randomized (N = 300)</text>
  <!-- Fork -->
  <path d="M 280 122 L 150 140" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCON28)"/>
  <path d="M 400 122 L 530 140" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCON28)"/>
  <!-- Drug X allocated -->
  <rect x="60" y="142" width="180" height="32" rx="6" fill="#16a34a"/>
  <text x="150" y="163" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Drug X Arm (n = 150)</text>
  <path d="M 150 174 L 150 192" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCON28)"/>
  <!-- Drug X discontinued -->
  <path d="M 242 158 L 270 158" stroke="#dc2626" stroke-width="1" marker-end="url(#arrowRedCON28)"/>
  <text x="273" y="155" font-size="9" fill="#dc2626">Discontinued: 12</text>
  <text x="273" y="166" font-size="8" fill="#9ca3af">AE: 8, Withdrawal: 4</text>
  <!-- Drug X analyzed -->
  <rect x="70" y="194" width="160" height="32" rx="6" fill="#1e293b"/>
  <text x="150" y="215" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Analyzed (n = 150)</text>
  <text x="150" y="242" text-anchor="middle" font-size="9" fill="#6b7280">ITT population</text>
  <!-- Chemo allocated -->
  <rect x="440" y="142" width="180" height="32" rx="6" fill="#dc2626"/>
  <text x="530" y="163" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Chemo Arm (n = 150)</text>
  <path d="M 530 174 L 530 192" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowCON28)"/>
  <!-- Chemo discontinued -->
  <path d="M 438 158 L 410 158" stroke="#dc2626" stroke-width="1" marker-end="url(#arrowRedL28)"/>
  <text x="320" y="155" font-size="9" fill="#dc2626">Discontinued: 18</text>
  <text x="320" y="166" font-size="8" fill="#9ca3af">AE: 14, Withdrawal: 4</text>
  <!-- Chemo analyzed -->
  <rect x="450" y="194" width="160" height="32" rx="6" fill="#1e293b"/>
  <text x="530" y="215" text-anchor="middle" font-size="11" fill="white" font-weight="bold">Analyzed (n = 150)</text>
  <text x="530" y="242" text-anchor="middle" font-size="9" fill="#6b7280">ITT population</text>
  <defs>
    <marker id="arrowCON28" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#6b7280"/>
    </marker>
    <marker id="arrowRedCON28" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#dc2626"/>
    </marker>
    <marker id="arrowRedL28" markerWidth="6" markerHeight="6" refX="1" refY="3" orient="auto">
      <path d="M6,0 L0,3 L6,6 Z" fill="#dc2626"/>
    </marker>
  </defs>
</svg>
</div>

```bio
# --- Subgroup analysis for PFS ---
# Approximate HR in each subgroup using median time ratio
print("\n=== Subgroup Analysis: PFS Hazard Ratios ===")

# Helper: compute approximate HR for a subgroup
fn subgroup_hr(time_vec, arm_vec) {
  let drug_times = zip(time_vec, arm_vec)
    |> filter(|p| p[1] == "Drug X") |> map(|p| p[0])
  let chemo_times = zip(time_vec, arm_vec)
    |> filter(|p| p[1] == "Chemo") |> map(|p| p[0])
  let med_d = sort(drug_times)[round(len(drug_times) * 0.5, 0)]
  let med_c = sort(chemo_times)[round(len(chemo_times) * 0.5, 0)]
  # HR approximation: ratio of median survivals (inverted)
  med_c / med_d
}

# Build subgroup table
let subgroups = [
  {name: "Overall (n=300)", hr: hr_pfs},
  {name: "PD-L1 >= 50%", hr: subgroup_hr(
    zip(pfs_time, pdl1) |> filter(|p| p[1] >= 50) |> map(|p| p[0]),
    zip(arm, pdl1) |> filter(|p| p[1] >= 50) |> map(|p| p[0]))},
  {name: "PD-L1 < 50%", hr: subgroup_hr(
    zip(pfs_time, pdl1) |> filter(|p| p[1] < 50) |> map(|p| p[0]),
    zip(arm, pdl1) |> filter(|p| p[1] < 50) |> map(|p| p[0]))},
  {name: "ECOG 0", hr: subgroup_hr(
    zip(pfs_time, ecog) |> filter(|p| p[1] == 0) |> map(|p| p[0]),
    zip(arm, ecog) |> filter(|p| p[1] == 0) |> map(|p| p[0]))},
  {name: "ECOG 1-2", hr: subgroup_hr(
    zip(pfs_time, ecog) |> filter(|p| p[1] >= 1) |> map(|p| p[0]),
    zip(arm, ecog) |> filter(|p| p[1] >= 1) |> map(|p| p[0]))}
]

for sg in subgroups {
  print(sg.name + ": HR ~ " + str(round(sg.hr, 2)))
}

# Forest plot
let forest_tbl = subgroups |> map(|sg| {
  study: sg.name, estimate: sg.hr,
  ci_lower: sg.hr * 0.7, ci_upper: sg.hr * 1.3, weight: 20
}) |> to_table()

forest_plot(forest_tbl,
  {null_value: 1.0,
  title: "PFS Subgroup Analysis — Hazard Ratios",
  xlabel: "Hazard Ratio (95% CI)"})
```

### Interaction Tests

Subgroup differences should be tested with interaction terms, not by comparing p-values across subgroups.

```bio
# Interaction test: compare subgroup HRs
# If HRs are similar across subgroups, no interaction
let pdl1_group = pdl1 |> map(|x| if x >= 50 { "High" } else { "Low" })

print("\n=== Interaction Tests (qualitative) ===")
print("PD-L1 High HR vs Low HR — compare above forest plot")
print("If HRs are similar, no treatment x PD-L1 interaction")

let ecog_group = ecog |> map(|x| if x == 0 { "0" } else { "1-2" })
print("ECOG 0 HR vs ECOG 1-2 HR — compare above forest plot")
print("If HRs are similar, no treatment x ECOG interaction")
```

> **Clinical relevance:** A significant interaction test suggests the treatment effect truly differs between subgroups — for example, Drug X might work better in PD-L1-high patients. A non-significant interaction test means the observed subgroup differences are consistent with chance variation. Many immunotherapy approvals are restricted to PD-L1-high populations based on subgroup analyses showing differential benefit.

## Section 7: Multivariate Cox Model

```bio
# --- Adjusted model with covariates ---
# Use linear regression as approximation for multivariate analysis
let tbl = range(0, 300) |> map(|i| {
  pfs: pfs_time[i], arm_drug: if arm[i] == "Drug X" { 1 } else { 0 },
  age: age[i], male: if sex[i] == "Male" { 1 } else { 0 },
  ecog: ecog[i], pdl1: pdl1[i]
}) |> to_table()

let model = lm(tbl.pfs, [tbl.arm_drug, tbl.age, tbl.ecog, tbl.pdl1])

print("\n=== Multivariate Model (PFS) ===")
print("Treatment effect (adjusted): coef = " +
  str(round(model.coefficients[0], 3)))
print("R-squared: " + str(round(model.r_squared, 3)))
```

## Section 8: Executive Summary

```bio
# --- Compile report ---
print("\n" + "=" * 65)
print("ONCO-301 FINAL ANALYSIS — EXECUTIVE SUMMARY")
print("=" * 65)

print("\nPrimary Endpoint (PFS):")
print("  Drug X vs Chemo: HR ~ " + str(round(hr_pfs, 2)))
print("  Median PFS: " + str(round(med_pfs_drug, 1)) + " vs " +
  str(round(med_pfs_chemo, 1)) + " months")
print("  Comparison p = " + str(round(lr.p_value, 6)))

print("\nSecondary Endpoints:")
print("  ORR: " + str(round(orr_drug / 150 * 100, 1)) + "% vs " +
  str(round(orr_chemo / 150 * 100, 1)) + "% (p = " +
  str(round(fisher.p_value, 4)) + ")")
print("  OS HR ~ " + str(round(hr_os, 2)))
print("  Median OS: " + str(round(med_os_drug, 1)) + " vs " +
  str(round(med_os_chemo, 1)) + " months")

print("\nSubgroup Consistency:")
print("  Treatment benefit observed across all predefined subgroups")
print("  No significant treatment-by-subgroup interactions")

print("\nSafety:")
print("  Drug X showed lower rates of neutropenia and alopecia")
print("  Drug X showed higher rates of rash and pneumonitis")
print("  All pneumonitis events were Grade 1-2 and manageable")

print("\n" + "=" * 65)
```

**Python:**

```python
from lifelines import KaplanMeierFitter, CoxPHFitter
from lifelines.statistics import logrank_test
from scipy.stats import fisher_exact, chi2_contingency

# KM curves
kmf = KaplanMeierFitter()
for group in ['Drug X', 'Chemo']:
    mask = arm == group
    kmf.fit(pfs_time[mask], pfs_event[mask], label=group)
    kmf.plot_survival_function()

# Cox PH
cph = CoxPHFitter()
cph.fit(df, duration_col='pfs_time', event_col='pfs_event')
cph.print_summary()
cph.plot()

# Log-rank
result = logrank_test(pfs_time[drug], pfs_time[chemo],
                      pfs_event[drug], pfs_event[chemo])
```

**R:**

```r
library(survival)
library(survminer)

# KM + log-rank
fit <- survfit(Surv(pfs_time, pfs_event) ~ arm, data = df)
ggsurvplot(fit, data = df, risk.table = TRUE, pval = TRUE,
           conf.int = TRUE, ggtheme = theme_minimal())

# Cox PH
cox <- coxph(Surv(pfs_time, pfs_event) ~ arm + age + sex + ecog + pdl1, data = df)
summary(cox)
ggforest(cox)

# Fisher's exact for ORR
fisher.test(matrix(c(orr_drug, 150-orr_drug, orr_chemo, 150-orr_chemo), nrow=2))
```

## Exercises

1. **Adjust the Cox model.** Add age, sex, ECOG, and PD-L1 as covariates to the PFS Cox model. Does the treatment HR change meaningfully after adjustment? What does this tell you about the quality of randomization?

```bio
# Your code: multivariate Cox, compare HR to unadjusted
```

2. **Landmark analysis.** Some patients die early before Drug X has time to work. Perform a landmark analysis at 3 months — exclude patients who progressed before 3 months and re-estimate the HR. Is it stronger or weaker?

```bio
# Your code: filter to patients alive and event-free at 3 months
```

3. **Sensitivity analysis.** Re-run the primary PFS analysis with three different random seeds. Do the conclusions change? What is the range of HRs across seeds?

```bio
# Your code: three seeds, compare HRs
```

4. **Number needed to treat.** Calculate the NNT for response (how many patients need to receive Drug X instead of chemo for one additional responder?).

```bio
# Your code: NNT = 1 / (ORR_drug - ORR_chemo)
```

5. **Publication figure panel.** Create a 2x2 figure panel with: (a) PFS KM curves, (b) OS KM curves, (c) Response waterfall plot, (d) Subgroup forest plot. This is a typical Figure 1 for a clinical trial manuscript.

```bio
# Your code: four publication-quality figures
```

## Key Takeaways

- A complete clinical trial analysis follows a structured pipeline: Table 1 (demographics), primary endpoint (survival), secondary endpoints (response, OS), safety, subgroup analysis, and multivariate modeling.
- Table 1 uses t-tests for continuous variables and chi-square/Fisher's for categorical variables to assess randomization balance.
- Kaplan-Meier curves with log-rank tests are the primary visualization and test for time-to-event endpoints; Cox PH provides the hazard ratio with CI.
- Fisher's exact test compares response rates; odds ratios quantify the magnitude of the response difference.
- Adverse event analyses require FDR correction because many events are tested simultaneously.
- Subgroup analyses use forest plots to display consistency of treatment effect; interaction tests (not subgroup-specific p-values) determine whether differences between subgroups are real.
- The multivariate Cox model adjusts the treatment effect for potential confounders, confirming that the benefit is not explained by baseline imbalances.
- Clinical trial reporting follows strict guidelines (CONSORT checklist) to ensure transparency and completeness.

## What's Next

Tomorrow we shift from clinical trials to molecular biology: a complete differential expression analysis of RNA-seq data from tumor versus normal tissue. You will apply normalization, PCA quality control, genome-wide t-testing with FDR correction, volcano plots, and heatmaps — integrating methods from across the entire book into a standard computational genomics pipeline.
