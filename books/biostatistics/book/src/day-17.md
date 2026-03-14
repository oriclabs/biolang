# Day 17: Survival Analysis — Time-to-Event Data

## The Problem

Dr. Elena Volkov is a cancer genomicist analyzing overall survival in 250 lung adenocarcinoma patients. She has tumor sequencing data and wants to answer: **Do TP53-mutant patients survive longer than TP53 wild-type patients?**

Her first attempt: compute the mean survival time for each group and run a t-test. But she immediately hits a wall. 40% of patients are **still alive** at the end of the study. Their survival time is **at least** 36 months — but she doesn't know their actual survival time. Dropping these patients biases the analysis (the longest survivors are removed). Using 36 months as their survival time underestimates it.

This is the problem of **censoring**, and it requires survival analysis.

## What Is Censoring?

**Right-censoring** occurs when the event of interest (death, relapse, progression) has not yet happened at the time of observation. The patient is lost to follow-up, the study ends, or they die of an unrelated cause.

| Patient | Follow-up | Status | What We Know |
|---------|-----------|--------|--------------|
| A | 24 months | Dead | Survived exactly 24 months |
| B | 36 months | Alive | Survived **at least** 36 months |
| C | 12 months | Lost | Survived **at least** 12 months |
| D | 48 months | Dead | Survived exactly 48 months |

Patients B and C are **censored** — we know a lower bound on their survival, but not the actual value.

> **Key insight:** Censored observations are NOT missing data. They contain real information ("this patient survived at least X months"). Throwing them away wastes data and biases results. Including them as if the event occurred underestimates survival.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="310" viewBox="0 0 650 310" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Censoring in Survival Data</text>
  <text x="325" y="42" text-anchor="middle" font-size="11" fill="#6b7280">Each line = one patient's follow-up period</text>
  <g transform="translate(100, 55)">
    <!-- Time axis -->
    <line x1="100" y1="230" x2="450" y2="230" stroke="#6b7280" stroke-width="1.5"/>
    <text x="275" y="256" text-anchor="middle" font-size="12" fill="#6b7280">Time (months)</text>
    <text x="100" y="246" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
    <text x="170" y="246" text-anchor="middle" font-size="10" fill="#6b7280">6</text>
    <text x="240" y="246" text-anchor="middle" font-size="10" fill="#6b7280">12</text>
    <text x="310" y="246" text-anchor="middle" font-size="10" fill="#6b7280">18</text>
    <text x="380" y="246" text-anchor="middle" font-size="10" fill="#6b7280">24</text>
    <text x="450" y="246" text-anchor="middle" font-size="10" fill="#6b7280">30</text>
    <!-- Tick marks on axis -->
    <line x1="100" y1="228" x2="100" y2="233" stroke="#6b7280" stroke-width="1"/>
    <line x1="170" y1="228" x2="170" y2="233" stroke="#6b7280" stroke-width="1"/>
    <line x1="240" y1="228" x2="240" y2="233" stroke="#6b7280" stroke-width="1"/>
    <line x1="310" y1="228" x2="310" y2="233" stroke="#6b7280" stroke-width="1"/>
    <line x1="380" y1="228" x2="380" y2="233" stroke="#6b7280" stroke-width="1"/>
    <line x1="450" y1="228" x2="450" y2="233" stroke="#6b7280" stroke-width="1"/>
    <!-- Patient timelines -->
    <!-- Patient 1: event at 8 months -->
    <text x="90" y="18" text-anchor="end" font-size="11" fill="#1e293b">Pt 1</text>
    <line x1="100" y1="15" x2="193" y2="15" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <text x="198" y="20" font-size="16" fill="#dc2626" font-weight="bold">X</text>
    <!-- Patient 2: censored at 28 months (still alive) -->
    <text x="90" y="43" text-anchor="end" font-size="11" fill="#1e293b">Pt 2</text>
    <line x1="100" y1="40" x2="427" y2="40" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <circle cx="433" cy="40" r="6" fill="none" stroke="#16a34a" stroke-width="2.5"/>
    <!-- Patient 3: event at 14 months -->
    <text x="90" y="68" text-anchor="end" font-size="11" fill="#1e293b">Pt 3</text>
    <line x1="100" y1="65" x2="263" y2="65" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <text x="268" y="70" font-size="16" fill="#dc2626" font-weight="bold">X</text>
    <!-- Patient 4: censored at 20 months (lost to follow-up) -->
    <text x="90" y="93" text-anchor="end" font-size="11" fill="#1e293b">Pt 4</text>
    <line x1="100" y1="90" x2="333" y2="90" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <circle cx="339" cy="90" r="6" fill="none" stroke="#16a34a" stroke-width="2.5"/>
    <!-- Patient 5: event at 5 months -->
    <text x="90" y="118" text-anchor="end" font-size="11" fill="#1e293b">Pt 5</text>
    <line x1="100" y1="115" x2="158" y2="115" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <text x="163" y="120" font-size="16" fill="#dc2626" font-weight="bold">X</text>
    <!-- Patient 6: censored at 24 months -->
    <text x="90" y="143" text-anchor="end" font-size="11" fill="#1e293b">Pt 6</text>
    <line x1="100" y1="140" x2="374" y2="140" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <circle cx="380" cy="140" r="6" fill="none" stroke="#16a34a" stroke-width="2.5"/>
    <!-- Patient 7: event at 18 months -->
    <text x="90" y="168" text-anchor="end" font-size="11" fill="#1e293b">Pt 7</text>
    <line x1="100" y1="165" x2="310" y2="165" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <text x="315" y="170" font-size="16" fill="#dc2626" font-weight="bold">X</text>
    <!-- Patient 8: event at 22 months -->
    <text x="90" y="193" text-anchor="end" font-size="11" fill="#1e293b">Pt 8</text>
    <line x1="100" y1="190" x2="357" y2="190" stroke="#2563eb" stroke-width="3" stroke-linecap="round"/>
    <text x="362" y="195" font-size="16" fill="#dc2626" font-weight="bold">X</text>
    <!-- Legend -->
    <text x="198" y="218" font-size="15" fill="#dc2626" font-weight="bold">X</text>
    <text x="215" y="218" font-size="10" fill="#1e293b">= Event (death/relapse)</text>
    <circle cx="325" cy="214" r="5" fill="none" stroke="#16a34a" stroke-width="2"/>
    <text x="340" y="218" font-size="10" fill="#1e293b">= Censored (alive / lost)</text>
  </g>
</svg>
</div>

### Why Standard Methods Fail

| Method | Problem with Censored Data |
|--------|---------------------------|
| Mean survival | Can't compute — don't know censored patients' true times |
| t-test | Assumes complete observations |
| Linear regression | Can't handle "at least" values |
| Simple proportions | Ignores timing of events |

## The Kaplan-Meier Estimator

The **Kaplan-Meier (KM) estimator** is the workhorse of survival analysis. It estimates the survival function S(t) = P(survival > t) as a step function that drops at each observed event.

**How it works:**
At each event time tⱼ:

$$\hat{S}(t) = \prod_{t_j \leq t} \left(1 - \frac{d_j}{n_j}\right)$$

Where:
- dⱼ = number of events at time tⱼ
- nⱼ = number at risk just before tⱼ (alive and not yet censored)

**Reading a KM curve:**
- Y-axis: proportion surviving (starts at 1.0 = 100%)
- X-axis: time
- Steps down: events (deaths)
- Tick marks: censored observations (patients lost but still contribute until that point)
- Steeper drops: periods of high event rate
- Flat plateaus: stable periods

### Median Survival

The **median survival** is the time at which the KM curve crosses 0.50 — the point where half the patients have experienced the event. It is more robust than the mean because it is less affected by a few extreme values.

> **Clinical relevance:** "Median overall survival was 18 months in the treatment arm versus 12 months in the control arm" is the standard way clinical trials report survival data. It's the single most important number in oncology clinical trials.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Kaplan-Meier Curve Anatomy</text>
  <g transform="translate(70, 40)">
    <!-- Axes -->
    <line x1="60" y1="280" x2="520" y2="280" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="280" x2="60" y2="20" stroke="#6b7280" stroke-width="1.5"/>
    <text x="290" y="310" text-anchor="middle" font-size="12" fill="#6b7280">Time (months)</text>
    <text x="15" y="150" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 15, 150)">Survival Probability</text>
    <!-- Y-axis labels -->
    <text x="52" y="284" text-anchor="end" font-size="10" fill="#6b7280">0.0</text>
    <text x="52" y="219" text-anchor="end" font-size="10" fill="#6b7280">0.25</text>
    <text x="52" y="154" text-anchor="end" font-size="10" fill="#6b7280">0.50</text>
    <text x="52" y="89" text-anchor="end" font-size="10" fill="#6b7280">0.75</text>
    <text x="52" y="24" text-anchor="end" font-size="10" fill="#6b7280">1.00</text>
    <!-- X-axis labels -->
    <text x="60" y="296" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
    <text x="153" y="296" text-anchor="middle" font-size="10" fill="#6b7280">6</text>
    <text x="243" y="296" text-anchor="middle" font-size="10" fill="#6b7280">12</text>
    <text x="335" y="296" text-anchor="middle" font-size="10" fill="#6b7280">18</text>
    <text x="428" y="296" text-anchor="middle" font-size="10" fill="#6b7280">24</text>
    <text x="520" y="296" text-anchor="middle" font-size="10" fill="#6b7280">30</text>
    <!-- Confidence band (shaded) for Group A -->
    <path d="M 60,20 L 60,20 L 120,20 L 120,40 L 180,40 L 180,65 L 220,65 L 220,90 L 260,90 L 260,115 L 310,115 L 310,140 L 370,140 L 370,170 L 430,170 L 430,195 L 520,195 L 520,235 L 430,235 L 430,210 L 370,210 L 370,185 L 310,185 L 310,160 L 260,160 L 260,135 L 220,135 L 220,110 L 180,110 L 180,80 L 120,80 L 120,55 L 60,55 Z" fill="#2563eb" opacity="0.08"/>
    <!-- Confidence band for Group B -->
    <path d="M 60,20 L 60,20 L 100,20 L 100,50 L 140,50 L 140,85 L 175,85 L 175,120 L 210,120 L 210,155 L 250,155 L 250,195 L 290,195 L 290,225 L 340,225 L 340,258 L 340,278 L 290,278 L 290,255 L 250,255 L 250,225 L 210,225 L 210,190 L 175,190 L 175,155 L 140,155 L 140,120 L 100,120 L 100,75 L 60,75 Z" fill="#dc2626" opacity="0.08"/>
    <!-- KM step curve - Group A (better survival) - blue -->
    <path d="M 60,20 L 120,20 L 120,47 L 180,47 L 180,72 L 220,72 L 220,100 L 260,100 L 260,125 L 310,125 L 310,152 L 370,152 L 370,178 L 430,178 L 430,215 L 520,215" stroke="#2563eb" stroke-width="2.5" fill="none"/>
    <!-- KM step curve - Group B (worse survival) - red -->
    <path d="M 60,20 L 100,20 L 100,62 L 140,62 L 140,102 L 175,102 L 175,140 L 210,140 L 210,175 L 250,175 L 250,212 L 290,212 L 290,240 L 340,240 L 340,268" stroke="#dc2626" stroke-width="2.5" fill="none"/>
    <!-- Censored tick marks on Group A -->
    <line x1="90" y1="16" x2="90" y2="24" stroke="#2563eb" stroke-width="2"/>
    <line x1="150" y1="43" x2="150" y2="51" stroke="#2563eb" stroke-width="2"/>
    <line x1="200" y1="68" x2="200" y2="76" stroke="#2563eb" stroke-width="2"/>
    <line x1="280" y1="121" x2="280" y2="129" stroke="#2563eb" stroke-width="2"/>
    <line x1="350" y1="148" x2="350" y2="156" stroke="#2563eb" stroke-width="2"/>
    <line x1="460" y1="211" x2="460" y2="219" stroke="#2563eb" stroke-width="2"/>
    <!-- Censored tick marks on Group B -->
    <line x1="80" y1="16" x2="80" y2="24" stroke="#dc2626" stroke-width="2"/>
    <line x1="120" y1="58" x2="120" y2="66" stroke="#dc2626" stroke-width="2"/>
    <line x1="192" y1="136" x2="192" y2="144" stroke="#dc2626" stroke-width="2"/>
    <line x1="270" y1="208" x2="270" y2="216" stroke="#dc2626" stroke-width="2"/>
    <!-- Median survival lines -->
    <line x1="60" y1="150" x2="307" y2="150" stroke="#2563eb" stroke-width="1" stroke-dasharray="4,3" opacity="0.5"/>
    <line x1="307" y1="150" x2="307" y2="280" stroke="#2563eb" stroke-width="1" stroke-dasharray="4,3" opacity="0.5"/>
    <line x1="60" y1="150" x2="205" y2="150" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" opacity="0.5"/>
    <line x1="205" y1="150" x2="205" y2="280" stroke="#dc2626" stroke-width="1" stroke-dasharray="4,3" opacity="0.5"/>
    <!-- Median survival label -->
    <text x="52" y="148" text-anchor="end" font-size="9" fill="#7c3aed" font-weight="bold">0.50</text>
    <text x="307" y="326" text-anchor="middle" font-size="9" fill="#2563eb">~17mo</text>
    <text x="205" y="326" text-anchor="middle" font-size="9" fill="#dc2626">~11mo</text>
    <!-- Annotations -->
    <line x1="150" y1="47" x2="165" y2="56" stroke="#6b7280" stroke-width="0.5"/>
    <rect x="140" y="56" width="88" height="16" rx="3" fill="white" stroke="#6b7280" stroke-width="0.5"/>
    <text x="184" y="67" text-anchor="middle" font-size="9" fill="#6b7280">| = censored obs</text>
    <!-- Step annotation -->
    <path d="M 254,100 L 254,125" stroke="#6b7280" stroke-width="0.5" marker-end="url(#arrowhead)"/>
    <text x="254" y="95" text-anchor="middle" font-size="9" fill="#6b7280">step = event</text>
    <!-- Legend -->
    <line x1="390" y1="50" x2="420" y2="50" stroke="#2563eb" stroke-width="2.5"/>
    <text x="425" y="54" font-size="10" fill="#1e293b">TP53 Wild-type</text>
    <line x1="390" y1="68" x2="420" y2="68" stroke="#dc2626" stroke-width="2.5"/>
    <text x="425" y="72" font-size="10" fill="#1e293b">TP53 Mutant</text>
    <rect x="388" y="76" width="8" height="2" fill="#6b7280"/>
    <text x="425" y="84" font-size="9" fill="#6b7280">Confidence band</text>
  </g>
</svg>
</div>

## The Log-Rank Test

The **log-rank test** compares survival curves between groups. It asks: "Is the survival experience significantly different between these groups?"

- **H₀:** The survival curves are identical
- **H₁:** The survival curves differ

The test compares observed events to expected events (under H₀) at each time point across the entire follow-up period. It gives most weight to later time points.

| Consideration | Detail |
|--------------|--------|
| Assumptions | Proportional hazards (constant HR over time) |
| Power | Best when hazard ratio is constant |
| Limitation | Only tests equality — doesn't estimate HOW different |
| Multiple groups | Can compare 3+ groups simultaneously |

> **Common pitfall:** The log-rank test can be non-significant even when curves look different, if the difference is early (then converges) or if curves cross. If hazards are not proportional, consider alternatives like the Wilcoxon (Breslow) test, which gives more weight to early events.

## Cox Proportional Hazards Model

The **Cox PH model** is the regression analog for survival data. It models the hazard (instantaneous event rate) as:

$$h(t|X) = h_0(t) \cdot \exp(\beta_1 X_1 + \beta_2 X_2 + \cdots)$$

Where h₀(t) is the baseline hazard and the exponential term scales it by covariates.

### The Hazard Ratio

The key output is the **hazard ratio (HR)**:

$$HR = e^{\beta}$$

| HR | Interpretation |
|----|---------------|
| 1.0 | No difference in hazard |
| 2.0 | Twice the hazard (worse survival) |
| 0.5 | Half the hazard (better survival) |

> **Key insight:** HR = 2.0 does NOT mean "dies twice as fast" or "survives half as long." It means that at any given time point, the hazard (instantaneous risk of the event) is 2x higher. The relationship between HR and median survival depends on the shape of the baseline hazard.

### Proportional Hazards Assumption

The Cox model assumes that the **ratio of hazards** between groups is constant over time. If TP53-mutant patients have HR = 1.5, this ratio should hold at 6 months, 12 months, and 24 months.

**Violations:**
- Curves that cross (treatment effect reverses over time)
- HR that changes with time (e.g., surgery risk is high early, then protective later)
- Delayed treatment effects (immunotherapy often shows late separation)

### Adjusting for Confounders

Like multiple regression, Cox models can include multiple predictors:

$$h(t) = h_0(t) \cdot \exp(\beta_1 \cdot \text{TP53} + \beta_2 \cdot \text{Age} + \beta_3 \cdot \text{Stage})$$

This gives the HR for TP53 mutation **adjusted for** age and stage — a cleaner estimate of its independent effect.

> **Clinical relevance:** A univariate HR for TP53 might be 1.8 (worse survival). But if TP53-mutant tumors also tend to be higher stage, the adjusted HR might drop to 1.3 after controlling for stage. The adjusted HR is what matters for understanding TP53's independent prognostic value.

## Survival Analysis in BioLang

### Kaplan-Meier Curves

```bio
set_seed(42)
# Lung adenocarcinoma survival by TP53 status
let n = 250

# Simulate TP53 status (40% mutant)
let tp53_mut = rnorm(n, 0, 1) |> map(|x| if x > 0.25 { 0 } else { 1 })

# Simulate survival times (exponential with TP53 effect)
let base_hazard = 0.03  # monthly hazard rate
let survival_time = []
let status = []  # 1 = dead, 0 = censored

for i in 0..n {
    let hazard = base_hazard * if tp53_mut[i] == 1 { 1.6 } else { 1.0 }
    let true_time = -log(rnorm(1, 0.5, 0.2)[0] |> max(0.01)) / hazard
    let censor_time = rnorm(1, 42, 10)[0] |> max(24)

    if true_time < censor_time {
        survival_time = survival_time + [true_time]
        status = status + [1]  # event observed
    } else {
        survival_time = survival_time + [censor_time]
        status = status + [0]  # censored
    }
}

print("Events: {status |> sum} / {n} ({(status |> sum) / n * 100 |> round(1)}%)")
print("Censored: {n - (status |> sum)}")

# Fit Kaplan-Meier for each group
let wt_times = []
let wt_status = []
let mut_times = []
let mut_status = []
for i in 0..n {
    if tp53_mut[i] == 0 {
        wt_times = wt_times + [survival_time[i]]
        wt_status = wt_status + [status[i]]
    } else {
        mut_times = mut_times + [survival_time[i]]
        mut_status = mut_status + [status[i]]
    }
}

let km_wt = kaplan_meier(wt_times, wt_status)
let km_mut = kaplan_meier(mut_times, mut_status)

print("Median survival (TP53 WT): {km_wt.median |> round(1)} months")
print("Median survival (TP53 mut): {km_mut.median |> round(1)} months")
```

### Kaplan-Meier Plot

```bio
# Publication-quality survival curves
# Plot KM data using plot()
let km_table = table({
    "time": km_wt.times ++ km_mut.times,
    "survival": km_wt.survival ++ km_mut.survival,
    "group": km_wt.times |> map(|t| "TP53 WT") ++ km_mut.times |> map(|t| "TP53 Mut")
})
plot(km_table, {type: "line", x: "time", y: "survival", color: "group",
    title: "Overall Survival by TP53 Status",
    x_label: "Time (months)", y_label: "Survival Probability"})
```

### Log-Rank Test

```bio
# Compare survival curves statistically
# Compute log-rank test from KM outputs
# The test compares observed vs expected events across groups
let km_all = kaplan_meier(survival_time, status)

# Use Cox PH as a proxy for log-rank (equivalent for single binary predictor)
let cox_lr = cox_ph(survival_time, status, [tp53_mut])
print("=== Log-Rank Test (via Cox) ===")
print("p-value: {cox_lr.p_value |> round(4)}")

if cox_lr.p_value < 0.05 {
    print("Significant difference in survival between TP53 groups")
} else {
    print("No significant difference detected (p > 0.05)")
}
```

### Cox Proportional Hazards Model

```bio
set_seed(42)
# Simulate additional covariates
let age = rnorm(n, 65, 10)
let stage = rnorm(n, 2.5, 0.8) |> map(|x| max(1, min(4, round(x))))

# Univariate Cox model
let cox_simple = cox_ph(survival_time, status, [tp53_mut])

print("=== Univariate Cox Model ===")
print("TP53 Mutation HR: {cox_simple.hazard_ratio |> round(2)}")
print("  p-value: {cox_simple.p_value |> round(4)}")

# Multivariable Cox model — adjust for age and stage
let cox_adjusted = cox_ph(survival_time, status, [tp53_mut, age, stage])

print("\n=== Multivariable Cox Model ===")
print("Hazard ratios: {cox_adjusted.hazard_ratios}")
print("p-values: {cox_adjusted.p_values}")

# Compare: does TP53 HR change after adjusting for stage?
print("\nTP53 HR unadjusted: {cox_simple.hazard_ratio |> round(2)}")
print("TP53 HR adjusted:   {cox_adjusted.hazard_ratios[0] |> round(2)}")
```

### Forest Plot of Hazard Ratios

```bio
# Visualize all HRs from the multivariable model
let hr_data = table({
    "predictor": ["TP53 Mutation", "Age", "Stage"],
    "hr": cox_adjusted.hazard_ratios,
    "p_value": cox_adjusted.p_values
})
forest_plot(hr_data)

# Left of 1 = protective, Right of 1 = harmful
```

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="280" viewBox="0 0 650 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Forest Plot: Hazard Ratios from Cox Model</text>
  <g transform="translate(30, 45)">
    <!-- Column headers -->
    <text x="110" y="12" text-anchor="end" font-size="11" fill="#6b7280" font-weight="bold">Variable</text>
    <text x="370" y="12" text-anchor="middle" font-size="11" fill="#6b7280" font-weight="bold">Hazard Ratio (95% CI)</text>
    <text x="545" y="12" text-anchor="middle" font-size="11" fill="#6b7280" font-weight="bold">HR [CI]</text>
    <line x1="0" y1="20" x2="600" y2="20" stroke="#e5e7eb" stroke-width="1"/>
    <!-- Reference line at HR=1 -->
    <line x1="370" y1="25" x2="370" y2="205" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4,3"/>
    <text x="370" y="222" text-anchor="middle" font-size="10" fill="#6b7280">HR = 1.0</text>
    <!-- Scale -->
    <text x="250" y="222" text-anchor="middle" font-size="9" fill="#9ca3af">0.5</text>
    <text x="490" y="222" text-anchor="middle" font-size="9" fill="#9ca3af">2.0</text>
    <text x="190" y="222" text-anchor="middle" font-size="9" fill="#9ca3af">0.25</text>
    <line x1="190" y1="210" x2="190" y2="215" stroke="#9ca3af" stroke-width="0.5"/>
    <line x1="250" y1="210" x2="250" y2="215" stroke="#9ca3af" stroke-width="0.5"/>
    <line x1="490" y1="210" x2="490" y2="215" stroke="#9ca3af" stroke-width="0.5"/>
    <!-- Axis line -->
    <line x1="190" y1="210" x2="550" y2="210" stroke="#9ca3af" stroke-width="0.5"/>
    <!-- Row labels -->
    <text x="250" y="238" text-anchor="middle" font-size="10" fill="#16a34a">Protective</text>
    <text x="490" y="238" text-anchor="middle" font-size="10" fill="#dc2626">Harmful</text>
    <!-- Row 1: TP53 Mutation HR=1.6 [1.1, 2.3] -->
    <text x="110" y="55" text-anchor="end" font-size="12" fill="#1e293b">TP53 Mutation</text>
    <line x1="322" y1="52" x2="440" y2="52" stroke="#dc2626" stroke-width="2"/>
    <rect x="392" y="46" width="12" height="12" fill="#dc2626" rx="1"/>
    <text x="545" y="56" text-anchor="middle" font-size="10" fill="#1e293b">1.60 [1.10, 2.30]</text>
    <!-- Row 2: Stage III HR=1.8 [1.2, 2.7] -->
    <text x="110" y="95" text-anchor="end" font-size="12" fill="#1e293b">Stage III</text>
    <line x1="334" y1="92" x2="462" y2="92" stroke="#dc2626" stroke-width="2"/>
    <rect x="406" y="86" width="12" height="12" fill="#dc2626" rx="1"/>
    <text x="545" y="96" text-anchor="middle" font-size="10" fill="#1e293b">1.80 [1.20, 2.70]</text>
    <!-- Row 3: Stage IV HR=3.2 [2.1, 4.9] -->
    <text x="110" y="135" text-anchor="end" font-size="12" fill="#1e293b">Stage IV</text>
    <line x1="410" y1="132" x2="548" y2="132" stroke="#dc2626" stroke-width="2"/>
    <rect x="474" y="126" width="12" height="12" fill="#dc2626" rx="1"/>
    <text x="545" y="136" text-anchor="middle" font-size="10" fill="#1e293b">3.20 [2.10, 4.90]</text>
    <!-- Row 4: Age>60 HR=1.3 [0.9, 1.8] — crosses 1 -->
    <text x="110" y="175" text-anchor="end" font-size="12" fill="#1e293b">Age > 60</text>
    <line x1="346" y1="172" x2="406" y2="172" stroke="#6b7280" stroke-width="2"/>
    <rect x="372" y="166" width="12" height="12" fill="#6b7280" rx="1"/>
    <text x="545" y="176" text-anchor="middle" font-size="10" fill="#6b7280">1.30 [0.90, 1.80]</text>
    <!-- Note about non-significance -->
    <text x="420" y="186" font-size="8" fill="#6b7280" font-style="italic">CI crosses 1 = not significant</text>
  </g>
</svg>
</div>

### Survival Curve from Cox Model

```bio
# Plot adjusted survival curves from KM estimates
let surv_table = table({
    "time": km_wt.times ++ km_mut.times,
    "survival": km_wt.survival ++ km_mut.survival,
    "group": km_wt.times |> map(|t| "TP53 WT") ++ km_mut.times |> map(|t| "TP53 Mut")
})

plot(surv_table, {type: "line", x: "time", y: "survival", color: "group",
    title: "Adjusted Survival Curves from Cox Model",
    x_label: "Time (months)", y_label: "Survival Probability"})
```

### Checking Proportional Hazards

```bio
# Check proportional hazards assumption
# If hazard ratios change over time, PH is violated
# Visual check: compare early vs late HR
let early_times = []
let early_status = []
let early_tp53 = []
let late_times = []
let late_status = []
let late_tp53 = []

let midpoint = 24  # months
for i in 0..n {
    if survival_time[i] <= midpoint {
        early_times = early_times + [survival_time[i]]
        early_status = early_status + [status[i]]
        early_tp53 = early_tp53 + [tp53_mut[i]]
    } else {
        late_times = late_times + [survival_time[i]]
        late_status = late_status + [status[i]]
        late_tp53 = late_tp53 + [tp53_mut[i]]
    }
}

print("=== Proportional Hazards Check ===")
print("If HRs differ substantially, PH assumption may be violated")
# If p < 0.05, consider: stratified Cox model, time-varying coefficients,
# or restricted mean survival time (RMST)
```

**Python:**

```python
from lifelines import KaplanMeierFitter, CoxPHFitter
from lifelines.statistics import logrank_test

# Kaplan-Meier
kmf = KaplanMeierFitter()
kmf.fit(time_wt, event_wt, label='TP53 WT')
kmf.plot()
kmf.fit(time_mut, event_mut, label='TP53 Mut')
kmf.plot()

# Log-rank test
results = logrank_test(time_wt, time_mut, event_wt, event_mut)
print(f"p = {results.p_value:.4f}")

# Cox model
cph = CoxPHFitter()
cph.fit(df, duration_col='time', event_col='status')
cph.print_summary()
cph.plot()  # forest plot

# Check PH assumption
cph.check_assumptions(df, show_plots=True)
```

**R:**

```r
library(survival)
library(survminer)

# Kaplan-Meier
km_fit <- survfit(Surv(time, status) ~ tp53, data = df)

# Publication-quality KM plot
ggsurvplot(km_fit,
           pval = TRUE, risk.table = TRUE,
           palette = c("#2196F3", "#F44336"))

# Log-rank test
survdiff(Surv(time, status) ~ tp53, data = df)

# Cox model
cox_model <- coxph(Surv(time, status) ~ tp53 + age + stage, data = df)
summary(cox_model)

# Forest plot
ggforest(cox_model)

# Proportional hazards test
cox.zph(cox_model)
```

## Exercises

### Exercise 1: Kaplan-Meier and Median Survival

A clinical trial follows 200 breast cancer patients treated with either chemotherapy or chemotherapy + targeted therapy. Compute KM curves and median survival for both arms.

```bio
set_seed(42)
let n = 200
let treatment = rnorm(n, 0, 1) |> map(|x| if x > 0 { 1 } else { 0 })  # 0=chemo, 1=chemo+targeted

# Simulate survival data
# Targeted therapy reduces hazard by 30%
# Generate survival times and censoring

# 1. Compute kaplan_meier() for each treatment arm
# 2. Plot KM curves with plot()
# 3. Report median survival for each arm
# 4. What is the absolute improvement in median survival?
```

### Exercise 2: Log-Rank Test with Multiple Groups

Compare survival across four molecular subtypes (Luminal A, Luminal B, HER2+, Triple-negative). Which pairs differ significantly?

```bio
set_seed(42)
let n = 300
# Assign subtypes: 0=LumA, 1=LumB, 2=HER2+, 3=TNBC
let subtype = rnorm(n, 0, 1) |> map(|x|
    if x < -0.39 { 0 }
    else if x < 0.25 { 1 }
    else if x < 0.64 { 2 }
    else { 3 })

# Simulate different hazard rates by subtype
# Luminal A: lowest hazard, Triple-neg: highest

# 1. Compute kaplan_meier() for all 4 subtypes
# 2. Fit cox_ph() with subtype as predictor
# 3. Which subtypes have the best and worst prognosis?
```

### Exercise 3: Multivariable Cox Model

Build a Cox model with TP53 status, age, stage, and smoking status. Determine which factors are independently prognostic after adjustment.

```bio
let n = 300

# Simulate covariates and survival
# Fit cox_ph() with all 4 predictors
# 1. Report hazard ratios for each predictor
# 2. Which predictors are significant after adjustment?
# 3. Create a forest_plot()
# 4. Does the TP53 hazard ratio change from univariate to multivariable?
```

### Exercise 4: Checking the PH Assumption

Simulate a scenario where the proportional hazards assumption is violated — an immunotherapy that has no effect in the first 3 months but a strong effect afterward (delayed separation). Show that the PH test flags this.

```bio
# Simulate two groups:
# Control: constant hazard
# Treatment: same hazard for months 0-3, then 50% reduced hazard

# 1. Plot KM curves — do they cross or show delayed separation?
# 2. Fit cox_ph() and check if HR changes over time
# 3. Is the PH assumption violated?
# 4. What would you do in practice?
```

### Exercise 5: Complete Survival Analysis Pipeline

Perform a full survival analysis: KM curves, log-rank test, univariate Cox for each predictor, multivariable Cox, forest plot. Write a one-paragraph summary of findings.

```bio
# Use 400 patients with 5 clinical/genomic variables
# Run the complete pipeline from raw data to clinical interpretation
```

## Key Takeaways

- **Censored observations** contain real information — survival analysis methods handle them correctly while standard methods cannot
- **Kaplan-Meier** estimates survival probabilities as a step function; **median survival** is the primary summary measure
- The **log-rank test** compares survival curves between groups (the "t-test" of survival analysis)
- **Cox proportional hazards** regression models the effect of multiple covariates on hazard; the **hazard ratio** is the key output
- HR = 2.0 means twice the instantaneous risk, NOT twice as fast to die or half the survival time
- Always **check the proportional hazards assumption** — violations (crossing curves, delayed effects) invalidate the standard Cox model
- **Forest plots** are the standard visualization for hazard ratios from Cox models
- Adjusted HRs (controlling for confounders) are more clinically meaningful than unadjusted ones

## What's Next

We've been analyzing data. But before collecting data, there's a critical question: **How many samples do you need?** Day 18 covers experimental design and statistical power — the science of planning studies that can actually detect the effects you're looking for.
