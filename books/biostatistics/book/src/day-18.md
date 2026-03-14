# Day 18: Experimental Design and Statistical Power

## The Problem

Dr. Ana Reyes is a junior PI writing her first R01 grant. She proposes a study comparing gene expression between psoriatic skin and normal skin using RNA-seq, planning **3 samples per group** because "that's what the lab down the hall used."

The grant comes back with this reviewer comment:

> *"The proposed sample size of n=3 per group is inadequate. The applicant provides no power analysis to justify this number. With 3 replicates, the study is severely underpowered to detect anything less than a 4-fold change, which is biologically unrealistic for most genes. We recommend at least 8-10 biological replicates per condition based on published power analyses for RNA-seq DE studies."*

Grant rejected. Six months of proposal writing, wasted — because she didn't plan the sample size.

**Statistical power** determines whether your study can actually detect the effect you're looking for. Getting it wrong wastes time, money, animals, and patient samples.

## What Is Statistical Power?

Power analysis sits at the intersection of four interconnected quantities:

| Quantity | Symbol | Definition | Typical Value |
|----------|--------|-----------|---------------|
| **Significance level** | α | Probability of false positive (Type I error) | 0.05 |
| **Power** | 1 - β | Probability of detecting a true effect | 0.80 (80%) |
| **Effect size** | d, Δ | Magnitude of the real biological difference | Varies |
| **Sample size** | n | Number of observations per group | What you solve for |

**The fundamental relationship:** Given any three, you can calculate the fourth.

### Type I and Type II Errors

| | H₀ True (no real effect) | H₀ False (real effect exists) |
|---|---|---|
| **Reject H₀** | Type I error (α) — false positive | Correct! (Power = 1 - β) |
| **Fail to reject H₀** | Correct! (1 - α) | Type II error (β) — false negative |

> **Key insight:** An underpowered study has a high β — it frequently misses real effects. This doesn't just waste resources; it can lead to the false conclusion that an effect doesn't exist, discouraging further research on a real phenomenon.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Statistical Power: Type I and Type II Errors</text>
  <g transform="translate(50, 50)">
    <!-- Axis -->
    <line x1="30" y1="220" x2="580" y2="220" stroke="#6b7280" stroke-width="1.5"/>
    <text x="305" y="252" text-anchor="middle" font-size="12" fill="#6b7280">Test Statistic</text>
    <!-- H0 distribution (centered at 180) -->
    <path d="M 30,220 C 50,218 70,210 90,195 C 110,170 130,130 150,90 C 160,70 170,55 180,48 C 190,55 200,70 210,90 C 230,130 250,170 270,195 C 290,210 310,218 330,220" fill="#3b82f6" opacity="0.15" stroke="#3b82f6" stroke-width="2"/>
    <text x="180" y="42" text-anchor="middle" font-size="12" fill="#2563eb" font-weight="bold">H0 (no effect)</text>
    <!-- H1 distribution (centered at 380) -->
    <path d="M 230,220 C 250,218 270,210 290,195 C 310,170 330,130 350,90 C 360,70 370,55 380,48 C 390,55 400,70 410,90 C 430,130 450,170 470,195 C 490,210 510,218 530,220" fill="#dc2626" opacity="0.15" stroke="#dc2626" stroke-width="2"/>
    <text x="380" y="42" text-anchor="middle" font-size="12" fill="#dc2626" font-weight="bold">H1 (true effect)</text>
    <!-- Critical value line -->
    <line x1="290" y1="15" x2="290" y2="225" stroke="#1e293b" stroke-width="2" stroke-dasharray="5,3"/>
    <text x="290" y="265" text-anchor="middle" font-size="10" fill="#1e293b" font-weight="bold">Critical value</text>
    <!-- Alpha region (right tail of H0) -->
    <path d="M 290,195 C 300,200 310,208 320,212 C 330,216 335,220 330,220 L 290,220 Z" fill="#2563eb" opacity="0.5"/>
    <line x1="300" y1="200" x2="340" y2="170" stroke="#2563eb" stroke-width="0.8"/>
    <rect x="340" y="158" width="95" height="30" rx="4" fill="white" stroke="#2563eb" stroke-width="0.8"/>
    <text x="387" y="172" text-anchor="middle" font-size="10" fill="#2563eb" font-weight="bold">alpha (Type I)</text>
    <text x="387" y="184" text-anchor="middle" font-size="9" fill="#2563eb">False positive</text>
    <!-- Beta region (left tail of H1, before critical value) -->
    <path d="M 230,220 C 250,218 260,214 270,208 C 278,202 284,198 290,195 L 290,220 Z" fill="#dc2626" opacity="0.4"/>
    <line x1="260" y1="210" x2="200" y2="170" stroke="#dc2626" stroke-width="0.8"/>
    <rect x="130" y="158" width="95" height="30" rx="4" fill="white" stroke="#dc2626" stroke-width="0.8"/>
    <text x="177" y="172" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">beta (Type II)</text>
    <text x="177" y="184" text-anchor="middle" font-size="9" fill="#dc2626">False negative</text>
    <!-- Power region (right portion of H1) -->
    <path d="M 290,195 C 310,170 330,130 350,90 C 360,70 370,55 380,48 C 390,55 400,70 410,90 C 430,130 450,170 470,195 C 490,210 510,218 530,220 L 290,220 Z" fill="#16a34a" opacity="0.2"/>
    <rect x="395" y="100" width="120" height="30" rx="4" fill="white" stroke="#16a34a" stroke-width="0.8"/>
    <text x="455" y="114" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">Power = 1 - beta</text>
    <text x="455" y="126" text-anchor="middle" font-size="9" fill="#16a34a">Correctly detect effect</text>
  </g>
</svg>
</div>

### Why 80% Power?

The convention of 80% power means accepting a 20% chance of missing a real effect. Some contexts demand higher:

| Context | Minimum Power | Rationale |
|---------|---------------|-----------|
| Exploratory study | 70-80% | Acceptable miss rate for discovery |
| Confirmatory clinical trial | 80-90% | Regulatory requirement |
| Safety/non-inferiority trial | 90-95% | Must not miss harmful effects |
| Rare disease (limited patients) | 60-70% | Pragmatic constraint |

## Effect Size: The Missing Ingredient

Effect size is the hardest quantity to estimate because it requires knowledge about the biology before doing the experiment. Sources:

| Source | Approach |
|--------|----------|
| **Pilot data** | Small preliminary experiment (best source) |
| **Literature** | Previous studies on similar questions |
| **Clinical significance** | "What's the smallest difference that matters?" |
| **Conventions** | Cohen's standards (d = 0.2 small, 0.5 medium, 0.8 large) |

> **Common pitfall:** Using an inflated effect size from a small pilot study. Small studies overestimate effects (the "winner's curse"). If your pilot with n=5 shows d=1.5, the true effect is probably smaller. Be conservative.

### Cohen's d for Two-Group Comparisons

$$d = \frac{|\mu_1 - \mu_2|}{\sigma_{pooled}}$$

| Cohen's d | Interpretation | Biological Example |
|-----------|---------------|--------------------|
| 0.2 | Small | Subtle expression change between tissues |
| 0.5 | Medium | DE gene in RNA-seq (2-fold change) |
| 0.8 | Large | Drug vs. placebo in responsive patients |
| 1.2 | Very large | Knockout vs. wild-type for target gene |

## Power for Common Designs

### Two-Group Comparison (t-test)

The most basic design: compare means between two independent groups.

**Required sample size per group** (approximate, for α=0.05, power=0.80):

| Effect Size (d) | n per group |
|-----------------|-------------|
| 0.2 (small) | 394 |
| 0.5 (medium) | 64 |
| 0.8 (large) | 26 |
| 1.0 | 17 |
| 1.5 | 9 |
| 2.0 | 6 |

> **Key insight:** Detecting a small effect requires nearly 400 samples per group! This is why GWAS studies need thousands of subjects — individual genetic variants typically have very small effects (d ≈ 0.1-0.2).

### Paired Design (Paired t-test)

When the same subjects are measured before and after treatment, pairing removes between-subject variability. Power increases dramatically:

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Paired vs. Unpaired Design: Why Pairing Reduces Noise</text>
  <!-- Left: Unpaired -->
  <g transform="translate(20, 40)">
    <text x="150" y="16" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Unpaired (Independent Groups)</text>
    <rect x="20" y="25" width="260" height="195" rx="6" fill="white" stroke="#e5e7eb" stroke-width="1"/>
    <!-- Group A points (scattered widely) -->
    <text x="90" y="45" text-anchor="middle" font-size="10" fill="#2563eb" font-weight="bold">Group A</text>
    <circle cx="60" cy="80" r="5" fill="#3b82f6" opacity="0.6"/><circle cx="80" cy="120" r="5" fill="#3b82f6" opacity="0.6"/>
    <circle cx="55" cy="160" r="5" fill="#3b82f6" opacity="0.6"/><circle cx="100" cy="95" r="5" fill="#3b82f6" opacity="0.6"/>
    <circle cx="75" cy="140" r="5" fill="#3b82f6" opacity="0.6"/><circle cx="110" cy="180" r="5" fill="#3b82f6" opacity="0.6"/>
    <!-- Group B points (scattered widely) -->
    <text x="210" y="45" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">Group B</text>
    <circle cx="190" cy="70" r="5" fill="#ef4444" opacity="0.6"/><circle cx="210" cy="110" r="5" fill="#ef4444" opacity="0.6"/>
    <circle cx="185" cy="145" r="5" fill="#ef4444" opacity="0.6"/><circle cx="230" cy="85" r="5" fill="#ef4444" opacity="0.6"/>
    <circle cx="205" cy="130" r="5" fill="#ef4444" opacity="0.6"/><circle cx="240" cy="170" r="5" fill="#ef4444" opacity="0.6"/>
    <!-- Variance arrows -->
    <text x="150" y="208" text-anchor="middle" font-size="10" fill="#6b7280">Between-subject variance</text>
    <text x="150" y="220" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">dominates the signal</text>
  </g>
  <!-- Right: Paired -->
  <g transform="translate(360, 40)">
    <text x="150" y="16" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Paired (Same Subjects)</text>
    <rect x="20" y="25" width="260" height="195" rx="6" fill="white" stroke="#e5e7eb" stroke-width="1"/>
    <!-- Paired points connected by lines -->
    <text x="70" y="45" text-anchor="middle" font-size="10" fill="#6b7280">Before</text>
    <text x="230" y="45" text-anchor="middle" font-size="10" fill="#6b7280">After</text>
    <!-- Subject 1 -->
    <circle cx="70" cy="70" r="5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="230" cy="60" r="5" fill="#16a34a" opacity="0.7"/>
    <line x1="75" y1="70" x2="225" y2="60" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
    <!-- Subject 2 -->
    <circle cx="70" cy="100" r="5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="230" cy="88" r="5" fill="#16a34a" opacity="0.7"/>
    <line x1="75" y1="100" x2="225" y2="88" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
    <!-- Subject 3 -->
    <circle cx="70" cy="130" r="5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="230" cy="118" r="5" fill="#16a34a" opacity="0.7"/>
    <line x1="75" y1="130" x2="225" y2="118" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
    <!-- Subject 4 -->
    <circle cx="70" cy="155" r="5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="230" cy="143" r="5" fill="#16a34a" opacity="0.7"/>
    <line x1="75" y1="155" x2="225" y2="143" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
    <!-- Subject 5 -->
    <circle cx="70" cy="180" r="5" fill="#3b82f6" opacity="0.7"/>
    <circle cx="230" cy="168" r="5" fill="#16a34a" opacity="0.7"/>
    <line x1="75" y1="180" x2="225" y2="168" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
    <!-- Annotation -->
    <text x="150" y="208" text-anchor="middle" font-size="10" fill="#6b7280">Each subject is its own control</text>
    <text x="150" y="220" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">Only within-subject noise matters</text>
  </g>
</svg>
</div>

| Correlation between pairs | Power improvement |
|---------------------------|-------------------|
| 0.3 | ~30% fewer samples needed |
| 0.5 | ~50% fewer samples needed |
| 0.7 | ~70% fewer samples needed |

### RNA-seq Differential Expression

RNA-seq power depends on additional factors:

| Factor | Effect on Power |
|--------|----------------|
| Sequencing depth | More reads → more power for low-expression genes |
| Biological replicates | THE major driver of power |
| Fold change threshold | Larger FC → easier to detect |
| Dispersion | Higher variability → need more samples |

**Rules of thumb for RNA-seq DE:**
- **Minimum:** 3 biological replicates (detects only >4-fold changes)
- **Good:** 6-8 replicates (detects 2-fold changes)
- **Ideal:** 12+ replicates (detects 1.5-fold changes)
- Technical replicates have diminishing returns — invest in biological replicates

> **Common pitfall:** Confusing technical replicates (sequencing the same library twice) with biological replicates (independent biological samples). Only biological replicates give you power to generalize. Ten technical replicates of one sample give you n=1, not n=10.

### GWAS

| Study Type | Typical n | Detectable Effect |
|------------|-----------|-------------------|
| Candidate gene | 500-1000 | Large OR (>2.0) |
| Moderate GWAS | 5,000-10,000 | Medium OR (1.3-1.5) |
| Large GWAS | 50,000-500,000 | Small OR (1.05-1.1) |
| UK Biobank scale | 500,000+ | Tiny effects |

## Power Curves

**Power curves** show how power varies with sample size for different effect sizes. They're the most informative visualization for study planning — you can see the "sweet spot" where adding more samples gives diminishing returns.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Power Curves by Effect Size</text>
  <g transform="translate(80, 40)">
    <!-- Grid -->
    <line x1="60" y1="52" x2="500" y2="52" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="104" x2="500" y2="104" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="156" x2="500" y2="156" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="208" x2="500" y2="208" stroke="#e5e7eb" stroke-width="0.5"/>
    <!-- 80% power line -->
    <line x1="60" y1="72" x2="500" y2="72" stroke="#16a34a" stroke-width="1.5" stroke-dasharray="6,3" opacity="0.6"/>
    <text x="505" y="76" font-size="10" fill="#16a34a" font-weight="bold">80% power</text>
    <!-- Axes -->
    <line x1="60" y1="260" x2="500" y2="260" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="260" x2="60" y2="30" stroke="#6b7280" stroke-width="1.5"/>
    <text x="280" y="295" text-anchor="middle" font-size="12" fill="#6b7280">Sample Size per Group</text>
    <text x="15" y="145" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 15, 145)">Statistical Power</text>
    <!-- Y-axis labels -->
    <text x="52" y="264" text-anchor="end" font-size="10" fill="#6b7280">0%</text>
    <text x="52" y="212" text-anchor="end" font-size="10" fill="#6b7280">20%</text>
    <text x="52" y="160" text-anchor="end" font-size="10" fill="#6b7280">40%</text>
    <text x="52" y="108" text-anchor="end" font-size="10" fill="#6b7280">60%</text>
    <text x="52" y="56" text-anchor="end" font-size="10" fill="#6b7280">80%</text>
    <text x="52" y="34" text-anchor="end" font-size="10" fill="#6b7280">100%</text>
    <!-- X-axis labels -->
    <text x="60" y="276" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
    <text x="148" y="276" text-anchor="middle" font-size="10" fill="#6b7280">20</text>
    <text x="236" y="276" text-anchor="middle" font-size="10" fill="#6b7280">40</text>
    <text x="324" y="276" text-anchor="middle" font-size="10" fill="#6b7280">60</text>
    <text x="412" y="276" text-anchor="middle" font-size="10" fill="#6b7280">80</text>
    <text x="500" y="276" text-anchor="middle" font-size="10" fill="#6b7280">100</text>
    <!-- Curve: d=0.3 (small) — slow to reach power -->
    <path d="M 60,252 C 100,248 148,240 200,228 C 250,215 300,198 350,180 C 400,160 440,142 480,126 C 495,120 500,118 500,116" stroke="#9ca3af" stroke-width="2.5" fill="none"/>
    <text x="505" y="120" font-size="10" fill="#9ca3af" font-weight="bold">d = 0.3</text>
    <!-- Curve: d=0.5 (medium) -->
    <path d="M 60,247 C 100,235 148,210 200,178 C 250,145 300,112 340,88 C 380,72 420,58 460,48 C 480,44 500,42 500,40" stroke="#7c3aed" stroke-width="2.5" fill="none"/>
    <text x="505" y="44" font-size="10" fill="#7c3aed" font-weight="bold">d = 0.5</text>
    <!-- Curve: d=0.8 (large) — quickly reaches power -->
    <path d="M 60,238 C 90,210 120,170 148,132 C 175,98 200,72 236,52 C 260,42 300,36 340,34 C 400,32 500,32 500,32" stroke="#2563eb" stroke-width="2.5" fill="none"/>
    <text x="505" y="36" font-size="10" fill="#2563eb" font-weight="bold">d = 0.8</text>
    <!-- Markers where each curve hits 80% power -->
    <circle cx="460" cy="48" r="5" fill="#7c3aed" stroke="white" stroke-width="1.5"/>
    <text x="460" y="66" text-anchor="middle" font-size="9" fill="#7c3aed">n~64</text>
    <circle cx="185" cy="72" r="5" fill="#2563eb" stroke="white" stroke-width="1.5"/>
    <text x="185" y="66" text-anchor="middle" font-size="9" fill="#2563eb">n~26</text>
    <!-- d=0.3 doesn't reach 80% in this range -->
    <text x="440" y="145" font-size="9" fill="#9ca3af" font-style="italic">n~394 needed!</text>
  </g>
</svg>
</div>

## The Cost of Underpowered Studies

An underpowered study is not just a failed study — it's actively harmful:

1. **Waste:** Money, time, and irreplaceable biological samples consumed for inconclusive results
2. **Publication bias:** Only "lucky" underpowered studies (that happen to find p < 0.05) get published, inflating reported effect sizes
3. **False negatives:** Real treatments or biomarkers get abandoned
4. **Ethical cost:** Patients enrolled in clinical trials with no realistic chance of detecting a benefit

> **Clinical relevance:** The FDA and EMA require power analyses for all clinical trial protocols. Journal reviewers increasingly require them for observational studies too. "How many samples do you need?" is the first question of good experimental design.

## Experimental Design in BioLang

### Basic Power Analysis for t-test

```bio
set_seed(42)
# How many samples to detect a 2-fold change in gene expression?

# Parameters
let alpha = 0.05
let power_target = 0.80
let effect_size = 0.8   # Cohen's d for ~2-fold change

# Simulate power at different sample sizes
let sample_sizes = [5, 10, 15, 20, 30, 50, 75, 100]
let n_simulations = 1000

print("=== Power Analysis: Two-Sample t-test ===")
print("Effect size (Cohen's d): {effect_size}")
print("Alpha: {alpha}")
print("")
print("n per group    Estimated Power")

for n in sample_sizes {
    let significant = 0

    for sim in 0..n_simulations {
        # Simulate two groups with known effect
        let group1 = rnorm(n, 0, 1)
        let group2 = rnorm(n, effect_size, 1)

        let result = ttest(group1, group2)
        if result.p_value < alpha {
            significant = significant + 1
        }
    }

    let power = significant / n_simulations
    let marker = if power >= power_target { " <-- sufficient" } else { "" }
    print("{n}            {power |> round(3)}{marker}")
}
```

### Power Curves for Different Effect Sizes

```bio
set_seed(42)
# Visualize power as a function of sample size
let sample_sizes = [5, 10, 15, 20, 25, 30, 40, 50, 75, 100]
let effect_sizes = [0.3, 0.5, 0.8, 1.2]
let n_sims = 500

let power_curves = {}

for d in effect_sizes {
    let powers = []

    for n in sample_sizes {
        let sig_count = 0
        for s in 0..n_sims {
            let g1 = rnorm(n, 0, 1)
            let g2 = rnorm(n, d, 1)
            if ttest(g1, g2).p_value < 0.05 {
                sig_count = sig_count + 1
            }
        }
        powers = powers + [sig_count / n_sims]
    }

    power_curves["{d}"] = powers
}

# Plot power curves
let curve_table = table({
    "n": sample_sizes,
    "d_0.3": power_curves["0.3"],
    "d_0.5": power_curves["0.5"],
    "d_0.8": power_curves["0.8"],
    "d_1.2": power_curves["1.2"]
})
plot(curve_table, {type: "line", x: "n",
    title: "Power Curves: Two-Sample t-test",
    x_label: "Sample Size per Group", y_label: "Statistical Power"})
```

### RNA-seq Experiment Design

```bio
set_seed(42)
# How many biological replicates for RNA-seq DE?

# Simulate RNA-seq-like data
let fold_changes = [1.5, 2.0, 3.0, 4.0]
let replicates = [3, 5, 8, 12, 20]
let n_sims = 200

print("=== RNA-seq Power by Fold Change and Replicates ===")
print("FC       n=3     n=5     n=8    n=12    n=20")

for fc in fold_changes {
    let powers = []

    for n in replicates {
        let detected = 0

        for sim in 0..n_sims {
            # Simulate one gene with known fold change
            let control = rnorm(n, 10, 2)
            let treatment = rnorm(n, 10 * fc, 2 * fc)

            # Log-transform (as in real RNA-seq analysis)
            let log_ctrl = control |> map(|x| log2(max(x, 0.1)))
            let log_treat = treatment |> map(|x| log2(max(x, 0.1)))

            let p = ttest(log_ctrl, log_treat).p_value
            if p < 0.05 { detected = detected + 1 }
        }

        powers = powers + [detected / n_sims]
    }

    print("{fc}     " ++ powers |> map(|p| "{(p * 100) |> round(0)}%") |> join("   "))
}

# Key takeaway: n=3 barely detects 4-fold changes;
# n=8 reliably detects 2-fold changes
```

### Paired vs. Unpaired Design

```bio
set_seed(42)
# Show the power advantage of paired designs
let n_sims = 1000
let n = 20
let effect = 0.5  # medium effect
let subject_sd = 2.0  # between-subject variability
let within_sd = 0.5    # within-subject variability

let power_unpaired = 0
let power_paired = 0

for sim in 0..n_sims {
    # Unpaired: independent groups
    let group1 = rnorm(n, 0, subject_sd)
    let group2 = rnorm(n, effect, subject_sd)
    if ttest(group1, group2).p_value < 0.05 {
        power_unpaired = power_unpaired + 1
    }

    # Paired: same subjects, before and after
    let baseline = rnorm(n, 0, subject_sd)
    let after = baseline + effect + rnorm(n, 0, within_sd)
    let diff = after - baseline
    if ttest_one(diff, 0).p_value < 0.05 {
        power_paired = power_paired + 1
    }
}

print("=== Paired vs Unpaired Design (n={n}, d={effect}) ===")
print("Unpaired power: {(power_unpaired / n_sims * 100) |> round(1)}%")
print("Paired power:   {(power_paired / n_sims * 100) |> round(1)}%")
print("Pairing advantage: {((power_paired - power_unpaired) / n_sims * 100) |> round(1)} percentage points")
```

### Multi-Group Design (ANOVA)

```bio
set_seed(42)
# Power for detecting differences among 4 treatment groups
let n_sims = 500
let k = 4  # number of groups
let group_means = [0, 0.3, 0.6, 0.9]  # increasing effect
let sample_sizes = [5, 10, 15, 20, 30]

print("=== ANOVA Power (k={k} groups) ===")
for n in sample_sizes {
    let sig = 0

    for sim in 0..n_sims {
        let groups = []
        for i in 0..k {
            groups = groups + [rnorm(n, group_means[i], 1)]
        }

        let result = anova(groups)
        if result.p_value < 0.05 { sig = sig + 1 }
    }

    print("n = {n} per group: power = {(sig / n_sims * 100) |> round(1)}%")
}
```

### Sample Size Recommendation Report

```bio
set_seed(42)
# Generate a complete sample size recommendation
let scenarios = [
    { name: "Conservative (d=0.5)", effect: 0.5 },
    { name: "Expected (d=0.8)", effect: 0.8 },
    { name: "Optimistic (d=1.2)", effect: 1.2 }
]

print("=== SAMPLE SIZE RECOMMENDATION REPORT ===")
print("Two-group comparison, alpha=0.05, power=80%")
print("")

for s in scenarios {
    # Find minimum n for 80% power via simulation
    let required_n = 0
    for n in 5..200 {
        let power = 0
        for sim in 0..500 {
            let g1 = rnorm(n, 0, 1)
            let g2 = rnorm(n, s.effect, 1)
            if ttest(g1, g2).p_value < 0.05 { power = power + 1 }
        }
        if power / 500 >= 0.80 {
            required_n = n
            break
        }
    }

    print("{s.name}: n = {required_n}/group")
}

print("")
print("Recommendation: Plan for the CONSERVATIVE")
print("estimate + 10-20% for dropout/QC failures")
```

**Python:**

```python
from scipy.stats import norm
from statsmodels.stats.power import TTestIndPower, TTestPower

# Power analysis for two-sample t-test
analysis = TTestIndPower()

# Required sample size
n = analysis.solve_power(effect_size=0.8, alpha=0.05, power=0.80)
print(f"Required n per group: {n:.0f}")

# Power curve
import matplotlib.pyplot as plt
fig = analysis.plot_power(
    dep_var='nobs', nobs=range(5, 100),
    effect_size=[0.3, 0.5, 0.8, 1.2])

# Simulation-based power
import numpy as np
from scipy.stats import ttest_ind

def simulate_power(n, d, n_sim=1000):
    sig = sum(ttest_ind(np.random.normal(0, 1, n),
                        np.random.normal(d, 1, n)).pvalue < 0.05
              for _ in range(n_sim))
    return sig / n_sim
```

**R:**

```r
# Power analysis for two-sample t-test
power.t.test(d = 0.8, sig.level = 0.05, power = 0.80)

# Power curve
library(pwr)
pwr.t.test(d = 0.8, sig.level = 0.05, power = 0.80)

# RNA-seq specific power
library(RNASeqPower)
rnapower(depth = 20e6, cv = 0.4, effect = 2,
         alpha = 0.05, power = 0.8)

# Simulation-based
library(simr)
# simr provides power simulation for mixed models
```

## Exercises

### Exercise 1: Power for Your Study

You're planning a study comparing tumor mutation burden between immunotherapy responders and non-responders. Pilot data suggests d ≈ 0.6 with SD = 5 mutations/Mb. How many patients per group do you need for 80% power?

```bio

# 1. Simulate power at n = 10, 20, 30, 50, 75, 100
# 2. Find the minimum n for 80% power
# 3. Add 15% for anticipated dropout
# 4. Create a power curve plot
```

### Exercise 2: Paired vs. Unpaired

A study can either use 30 independent samples per group OR 30 paired before/after measurements. The between-subject SD is 3x the within-subject SD. Compare the power of both designs.

```bio

# 1. Simulate paired and unpaired designs with n=30
# 2. Effect size d = 0.5
# 3. Between-subject SD = 3, within-subject SD = 1
# 4. Which design achieves higher power?
# 5. How many unpaired samples would match the paired design's power?
```

### Exercise 3: RNA-seq Planning

You're designing an RNA-seq experiment to identify genes with at least 1.5-fold change between two conditions. Your budget allows either 6 samples at 30M reads each or 12 samples at 15M reads each. Which design has more power?

```bio

# Simulate both scenarios
# Track how many "true DE genes" each design detects
# Which is better: more depth or more replicates?
```

### Exercise 4: The Underpowered Literature

Simulate 100 "studies" with n=10 per group and a true small effect (d=0.3). Show that:
1. Most studies (>80%) fail to detect the effect
2. The "significant" studies dramatically overestimate the effect size
3. This creates a biased picture in the published literature

```bio

# 1. Run 100 simulated two-sample t-tests (n=10, d=0.3)
# 2. Count how many achieve p < 0.05
# 3. For the significant ones, compute Cohen's d from the data
# 4. Compare the average "published" d to the true d = 0.3
# 5. This is the "winner's curse" — published effects are inflated
```

## Key Takeaways

- **Power analysis** determines how many samples you need BEFORE starting an experiment — it's not optional, it's essential
- The four pillars are **α** (false positive rate), **power** (1-β, false negative rate), **effect size**, and **sample size** — fix three, solve for the fourth
- **Underpowered studies** waste resources, inflate published effect sizes, and can falsely suggest an effect doesn't exist
- **Biological replicates** drive power in genomics — technical replicates give diminishing returns
- For RNA-seq: n=3 is barely adequate (detects >4-fold), n=8 is good (2-fold), n=12+ is ideal (1.5-fold)
- **Paired designs** dramatically increase power by removing between-subject variability
- The **winner's curse**: underpowered studies that happen to be significant overestimate the true effect
- Always use **conservative** effect size estimates and add buffer for dropout/QC failures
- Power curves visualize the sample size / power trade-off and help identify the sweet spot

## What's Next

Statistical significance tells you whether an effect is real, but not whether it **matters**. Day 19 introduces **effect sizes** — Cohen's d, odds ratios, relative risk — and the critical distinction between statistical significance and practical importance.
