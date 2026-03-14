# Day 23: Resampling — Bootstrap and Permutation Tests

<div class="day-meta">
<span class="badge">Day 23 of 30</span>
<span class="badge">Prerequisites: Days 6-8</span>
<span class="badge">~60 min reading</span>
<span class="badge">Non-Parametric Inference</span>
</div>

## The Problem

Your collaborator is studying a rare metabolic disorder. She has tissue samples from 6 affected mice and 6 controls, measuring enzyme activity in each. The treatment group shows higher median enzyme activity, and she wants to know if the difference is real.

You reach for a t-test, but hesitate. With only 6 observations per group, you cannot meaningfully assess whether the data is normally distributed. A Shapiro-Wilk test on 6 points has almost no power. The Wilcoxon rank-sum test is an option, but with only 6 per group, it can only detect very large effects.

What if you could test the hypothesis without assuming any distribution at all? What if you could build a confidence interval for any statistic — not just the mean, but the median, the trimmed mean, the ratio of two variances, the 90th percentile — without knowing the population distribution?

You can. Resampling methods let the data speak for itself, replacing theoretical assumptions with raw computational power. They are among the most broadly applicable tools in statistics, and after today, you will wonder how you ever lived without them.

## What Are Resampling Methods?

Resampling methods draw repeated samples from your data to estimate the sampling distribution of a statistic. Instead of deriving a formula for how the mean varies from sample to sample (the classical approach), you literally simulate the process: draw a new sample, compute the statistic, repeat thousands of times, and look at the distribution of results.

Think of it like this. You have a bag containing 12 marbles (your data). You want to know how variable the "average marble weight" is. The classical approach derives a formula based on the normal distribution. The resampling approach says: shake the bag, pull out 12 marbles (with replacement), weigh them, compute the average. Put them back. Repeat 10,000 times. The distribution of those 10,000 averages tells you everything you need to know — no formula, no assumptions.

> **Key insight:** Resampling methods trade mathematical assumptions for computational effort. With modern computers, 10,000 resamples takes milliseconds. The only assumption is that your sample is representative of the population — the same assumption underlying all of statistics.

## The Bootstrap

The bootstrap, invented by Bradley Efron in 1979, is one of the most important ideas in modern statistics. It estimates the sampling distribution of any statistic by resampling with replacement from your data.

### How It Works

1. You have a sample of n observations.
2. Draw a new sample of n observations **with replacement** from your original data. Some observations will appear multiple times; some will not appear at all.
3. Compute your statistic of interest (mean, median, standard deviation, correlation, whatever) on this bootstrap sample.
4. Repeat steps 2-3 many times (typically 10,000).
5. The distribution of the computed statistics is the **bootstrap distribution**. It approximates the sampling distribution of your statistic.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="360" viewBox="0 0 680 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Bootstrap Resampling: How It Works</text>
  <!-- Original dataset -->
  <rect x="20" y="50" width="150" height="130" rx="6" fill="white" stroke="#2563eb" stroke-width="1.5"/>
  <text x="95" y="70" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Original Data (n=8)</text>
  <g font-size="14" fill="#1e293b" text-anchor="middle">
    <text x="50" y="95">3.8</text><text x="95" y="95">4.2</text><text x="140" y="95">4.9</text>
    <text x="50" y="118">5.1</text><text x="95" y="118">5.5</text><text x="140" y="118">5.8</text>
    <text x="50" y="141">6.3</text><text x="95" y="141">4.7</text>
  </g>
  <text x="95" y="172" text-anchor="middle" font-size="11" fill="#6b7280">Sample with replacement</text>

  <!-- Arrow -->
  <text x="192" y="115" font-size="24" fill="#9ca3af">&rarr;</text>

  <!-- Bootstrap samples -->
  <rect x="215" y="45" width="130" height="75" rx="5" fill="white" stroke="#3b82f6"/>
  <text x="280" y="62" text-anchor="middle" font-size="10" font-weight="bold" fill="#3b82f6">Bootstrap #1</text>
  <text x="280" y="80" text-anchor="middle" font-size="11" fill="#1e293b">4.2 5.1 4.2 6.3</text>
  <text x="280" y="96" text-anchor="middle" font-size="11" fill="#1e293b">5.5 3.8 5.1 4.9</text>
  <text x="280" y="112" text-anchor="middle" font-size="10" fill="#7c3aed">median = 5.0</text>

  <rect x="215" y="128" width="130" height="75" rx="5" fill="white" stroke="#3b82f6"/>
  <text x="280" y="145" text-anchor="middle" font-size="10" font-weight="bold" fill="#3b82f6">Bootstrap #2</text>
  <text x="280" y="163" text-anchor="middle" font-size="11" fill="#1e293b">5.8 5.8 3.8 4.7</text>
  <text x="280" y="179" text-anchor="middle" font-size="11" fill="#1e293b">6.3 4.9 5.5 6.3</text>
  <text x="280" y="195" text-anchor="middle" font-size="10" fill="#7c3aed">median = 5.35</text>

  <rect x="215" y="211" width="130" height="75" rx="5" fill="white" stroke="#3b82f6"/>
  <text x="280" y="228" text-anchor="middle" font-size="10" font-weight="bold" fill="#3b82f6">Bootstrap #10000</text>
  <text x="280" y="246" text-anchor="middle" font-size="11" fill="#1e293b">4.7 5.1 4.2 5.5</text>
  <text x="280" y="262" text-anchor="middle" font-size="11" fill="#1e293b">4.2 3.8 5.8 4.9</text>
  <text x="280" y="278" text-anchor="middle" font-size="10" fill="#7c3aed">median = 4.8</text>

  <!-- Dots between samples -->
  <text x="280" y="299" text-anchor="middle" font-size="14" fill="#9ca3af">...</text>

  <!-- Arrow to distribution -->
  <text x="365" y="165" font-size="24" fill="#9ca3af">&rarr;</text>

  <!-- Bootstrap distribution -->
  <rect x="395" y="50" width="270" height="260" rx="6" fill="white" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="530" y="70" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">Bootstrap Distribution</text>
  <!-- Histogram bars approximating a bell shape -->
  <rect x="420" y="230" width="18" height="15" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="440" y="215" width="18" height="30" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="460" y="190" width="18" height="55" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="480" y="155" width="18" height="90" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="500" y="120" width="18" height="125" rx="1" fill="#3b82f6" opacity="0.7"/>
  <rect x="520" y="105" width="18" height="140" rx="1" fill="#3b82f6" opacity="0.7"/>
  <rect x="540" y="115" width="18" height="130" rx="1" fill="#3b82f6" opacity="0.7"/>
  <rect x="560" y="150" width="18" height="95" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="580" y="185" width="18" height="60" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="600" y="210" width="18" height="35" rx="1" fill="#93c5fd" opacity="0.7"/>
  <rect x="620" y="228" width="18" height="17" rx="1" fill="#93c5fd" opacity="0.7"/>
  <!-- Axis -->
  <line x1="415" y1="245" x2="645" y2="245" stroke="#9ca3af" stroke-width="1"/>
  <text x="530" y="262" text-anchor="middle" font-size="11" fill="#6b7280">Bootstrap Median Values</text>
  <!-- CI markers -->
  <line x1="448" y1="248" x2="448" y2="280" stroke="#dc2626" stroke-width="2"/>
  <line x1="610" y1="248" x2="610" y2="280" stroke="#dc2626" stroke-width="2"/>
  <line x1="448" y1="275" x2="610" y2="275" stroke="#dc2626" stroke-width="1.5"/>
  <text x="529" y="292" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">95% Confidence Interval</text>
  <text x="448" y="298" text-anchor="middle" font-size="10" fill="#dc2626">2.5th %ile</text>
  <text x="610" y="298" text-anchor="middle" font-size="10" fill="#dc2626">97.5th %ile</text>
  <!-- Note about repeated values -->
  <rect x="15" y="325" width="650" height="25" rx="4" fill="#f1f5f9"/>
  <text x="340" y="342" text-anchor="middle" font-size="11" fill="#6b7280">Note: some values repeat (4.2 appears twice in #1) and some are missing -- this is sampling with replacement</text>
</svg>
</div>

### Bootstrap Confidence Intervals

The simplest bootstrap CI uses percentiles. For a 95% CI, take the 2.5th and 97.5th percentiles of the bootstrap distribution.

```bio
set_seed(42)
# Enzyme activity in 6 treatment mice
let treatment = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5]

# Bootstrap 95% CI for the median
let n_boot = 10000
let boot_medians = []
for i in 0..n_boot {
  let resample = seq(1, length(treatment))
    |> map(|_| treatment[random_int(0, length(treatment) - 1)])
  boot_medians = boot_medians + [median(resample)]
}
let ci_lower = quantile(boot_medians, 0.025)
let ci_upper = quantile(boot_medians, 0.975)

print("Observed median: " + str(median(treatment)))
print("Bootstrap 95% CI: [" +
  str(round(ci_lower, 2)) + ", " +
  str(round(ci_upper, 2)) + "]")

# Visualize the bootstrap distribution
histogram(boot_medians, {bins: 50,
  title: "Bootstrap Distribution of Median Enzyme Activity"})
```

### Why Replacement Matters

Drawing with replacement is what makes the bootstrap work. Each bootstrap sample is a plausible alternative dataset — a dataset you might have obtained if you had re-run the experiment. By computing your statistic on thousands of these alternative datasets, you learn how much the statistic would vary from experiment to experiment.

Without replacement, you would just get your original data back (in a different order), and the statistic would never change.

### Bootstrap for Any Statistic

The beauty of the bootstrap is its universality. Classical formulas for confidence intervals exist for means, proportions, and a few other well-behaved statistics. But what about the median? The inter-quartile range? The ratio of two means? The coefficient of variation? The 95th percentile? For most of these, no clean formula exists. The bootstrap handles them all identically.

| Statistic | Classical CI formula? | Bootstrap works? |
|---|---|---|
| Mean | Yes (t-interval) | Yes |
| Median | Complicated, approximate | Yes |
| Standard deviation | Approximate (chi-square) | Yes |
| Correlation | Fisher z-transform | Yes |
| Ratio of means | No simple formula | Yes |
| 90th percentile | No simple formula | Yes |
| Difference in medians | No simple formula | Yes |
| Any custom function | Almost never | Yes |

```bio
set_seed(42)
# Bootstrap CI for the coefficient of variation
let data = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5, 4.7, 5.8]

let n_boot = 10000
let boot_cvs = []
for i in 0..n_boot {
  let resample = seq(1, length(data))
    |> map(|_| data[random_int(0, length(data) - 1)])
  boot_cvs = boot_cvs + [stdev(resample) / mean(resample)]
}

print("CV: " + str(round(stdev(data) / mean(data), 3)))
print("95% CI: [" + str(round(quantile(boot_cvs, 0.025), 3)) + ", " +
  str(round(quantile(boot_cvs, 0.975), 3)) + "]")
```

### Bootstrap Hypothesis Testing

You can also use the bootstrap for hypothesis testing. To test whether the mean of a population is equal to some value mu_0:

1. Shift your data so that its mean equals mu_0 (subtract the difference).
2. Bootstrap from the shifted data.
3. Compute the statistic on each bootstrap sample.
4. The p-value is the proportion of bootstrap statistics as extreme as or more extreme than your observed statistic.

```bio
set_seed(42)
# Bootstrap test: is the mean enzyme activity > 4.0?
let observed_mean = mean(treatment)
let shifted = treatment |> map(|x| x - (observed_mean - 4.0))

let boot_means = []
for i in 0..10000 {
  let resample = seq(1, length(shifted))
    |> map(|_| shifted[random_int(0, length(shifted) - 1)])
  boot_means = boot_means + [mean(resample)]
}
let p_value = boot_means
  |> filter(|x| x >= observed_mean)
  |> length() / 10000

print("Observed mean: " + str(round(observed_mean, 2)))
print("Bootstrap p-value (H0: mu = 4.0): " + str(round(p_value, 4)))
```

> **Common pitfall:** The bootstrap is not magic. It cannot create information that is not in your data. With only 6 observations, the bootstrap distribution is limited to combinations of those 6 values. The bootstrap CI will be approximate, and it underestimates uncertainty when n is very small (say, n < 10). It works best when n is moderate to large.

## The Permutation Test

The permutation test directly addresses the question: "Could the observed difference between groups have arisen by chance?" It does so by exhaustively (or approximately) considering all possible ways to assign labels to the data.

### How It Works

1. Compute the test statistic (e.g., difference in means) for the observed data.
2. **Shuffle** the group labels randomly, keeping the data values fixed.
3. Recompute the test statistic on the shuffled data.
4. Repeat steps 2-3 many times (10,000 or more).
5. The p-value is the proportion of shuffled statistics as extreme as or more extreme than the observed one.

The logic is simple: if group membership does not matter (the null hypothesis), then shuffling labels should produce similar statistics. If the observed statistic is far from what shuffling produces, the group difference is real.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Permutation Test: Shuffling Labels Under the Null</text>
  <!-- Original data -->
  <rect x="20" y="48" width="190" height="120" rx="6" fill="white" stroke="#2563eb" stroke-width="1.5"/>
  <text x="115" y="68" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">Original Data</text>
  <text x="65" y="88" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">Treatment</text>
  <text x="160" y="88" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">Control</text>
  <g font-size="12" fill="#1e293b">
    <text x="65" y="108" text-anchor="middle">4.2, 5.1</text>
    <text x="65" y="124" text-anchor="middle">3.8, 6.3</text>
    <text x="65" y="140" text-anchor="middle">4.9, 5.5</text>
    <text x="160" y="108" text-anchor="middle">3.1, 2.8</text>
    <text x="160" y="124" text-anchor="middle">3.5, 3.2</text>
    <text x="160" y="140" text-anchor="middle">2.9, 3.6</text>
  </g>
  <text x="115" y="162" text-anchor="middle" font-size="11" fill="#7c3aed" font-weight="bold">diff = 1.80</text>

  <!-- Shuffle arrow -->
  <g transform="translate(225, 85)">
    <text x="20" y="15" font-size="11" fill="#6b7280">Shuffle</text>
    <text x="20" y="30" font-size="11" fill="#6b7280">labels</text>
    <text x="20" y="50" font-size="22" fill="#9ca3af">&rarr;</text>
  </g>

  <!-- Shuffled example -->
  <rect x="280" y="48" width="190" height="120" rx="6" fill="white" stroke="#9ca3af" stroke-width="1.5"/>
  <text x="375" y="68" text-anchor="middle" font-size="12" font-weight="bold" fill="#9ca3af">Shuffled Labels</text>
  <text x="325" y="88" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">"Treatment"</text>
  <text x="425" y="88" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">"Control"</text>
  <g font-size="12" fill="#1e293b">
    <text x="325" y="108" text-anchor="middle">3.1, 5.1</text>
    <text x="325" y="124" text-anchor="middle">4.9, 3.2</text>
    <text x="325" y="140" text-anchor="middle">6.3, 2.8</text>
    <text x="425" y="108" text-anchor="middle">4.2, 3.5</text>
    <text x="425" y="124" text-anchor="middle">5.5, 2.9</text>
    <text x="425" y="140" text-anchor="middle">3.8, 3.6</text>
  </g>
  <text x="375" y="162" text-anchor="middle" font-size="11" fill="#9ca3af">diff = 0.37</text>

  <!-- Repeat arrow -->
  <text x="485" y="110" font-size="14" fill="#9ca3af">Repeat</text>
  <text x="485" y="128" font-size="14" fill="#9ca3af">10,000x</text>
  <text x="510" y="148" font-size="22" fill="#9ca3af">&rarr;</text>

  <!-- Null distribution -->
  <rect x="540" y="48" width="128" height="130" rx="6" fill="white" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="604" y="68" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">Null Distribution</text>
  <!-- Mini histogram -->
  <rect x="555" y="135" width="8" height="10" fill="#93c5fd" opacity="0.6"/>
  <rect x="565" y="125" width="8" height="20" fill="#93c5fd" opacity="0.6"/>
  <rect x="575" y="105" width="8" height="40" fill="#93c5fd" opacity="0.6"/>
  <rect x="585" y="85" width="8" height="60" fill="#93c5fd" opacity="0.6"/>
  <rect x="595" y="80" width="8" height="65" fill="#93c5fd" opacity="0.6"/>
  <rect x="605" y="88" width="8" height="57" fill="#93c5fd" opacity="0.6"/>
  <rect x="615" y="108" width="8" height="37" fill="#93c5fd" opacity="0.6"/>
  <rect x="625" y="122" width="8" height="23" fill="#93c5fd" opacity="0.6"/>
  <rect x="635" y="132" width="8" height="13" fill="#93c5fd" opacity="0.6"/>
  <line x1="553" y1="145" x2="648" y2="145" stroke="#9ca3af" stroke-width="1"/>
  <!-- Observed mark -->
  <line x1="650" y1="75" x2="650" y2="150" stroke="#dc2626" stroke-width="2.5"/>
  <text x="650" y="166" text-anchor="middle" font-size="9" font-weight="bold" fill="#dc2626">Observed</text>
  <text x="650" y="176" text-anchor="middle" font-size="9" fill="#dc2626">(1.80)</text>

  <!-- Bottom explanation: p-value -->
  <rect x="20" y="195" width="645" height="45" rx="6" fill="#f0f9ff" stroke="#93c5fd"/>
  <text x="340" y="215" text-anchor="middle" font-size="12" fill="#1e293b">
    <tspan font-weight="bold">p-value</tspan> = fraction of shuffled differences as extreme as observed = (count where |diff| &gt;= 1.80) / 10,000</text>
  <text x="340" y="233" text-anchor="middle" font-size="11" fill="#6b7280">If very few shuffled values are this extreme, the group difference is unlikely due to chance.</text>

  <!-- K-fold CV section -->
  <text x="340" y="270" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">5-Fold Cross-Validation</text>
  <!-- Fold visualization: 5 rows, each with 5 segments, one highlighted as test -->
  <g transform="translate(90, 280)">
    <!-- Row labels -->
    <text x="-10" y="13" text-anchor="end" font-size="10" fill="#6b7280">Fold 1</text>
    <text x="-10" y="30" text-anchor="end" font-size="10" fill="#6b7280">Fold 2</text>
    <text x="-10" y="47" text-anchor="end" font-size="10" fill="#6b7280">Fold 3</text>
    <text x="-10" y="64" text-anchor="end" font-size="10" fill="#6b7280">Fold 4</text>
    <text x="-10" y="81" text-anchor="end" font-size="10" fill="#6b7280">Fold 5</text>
    <!-- Fold 1 -->
    <rect x="0" y="2" width="95" height="14" rx="2" fill="#dc2626" opacity="0.7"/>
    <rect x="98" y="2" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="196" y="2" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="294" y="2" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="392" y="2" width="95" height="14" rx="2" fill="#93c5fd"/>
    <!-- Fold 2 -->
    <rect x="0" y="19" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="98" y="19" width="95" height="14" rx="2" fill="#dc2626" opacity="0.7"/>
    <rect x="196" y="19" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="294" y="19" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="392" y="19" width="95" height="14" rx="2" fill="#93c5fd"/>
    <!-- Fold 3 -->
    <rect x="0" y="36" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="98" y="36" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="196" y="36" width="95" height="14" rx="2" fill="#dc2626" opacity="0.7"/>
    <rect x="294" y="36" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="392" y="36" width="95" height="14" rx="2" fill="#93c5fd"/>
    <!-- Fold 4 -->
    <rect x="0" y="53" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="98" y="53" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="196" y="53" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="294" y="53" width="95" height="14" rx="2" fill="#dc2626" opacity="0.7"/>
    <rect x="392" y="53" width="95" height="14" rx="2" fill="#93c5fd"/>
    <!-- Fold 5 -->
    <rect x="0" y="70" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="98" y="70" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="196" y="70" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="294" y="70" width="95" height="14" rx="2" fill="#93c5fd"/>
    <rect x="392" y="70" width="95" height="14" rx="2" fill="#dc2626" opacity="0.7"/>
    <!-- Legend -->
    <rect x="500" y="25" width="12" height="12" rx="2" fill="#dc2626" opacity="0.7"/>
    <text x="518" y="35" font-size="10" fill="#6b7280">Test set</text>
    <rect x="500" y="45" width="12" height="12" rx="2" fill="#93c5fd"/>
    <text x="518" y="55" font-size="10" fill="#6b7280">Training set</text>
  </g>
</svg>
</div>

```bio
set_seed(42)
# Treatment vs control enzyme activity
let treatment = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5]
let control = [3.1, 2.8, 3.5, 3.2, 2.9, 3.6]

# Permutation test for difference in means
let observed_diff = mean(treatment) - mean(control)
let combined = treatment + control
let n_perm = 10000
let null_diffs = []

for i in 0..n_perm {
  let shuffled = shuffle(combined)
  let perm_diff = mean(shuffled |> take(6)) - mean(shuffled |> drop(6))
  null_diffs = null_diffs + [perm_diff]
}

let p_value = null_diffs
  |> filter(|d| abs(d) >= abs(observed_diff))
  |> length() / n_perm

print("Observed difference: " + str(round(observed_diff, 3)))
print("Permutation p-value: " + str(round(p_value, 4)))

# Visualize the null distribution
histogram(null_diffs, {bins: 50,
  title: "Permutation Null Distribution"})
```

### Permutation vs Bootstrap

| Feature | Bootstrap | Permutation Test |
|---|---|---|
| Primary use | Confidence intervals | Hypothesis testing |
| Resamples from | One group (with replacement) | Combined data (without replacement, shuffling labels) |
| Tests | Any statistic; H0: parameter = value | Two-group (or multi-group) comparisons |
| Null distribution | Centered at observed statistic | Centered at zero difference |
| Assumptions | Sample is representative | Exchangeability under H0 |

> **Key insight:** Use the bootstrap when you want a confidence interval. Use the permutation test when you want a p-value comparing groups. They are complementary, not competing methods.

### Exact Permutation Tests

With small samples, you can enumerate all possible permutations. For 6 treatment and 6 control observations, there are C(12, 6) = 924 possible label assignments. You can compute the test statistic for every one and get an exact p-value.

```bio
# Exact permutation test (small sample)
# With 6+6 = 12 observations, there are C(12,6) = 924 permutations
# We can enumerate all of them for an exact p-value
# (In practice, use the randomized version above for larger datasets)
print("Exact permutation: enumerate all 924 label assignments")
print("This gives exact p-values for small samples")
```

## Cross-Validation

Cross-validation is a resampling method for evaluating predictive models. Instead of testing a model on the same data it was trained on (which overestimates performance), you repeatedly hold out a portion of the data for testing.

### K-Fold Cross-Validation

1. Divide the data into k equally-sized folds.
2. For each fold: train the model on the other k-1 folds, predict on the held-out fold, record the error.
3. Average the errors across all k folds.

Common choices: k = 5 or k = 10. Leave-one-out cross-validation (LOOCV) uses k = n.

```bio
set_seed(42)
# Generate data with a true linear relationship
let x = seq(1, 100) |> take(50)
let y = x |> map(|xi| 2.5 * xi + 10 + rnorm(1, 0, 15)[0])

# 5-fold cross-validation for linear regression
let k = 5
let fold_size = length(x) / k
let fold_errors = []

for f in 0..k {
  let test_idx = seq(f * fold_size, (f + 1) * fold_size - 1)
  let train_x = []
  let train_y = []
  let test_x = []
  let test_y = []
  for i in 0..length(x) {
    if i >= f * fold_size && i < (f + 1) * fold_size {
      test_x = test_x + [x[i]]
      test_y = test_y + [y[i]]
    } else {
      train_x = train_x + [x[i]]
      train_y = train_y + [y[i]]
    }
  }
  let model = lm(train_y, train_x)
  let preds = test_x |> map(|xi| model.intercept + model.slope * xi)
  let mse = mean(zip(preds, test_y) |> map(|p| (p[0] - p[1]) * (p[0] - p[1])))
  fold_errors = fold_errors + [mse]
}

print("Mean squared error per fold: " + str(fold_errors))
print("Average MSE: " + str(round(mean(fold_errors), 2)))
print("Standard deviation of MSE: " + str(round(stdev(fold_errors), 2)))
```

### Why Cross-Validation Matters in Genomics

In genomics, overfitting is pervasive. A model trained on 20,000 gene expression features and 100 samples can easily memorize the training data while learning nothing generalizable. Cross-validation reveals this: if training accuracy is 99% but cross-validated accuracy is 52%, the model is memorizing, not learning.

> **Clinical relevance:** Gene expression classifiers for cancer prognosis (like MammaPrint or Oncotype DX) must be validated on independent cohorts. Cross-validation provides an estimate of out-of-sample performance during development, but ultimate validation requires truly independent datasets.

## The Jackknife

The jackknife (predating the bootstrap by two decades) is a leave-one-out resampling method. It computes the statistic n times, each time leaving out one observation. The variation among these n estimates quantifies uncertainty.

```bio
# Jackknife estimate of standard error for the median
let data = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5, 4.7, 5.8]
let n = length(data)
let full_stat = median(data)

let jk_estimates = []
for i in 0..n {
  let subset = data |> filter_index(|j, _| j != i)
  jk_estimates = jk_estimates + [median(subset)]
}

let jk_mean = mean(jk_estimates)
let jk_bias = (n - 1) * (jk_mean - full_stat)
let jk_se = sqrt((n - 1) / n * sum(jk_estimates |> map(|x| (x - jk_mean) * (x - jk_mean))))

print("Jackknife estimate: " + str(round(full_stat, 3)))
print("Jackknife SE: " + str(round(jk_se, 3)))
print("Jackknife bias: " + str(round(jk_bias, 4)))
```

The jackknife is less versatile than the bootstrap but useful for bias estimation and influence analysis (which observation has the most impact on the statistic?).

| Method | Resamples | With replacement | Primary use |
|---|---|---|---|
| Bootstrap | 10,000+ random | Yes | CIs for any statistic |
| Permutation | 10,000+ shuffled | No (labels shuffled) | Hypothesis testing |
| Cross-validation | k folds | No (systematic split) | Model evaluation |
| Jackknife | n leave-one-out | No | Bias, SE, influence |

## When to Use Resampling

Resampling methods are appropriate when:

- **Sample size is small** and normality cannot be verified
- **The statistic has no known distribution** (median, ratio, custom function)
- **You want to avoid distributional assumptions** entirely
- **Classical methods are unavailable** for your specific analysis
- **You want a second opinion** — bootstrap CIs should roughly agree with parametric CIs when assumptions hold

Resampling methods are less appropriate when:

- **The sample is very small** (n < 5): too few unique bootstrap samples
- **The data has complex structure** (time series, spatial): naive resampling breaks the structure
- **Computational cost matters**: millions of bootstraps on large datasets can be slow
- **A well-validated parametric method exists** and assumptions are met: the parametric method will be more efficient (tighter CIs for the same sample size)

## Resampling in BioLang — Complete Pipeline

```bio
set_seed(42)
# Mouse experiment: enzyme activity
let treatment = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5]
let control = [3.1, 2.8, 3.5, 3.2, 2.9, 3.6]

# Helper: bootstrap a statistic from a list
fn boot_ci(data, stat_fn, n_boot) {
  let samples = []
  for i in 0..n_boot {
    let resample = seq(1, length(data))
      |> map(|_| data[random_int(0, length(data) - 1)])
    samples = samples + [stat_fn(resample)]
  }
  {lower: quantile(samples, 0.025), upper: quantile(samples, 0.975), dist: samples}
}

# ============================================
# 1. Bootstrap CI for median in each group
# ============================================
let boot_treat = boot_ci(treatment, median, 10000)
let boot_ctrl = boot_ci(control, median, 10000)

print("=== Bootstrap CIs for Median ===")
print("Treatment: " + str(round(median(treatment), 2)) +
  " [" + str(round(boot_treat.lower, 2)) + ", " +
  str(round(boot_treat.upper, 2)) + "]")
print("Control: " + str(round(median(control), 2)) +
  " [" + str(round(boot_ctrl.lower, 2)) + ", " +
  str(round(boot_ctrl.upper, 2)) + "]")

# ============================================
# 2. Bootstrap CI for difference in medians
# ============================================
let boot_diffs = []
for i in 0..10000 {
  let res_t = seq(1, 6) |> map(|_| treatment[random_int(0, 5)])
  let res_c = seq(1, 6) |> map(|_| control[random_int(0, 5)])
  boot_diffs = boot_diffs + [median(res_t) - median(res_c)]
}

print("\n=== Bootstrap CI for Median Difference ===")
print("Difference: " + str(round(median(treatment) - median(control), 2)))
print("95% CI: [" + str(round(quantile(boot_diffs, 0.025), 2)) + ", " +
  str(round(quantile(boot_diffs, 0.975), 2)) + "]")

# ============================================
# 3. Permutation test
# ============================================
let obs_diff = mean(treatment) - mean(control)
let combined = treatment + control
let null_diffs = []
for i in 0..10000 {
  let s = shuffle(combined)
  null_diffs = null_diffs + [mean(s |> take(6)) - mean(s |> drop(6))]
}
let perm_p = null_diffs |> filter(|d| abs(d) >= abs(obs_diff)) |> length() / 10000

print("\n=== Permutation Test ===")
print("Observed mean diff: " + str(round(obs_diff, 3)))
print("Permutation p-value: " + str(round(perm_p, 4)))

# Compare to Welch t-test
let tt = ttest(treatment, control)
print("Welch t-test p-value: " + str(round(tt.p_value, 4)))

# Visualize the null distribution
histogram(null_diffs, {bins: 50,
  title: "Permutation Null vs Observed"})

# ============================================
# 4. Cross-validate a regression model
# ============================================
let gene_expr = seq(1, 100) |> take(40)
let drug_response = gene_expr |> map(|x| -0.5 * x + 80 + rnorm(1, 0, 10)[0])

let k = 5
let fold_size = length(gene_expr) / k
let fold_errors = []
for f in 0..k {
  let train_x = []
  let train_y = []
  let test_x = []
  let test_y = []
  for j in 0..length(gene_expr) {
    if j >= f * fold_size && j < (f + 1) * fold_size {
      test_x = test_x + [gene_expr[j]]
      test_y = test_y + [drug_response[j]]
    } else {
      train_x = train_x + [gene_expr[j]]
      train_y = train_y + [drug_response[j]]
    }
  }
  let model = lm(train_y, train_x)
  let preds = test_x |> map(|xi| model.intercept + model.slope * xi)
  let mse = mean(zip(preds, test_y) |> map(|p| (p[0] - p[1]) * (p[0] - p[1])))
  fold_errors = fold_errors + [mse]
}

print("\n=== Cross-Validation ===")
print("5-fold CV MSE: " + str(round(mean(fold_errors), 2)) +
  " +/- " + str(round(stdev(fold_errors), 2)))

# ============================================
# 5. Compare bootstrap vs parametric CI
# ============================================
let boot_means = boot_ci(treatment, mean, 10000)
let t_ci = ttest_one(treatment, 0)

print("\n=== Bootstrap vs Parametric CI for Mean ===")
print("Bootstrap 95% CI: [" + str(round(boot_means.lower, 2)) +
  ", " + str(round(boot_means.upper, 2)) + "]")
print("t-based 95% CI:   [" + str(round(t_ci.ci_lower, 2)) +
  ", " + str(round(t_ci.ci_upper, 2)) + "]")

# ============================================
# 6. Jackknife for influence detection
# ============================================
let n = length(treatment)
let jk_estimates = []
for i in 0..n {
  let subset = treatment |> filter_index(|j, _| j != i)
  jk_estimates = jk_estimates + [mean(subset)]
}
print("\n=== Jackknife ===")
print("Leave-one-out estimates:")
for i in 0..n {
  print("  Without obs " + str(i+1) + ": " +
    str(round(jk_estimates[i], 3)))
}
let max_diff = 0
let max_idx = 0
for i in 0..n {
  let d = abs(jk_estimates[i] - mean(treatment))
  if d > max_diff { max_diff = d; max_idx = i }
}
print("Most influential observation: " + str(max_idx + 1))
```

**Python:**

```python
import numpy as np
from scipy import stats
from sklearn.model_selection import cross_val_score
from sklearn.linear_model import LinearRegression
from sklearn.utils import resample

# Bootstrap CI for median
np.random.seed(42)
treatment = np.array([4.2, 5.1, 3.8, 6.3, 4.9, 5.5])
boot_medians = [np.median(resample(treatment)) for _ in range(10000)]
ci = np.percentile(boot_medians, [2.5, 97.5])
print(f"Bootstrap 95% CI for median: [{ci[0]:.2f}, {ci[1]:.2f}]")

# Permutation test
control = np.array([3.1, 2.8, 3.5, 3.2, 2.9, 3.6])
observed = treatment.mean() - control.mean()
combined = np.concatenate([treatment, control])
null_dist = []
for _ in range(10000):
    np.random.shuffle(combined)
    null_dist.append(combined[:6].mean() - combined[6:].mean())
p_value = np.mean(np.abs(null_dist) >= np.abs(observed))

# Cross-validation
from sklearn.model_selection import cross_val_score
model = LinearRegression()
scores = cross_val_score(model, X.reshape(-1,1), y, cv=5, scoring='neg_mean_squared_error')
```

**R:**

```r
# Bootstrap CI for median
library(boot)
med_fn <- function(data, i) median(data[i])
b <- boot(treatment, med_fn, R = 10000)
boot.ci(b, type = "perc")

# Permutation test
library(coin)
wilcox_test(values ~ group, data = df, distribution = "exact")

# Or manual permutation
observed <- mean(treatment) - mean(control)
combined <- c(treatment, control)
null_dist <- replicate(10000, {
  perm <- sample(combined)
  mean(perm[1:6]) - mean(perm[7:12])
})
p_value <- mean(abs(null_dist) >= abs(observed))

# Cross-validation
library(caret)
cv <- train(y ~ x, data = df, method = "lm",
            trControl = trainControl(method = "cv", number = 5))
```

## Exercises

1. **Bootstrap the correlation.** Given paired measurements from 15 patients (gene expression and drug response), compute the Pearson correlation and a bootstrap 95% CI. Is the correlation significantly different from zero?

```bio
let expr = [2.1, 4.5, 3.2, 5.8, 1.9, 6.2, 3.8, 4.1, 5.5, 2.8, 3.5, 6.0, 4.3, 2.5, 5.1]
let response = [35, 62, 41, 78, 30, 82, 55, 50, 71, 38, 48, 79, 58, 33, 68]
# Your code: bootstrap CI for cor(expr, response)
```

2. **Permutation test for median.** Using the treatment and control data from this chapter, run a permutation test using the difference in medians (not means) as the test statistic. Compare the p-value to the mean-based permutation test.

```bio
# Your code: permutation_test with statistic: "median_diff"
```

3. **Bootstrap vs t-test at n=6.** Generate 1,000 datasets of n=6 from a skewed distribution (e.g., exponential). For each, compute both a t-based CI and a bootstrap CI for the mean. Compare coverage rates (how often the true mean falls within the CI). Which method is more reliable?

```bio
# Your code: simulation study comparing CI coverage
```

4. **Cross-validation showdown.** Fit a linear regression predicting drug response from gene expression. Compare 5-fold, 10-fold, and leave-one-out cross-validation. Do they agree on the model's prediction error?

```bio
# Your code: three CV approaches, compare MSE estimates
```

5. **Jackknife influence.** Compute the jackknife influence values for the mean of the treatment data. Which observation, when removed, changes the mean the most? Does this make sense given the data?

```bio
let treatment = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5]
# Your code: jackknife, identify most influential observation
```

## Key Takeaways

- The bootstrap estimates the sampling distribution of any statistic by resampling with replacement — no distributional assumptions needed.
- Bootstrap confidence intervals use percentiles of the bootstrap distribution; they work for means, medians, ratios, correlations, or any custom function.
- Permutation tests build a null distribution by shuffling group labels, providing an exact or approximate p-value for group comparisons without distributional assumptions.
- Cross-validation evaluates predictive models honestly by holding out data, preventing the overfitting that plagues high-dimensional genomics.
- The jackknife identifies influential observations and estimates bias.
- Resampling methods complement, rather than replace, parametric methods — when assumptions are met, parametric methods are more efficient; when they are not, resampling provides a robust alternative.
- With very small samples (n < 5-10), even resampling methods have limited resolution — there are simply too few unique resamples.

## What's Next

Tomorrow we cross the philosophical divide in statistics. Everything so far has been frequentist: probabilities describe long-run frequencies of events. Bayesian statistics takes a fundamentally different view — probability describes degrees of belief, and we update those beliefs as evidence accumulates. You will learn to combine prior knowledge with observed data using Bayes' theorem, build posterior distributions, and discover why Bayesian thinking is particularly natural for interpreting variants of uncertain significance.
