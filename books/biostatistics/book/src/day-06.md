# Day 6: Confidence Intervals — The Range of Truth

## The Problem

Dr. Amara Chen's pharmacology team has spent six months developing a novel kinase inhibitor for triple-negative breast cancer. After extensive optimization, they measure the half-maximal inhibitory concentration (IC50) across eight independent replicates: 11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, and 12.5 nanomolar. The mean is 12.3 nM — an excellent result that would place their compound among the most potent in its class.

But when Dr. Chen presents these results to the medicinal chemistry team, the lead chemist asks the uncomfortable question: "If you ran the experiment again tomorrow, would you get 12.3 nM? Or could it be 15? Or 9?" The point estimate of 12.3 nM tells them where the center of their data is, but it says nothing about how *confident* they should be in that number. They need a range — a confidence interval — that captures the uncertainty inherent in measuring anything biological.

This chapter introduces the confidence interval: a range of plausible values for a population parameter, built from sample data. It is one of the most important and most misunderstood tools in all of biostatistics.

## What Is a Confidence Interval?

Imagine you are trying to measure the height of a building, but your measuring tape is slightly stretchy. Each time you measure, you get a slightly different answer. A confidence interval is like saying: "Based on my eight measurements, I am 95% confident the true height is somewhere between 48.2 and 52.1 meters."

More precisely: if you repeated your experiment 100 times and computed a 95% confidence interval each time, about 95 of those 100 intervals would contain the true population parameter. The remaining 5 would miss it entirely.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="520" viewBox="0 0 680 520" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="30" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">20 Confidence Intervals from Repeated Experiments</text>
  <text x="340" y="48" text-anchor="middle" font-size="12" fill="#6b7280">~19 capture the true parameter (blue), ~1 misses it (red)</text>
  <!-- True parameter line -->
  <line x1="340" y1="60" x2="340" y2="500" stroke="#7c3aed" stroke-width="2" stroke-dasharray="6,3"/>
  <text x="344" y="75" font-size="11" fill="#7c3aed" font-weight="bold">True value (mu)</text>
  <!-- 20 CIs: 19 blue (cross the line), 1 red (misses) -->
  <!-- CI 1 --> <line x1="250" y1="90" x2="430" y2="90" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="340" cy="90" r="4" fill="#2563eb"/> <line x1="250" y1="86" x2="250" y2="94" stroke="#3b82f6" stroke-width="2"/> <line x1="430" y1="86" x2="430" y2="94" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 2 --> <line x1="280" y1="110" x2="410" y2="110" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="345" cy="110" r="4" fill="#2563eb"/> <line x1="280" y1="106" x2="280" y2="114" stroke="#3b82f6" stroke-width="2"/> <line x1="410" y1="106" x2="410" y2="114" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 3 --> <line x1="260" y1="130" x2="440" y2="130" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="350" cy="130" r="4" fill="#2563eb"/> <line x1="260" y1="126" x2="260" y2="134" stroke="#3b82f6" stroke-width="2"/> <line x1="440" y1="126" x2="440" y2="134" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 4 --> <line x1="290" y1="150" x2="420" y2="150" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="355" cy="150" r="4" fill="#2563eb"/> <line x1="290" y1="146" x2="290" y2="154" stroke="#3b82f6" stroke-width="2"/> <line x1="420" y1="146" x2="420" y2="154" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 5 --> <line x1="270" y1="170" x2="390" y2="170" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="330" cy="170" r="4" fill="#2563eb"/> <line x1="270" y1="166" x2="270" y2="174" stroke="#3b82f6" stroke-width="2"/> <line x1="390" y1="166" x2="390" y2="174" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 6 (RED - misses!) --> <line x1="350" y1="190" x2="500" y2="190" stroke="#dc2626" stroke-width="2.5" stroke-linecap="round"/> <circle cx="425" cy="190" r="4" fill="#dc2626"/> <line x1="350" y1="186" x2="350" y2="194" stroke="#dc2626" stroke-width="2"/> <line x1="500" y1="186" x2="500" y2="194" stroke="#dc2626" stroke-width="2"/>
  <text x="510" y="194" font-size="11" fill="#dc2626" font-weight="bold">Misses!</text>
  <!-- CI 7 --> <line x1="240" y1="210" x2="420" y2="210" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="330" cy="210" r="4" fill="#2563eb"/> <line x1="240" y1="206" x2="240" y2="214" stroke="#3b82f6" stroke-width="2"/> <line x1="420" y1="206" x2="420" y2="214" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 8 --> <line x1="300" y1="230" x2="440" y2="230" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="370" cy="230" r="4" fill="#2563eb"/> <line x1="300" y1="226" x2="300" y2="234" stroke="#3b82f6" stroke-width="2"/> <line x1="440" y1="226" x2="440" y2="234" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 9 --> <line x1="255" y1="250" x2="405" y2="250" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="330" cy="250" r="4" fill="#2563eb"/> <line x1="255" y1="246" x2="255" y2="254" stroke="#3b82f6" stroke-width="2"/> <line x1="405" y1="246" x2="405" y2="254" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 10 --> <line x1="275" y1="270" x2="425" y2="270" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="350" cy="270" r="4" fill="#2563eb"/> <line x1="275" y1="266" x2="275" y2="274" stroke="#3b82f6" stroke-width="2"/> <line x1="425" y1="266" x2="425" y2="274" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 11 --> <line x1="310" y1="290" x2="430" y2="290" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="370" cy="290" r="4" fill="#2563eb"/> <line x1="310" y1="286" x2="310" y2="294" stroke="#3b82f6" stroke-width="2"/> <line x1="430" y1="286" x2="430" y2="294" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 12 --> <line x1="245" y1="310" x2="395" y2="310" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="320" cy="310" r="4" fill="#2563eb"/> <line x1="245" y1="306" x2="245" y2="314" stroke="#3b82f6" stroke-width="2"/> <line x1="395" y1="306" x2="395" y2="314" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 13 --> <line x1="285" y1="330" x2="450" y2="330" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="368" cy="330" r="4" fill="#2563eb"/> <line x1="285" y1="326" x2="285" y2="334" stroke="#3b82f6" stroke-width="2"/> <line x1="450" y1="326" x2="450" y2="334" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 14 --> <line x1="265" y1="350" x2="415" y2="350" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="340" cy="350" r="4" fill="#2563eb"/> <line x1="265" y1="346" x2="265" y2="354" stroke="#3b82f6" stroke-width="2"/> <line x1="415" y1="346" x2="415" y2="354" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 15 --> <line x1="295" y1="370" x2="445" y2="370" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="370" cy="370" r="4" fill="#2563eb"/> <line x1="295" y1="366" x2="295" y2="374" stroke="#3b82f6" stroke-width="2"/> <line x1="445" y1="366" x2="445" y2="374" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 16 --> <line x1="250" y1="390" x2="400" y2="390" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="325" cy="390" r="4" fill="#2563eb"/> <line x1="250" y1="386" x2="250" y2="394" stroke="#3b82f6" stroke-width="2"/> <line x1="400" y1="386" x2="400" y2="394" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 17 --> <line x1="280" y1="410" x2="435" y2="410" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="358" cy="410" r="4" fill="#2563eb"/> <line x1="280" y1="406" x2="280" y2="414" stroke="#3b82f6" stroke-width="2"/> <line x1="435" y1="406" x2="435" y2="414" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 18 --> <line x1="260" y1="430" x2="410" y2="430" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="335" cy="430" r="4" fill="#2563eb"/> <line x1="260" y1="426" x2="260" y2="434" stroke="#3b82f6" stroke-width="2"/> <line x1="410" y1="426" x2="410" y2="434" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 19 --> <line x1="305" y1="450" x2="425" y2="450" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="365" cy="450" r="4" fill="#2563eb"/> <line x1="305" y1="446" x2="305" y2="454" stroke="#3b82f6" stroke-width="2"/> <line x1="425" y1="446" x2="425" y2="454" stroke="#3b82f6" stroke-width="2"/>
  <!-- CI 20 --> <line x1="270" y1="470" x2="430" y2="470" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round"/> <circle cx="350" cy="470" r="4" fill="#2563eb"/> <line x1="270" y1="466" x2="270" y2="474" stroke="#3b82f6" stroke-width="2"/> <line x1="430" y1="466" x2="430" y2="474" stroke="#3b82f6" stroke-width="2"/>
  <!-- Legend -->
  <line x1="80" y1="500" x2="110" y2="500" stroke="#3b82f6" stroke-width="2.5"/> <text x="115" y="504" font-size="11" fill="#3b82f6">Contains true value (19/20)</text>
  <line x1="340" y1="500" x2="370" y2="500" stroke="#dc2626" stroke-width="2.5"/> <text x="375" y="504" font-size="11" fill="#dc2626">Misses true value (1/20)</text>
</svg>
</div>

> **Common pitfall:** A 95% CI does NOT mean "there is a 95% probability the true value is in this interval." Once you compute a specific interval, the true value is either in it or it isn't. The 95% refers to the *procedure's* long-run success rate, not the probability for any single interval.

## Point Estimates Are Not Enough

A point estimate is a single number — a sample mean, a proportion, a median. It is our best guess, but it carries no information about precision.

| Scenario | Point Estimate | What's Missing? |
|---|---|---|
| Drug IC50 from 8 replicates | 12.3 nM | Could be 8-16 nM or 11.9-12.7 nM |
| Mutation frequency in 50 patients | 34% | Could be 21-47% or 30-38% |
| Mean tumor volume after treatment | 180 mm³ | How variable was the response? |

The confidence interval supplements the point estimate with a measure of uncertainty. Narrow intervals mean precise estimates; wide intervals mean the data leaves much room for doubt.

## CI for a Mean: x-bar plus-or-minus t times SE

The most common confidence interval is for a population mean. The formula is:

**CI = x-bar +/- t(alpha/2, df) x SE**

Where:
- **x-bar** is the sample mean
- **SE = s / sqrt(n)** is the standard error of the mean
- **t(alpha/2, df)** is the critical value from the t-distribution with df = n - 1
- **alpha** = 1 - confidence level (for 95% CI, alpha = 0.05)

### Why the t-Distribution for Small Samples?

When n is large (say, n > 30), the t-distribution closely resembles the normal distribution. But for small n — common in biology where each replicate is expensive — the t-distribution has heavier tails, producing wider intervals that honestly reflect our greater uncertainty.

| Sample Size (n) | t-critical (95%) | z-critical (95%) | Difference |
|---|---|---|---|
| 5 | 2.776 | 1.960 | 42% wider |
| 10 | 2.262 | 1.960 | 15% wider |
| 30 | 2.045 | 1.960 | 4% wider |
| 100 | 1.984 | 1.960 | ~1% wider |
| 1000 | 1.962 | 1.960 | Negligible |

> **Key insight:** For biological experiments with n < 30, always use the t-distribution. Using z would give falsely narrow intervals that overstate your precision.

## CI for a Proportion

When the variable is binary — mutation present/absent, responder/non-responder — we need a CI for a proportion p-hat = x/n.

### Wald Interval (Simple but Flawed)

**CI = p-hat +/- z x sqrt(p-hat(1 - p-hat) / n)**

This is the textbook formula, but it performs poorly when p is near 0 or 1, or when n is small. It can even produce intervals that extend below 0 or above 1.

### Wilson Interval (Preferred)

The Wilson score interval adjusts the center and width, and is recommended for most biological applications:

**CI = (p-hat + z²/2n +/- z x sqrt(p-hat(1-p-hat)/n + z²/4n²)) / (1 + z²/n)**

> **Clinical relevance:** When reporting mutation carrier frequencies, drug response rates, or diagnostic sensitivity/specificity, always use Wilson intervals. Regulatory agencies expect intervals that behave properly even at extreme proportions.

## CI for the Difference Between Two Means

Often the real question is not "what is the mean?" but "how much do two groups differ?" The CI for the difference between two independent means is:

**CI = (x-bar1 - x-bar2) +/- t x SE_diff**

Where SE_diff = sqrt(s1²/n1 + s2²/n2) for Welch's approach.

**The critical interpretation:** If the CI for the difference includes zero, the data are consistent with no difference between the groups. If it excludes zero, the difference is statistically significant.

| CI for Difference | Interpretation |
|---|---|
| [1.2, 4.8] | Groups differ; difference is between 1.2 and 4.8 units |
| [-0.5, 3.1] | Includes zero; cannot rule out no difference |
| [-4.2, -1.1] | Groups differ; group 2 is higher by 1.1 to 4.2 units |

## Bootstrap Confidence Intervals

What if your statistic is a median, a ratio, or something with no tidy formula? The **bootstrap** is a computer-intensive method that works for *any* statistic:

1. Resample your data **with replacement**, same size as original
2. Compute the statistic on the resample
3. Repeat 10,000 times
4. Take the 2.5th and 97.5th percentiles of the bootstrap distribution

This is called the **percentile method**. No assumptions about normality or distribution shape are required.

> **Key insight:** Bootstrap CIs are the Swiss army knife of interval estimation. When in doubt, bootstrap it.

## What Controls CI Width?

Three factors determine how wide or narrow your confidence interval will be:

| Factor | Effect on Width | Biological Implication |
|---|---|---|
| **Sample size (n)** | Width ~ 1/sqrt(n) | Doubling n cuts width by ~30% |
| **Variability (s)** | Width ~ s | High biological variability = wider CIs |
| **Confidence level** | 99% > 95% > 90% | Higher confidence = wider interval |

This is why power calculations matter: before an experiment, you choose n to achieve a CI narrow enough to be scientifically useful.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="320" viewBox="0 0 650 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">CI Width Narrows with Larger Sample Size</text>
  <text x="325" y="46" text-anchor="middle" font-size="12" fill="#6b7280">Same population, same true mean -- only n changes</text>
  <!-- True value line -->
  <line x1="325" y1="65" x2="325" y2="280" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="5,3"/>
  <text x="329" y="78" font-size="10" fill="#7c3aed">True mean</text>
  <!-- n = 10: wide CI -->
  <text x="100" y="125" text-anchor="end" font-size="13" fill="#374151" font-weight="bold">n = 10</text>
  <line x1="175" y1="120" x2="475" y2="120" stroke="#ef4444" stroke-width="4" stroke-linecap="round"/>
  <line x1="175" y1="112" x2="175" y2="128" stroke="#ef4444" stroke-width="2.5"/>
  <line x1="475" y1="112" x2="475" y2="128" stroke="#ef4444" stroke-width="2.5"/>
  <circle cx="325" cy="120" r="6" fill="#dc2626"/>
  <text x="485" y="125" font-size="11" fill="#dc2626">Wide: +/- 2.1 nM</text>
  <!-- n = 50: medium CI -->
  <text x="100" y="185" text-anchor="end" font-size="13" fill="#374151" font-weight="bold">n = 50</text>
  <line x1="245" y1="180" x2="405" y2="180" stroke="#3b82f6" stroke-width="4" stroke-linecap="round"/>
  <line x1="245" y1="172" x2="245" y2="188" stroke="#3b82f6" stroke-width="2.5"/>
  <line x1="405" y1="172" x2="405" y2="188" stroke="#3b82f6" stroke-width="2.5"/>
  <circle cx="325" cy="180" r="6" fill="#2563eb"/>
  <text x="415" y="185" font-size="11" fill="#2563eb">Medium: +/- 0.9 nM</text>
  <!-- n = 200: narrow CI -->
  <text x="100" y="245" text-anchor="end" font-size="13" fill="#374151" font-weight="bold">n = 200</text>
  <line x1="285" y1="240" x2="365" y2="240" stroke="#16a34a" stroke-width="4" stroke-linecap="round"/>
  <line x1="285" y1="232" x2="285" y2="248" stroke="#16a34a" stroke-width="2.5"/>
  <line x1="365" y1="232" x2="365" y2="248" stroke="#16a34a" stroke-width="2.5"/>
  <circle cx="325" cy="240" r="6" fill="#16a34a"/>
  <text x="375" y="245" font-size="11" fill="#16a34a">Narrow: +/- 0.5 nM</text>
  <!-- Formula annotation -->
  <text x="325" y="300" text-anchor="middle" font-size="12" fill="#6b7280">Width ~ 1/sqrt(n): doubling n cuts width by ~30%, quadrupling cuts by ~50%</text>
</svg>
</div>

## Confidence Intervals in BioLang

### IC50 Confidence Interval — Parametric

```bio
# IC50 measurements (nM) from 8 replicates
let ic50 = [11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, 12.5]

let n = len(ic50)
let x_bar = mean(ic50)
let se = stdev(ic50) / sqrt(n)

# 95% CI using normal approximation (for small n, t > z)
let t_crit = qnorm(0.975)
let ci_lower = x_bar - t_crit * se
let ci_upper = x_bar + t_crit * se

print("IC50 mean: {x_bar:.2} nM")
print("95% CI: [{ci_lower:.2}, {ci_upper:.2}] nM")
print("Standard error: {se:.3} nM")
print("Critical value: {t_crit:.3}")
```

### IC50 Confidence Interval — Bootstrap

```bio
set_seed(42)
# Bootstrap CI: no distributional assumptions

let ic50 = [11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, 12.5]

# Bootstrap: resample 10,000 times, compute mean each time
let n_boot = 10000
let boot_means = []
for i in range(0, n_boot) {
    let resample = []
    for j in range(0, len(ic50)) {
        resample = append(resample, ic50[random_int(0, len(ic50) - 1)])
    }
    boot_means = append(boot_means, mean(resample))
}

# Percentile method
let boot_lower = quantile(boot_means, 0.025)
let boot_upper = quantile(boot_means, 0.975)

print("Bootstrap 95% CI: [{boot_lower:.2}, {boot_upper:.2}] nM")

# Visualize the bootstrap distribution
histogram(boot_means, {bins: 50, title: "Bootstrap Distribution of IC50 Mean", x_label: "Mean IC50 (nM)"})
```

### Bootstrap CI for Median (No Parametric Formula Exists)

```bio
set_seed(42)
# Gene expression values (FPKM) — skewed distribution
let expression = [0.1, 0.3, 0.8, 1.2, 1.5, 2.1, 3.4, 8.7, 12.1, 45.6]

let obs_median = median(expression)

# Bootstrap the median
let n_boot = 10000
let boot_medians = []
for i in range(0, n_boot) {
    let resample = []
    for j in range(0, len(expression)) {
        resample = append(resample, expression[random_int(0, len(expression) - 1)])
    }
    boot_medians = append(boot_medians, median(resample))
}
let ci_lower = quantile(boot_medians, 0.025)
let ci_upper = quantile(boot_medians, 0.975)

print("Observed median: {obs_median:.2} FPKM")
print("Bootstrap 95% CI for median: [{ci_lower:.2}, {ci_upper:.2}] FPKM")
```

### Error Bar Plot: Comparing Drug Concentrations

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="380" viewBox="0 0 650 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Error Bar Plot: 3 Drug Candidates with 95% CIs</text>
  <!-- Axes -->
  <line x1="120" y1="50" x2="120" y2="310" stroke="#374151" stroke-width="1.5"/>
  <line x1="120" y1="310" x2="580" y2="310" stroke="#374151" stroke-width="1.5"/>
  <!-- Y-axis labels -->
  <text x="110" y="314" text-anchor="end" font-size="11" fill="#6b7280">0</text>
  <text x="110" y="258" text-anchor="end" font-size="11" fill="#6b7280">5</text>
  <text x="110" y="206" text-anchor="end" font-size="11" fill="#6b7280">10</text>
  <text x="110" y="154" text-anchor="end" font-size="11" fill="#6b7280">15</text>
  <text x="110" y="102" text-anchor="end" font-size="11" fill="#6b7280">20</text>
  <text x="110" y="54" text-anchor="end" font-size="11" fill="#6b7280">25</text>
  <!-- Y-axis gridlines -->
  <line x1="120" y1="254" x2="580" y2="254" stroke="#e5e7eb" stroke-width="0.5"/>
  <line x1="120" y1="202" x2="580" y2="202" stroke="#e5e7eb" stroke-width="0.5"/>
  <line x1="120" y1="150" x2="580" y2="150" stroke="#e5e7eb" stroke-width="0.5"/>
  <line x1="120" y1="98" x2="580" y2="98" stroke="#e5e7eb" stroke-width="0.5"/>
  <!-- Y-axis title -->
  <text x="30" y="185" text-anchor="middle" font-size="12" fill="#374151" transform="rotate(-90, 30, 185)">IC50 (nM)</text>
  <!-- Drug A: mean ~12.3, CI ~[11.5, 13.1] -->
  <rect x="180" y="183" width="60" height="127" fill="#93c5fd" rx="3"/>
  <line x1="210" y1="171" x2="210" y2="174" stroke="#2563eb" stroke-width="2.5"/>
  <line x1="195" y1="171" x2="225" y2="171" stroke="#2563eb" stroke-width="2"/>
  <line x1="210" y1="178" x2="210" y2="174" stroke="#2563eb" stroke-width="2.5"/>
  <line x1="195" y1="178" x2="225" y2="178" stroke="#2563eb" stroke-width="2"/>
  <circle cx="210" cy="183" r="5" fill="#2563eb"/>
  <text x="210" y="340" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">Drug A</text>
  <text x="210" y="355" text-anchor="middle" font-size="10" fill="#6b7280">12.3 nM</text>
  <!-- Drug B: mean ~25.5, CI ~[23.5, 27.5] -->
  <rect x="310" y="48" width="60" height="262" fill="#c4b5fd" rx="3"/>
  <line x1="340" y1="42" x2="340" y2="54" stroke="#7c3aed" stroke-width="2.5"/>
  <line x1="325" y1="42" x2="355" y2="42" stroke="#7c3aed" stroke-width="2"/>
  <line x1="325" y1="54" x2="355" y2="54" stroke="#7c3aed" stroke-width="2"/>
  <circle cx="340" cy="48" r="5" fill="#7c3aed"/>
  <text x="340" y="340" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">Drug B</text>
  <text x="340" y="355" text-anchor="middle" font-size="10" fill="#6b7280">25.5 nM</text>
  <!-- Drug C: mean ~8.6, CI ~[7.5, 9.7] -->
  <rect x="440" y="221" width="60" height="89" fill="#bbf7d0" rx="3"/>
  <line x1="470" y1="213" x2="470" y2="210" stroke="#16a34a" stroke-width="2.5"/>
  <line x1="455" y1="210" x2="485" y2="210" stroke="#16a34a" stroke-width="2"/>
  <line x1="470" y1="227" x2="470" y2="230" stroke="#16a34a" stroke-width="2.5"/>
  <line x1="455" y1="230" x2="485" y2="230" stroke="#16a34a" stroke-width="2"/>
  <circle cx="470" cy="221" r="5" fill="#16a34a"/>
  <text x="470" y="340" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">Drug C</text>
  <text x="470" y="355" text-anchor="middle" font-size="10" fill="#6b7280">8.6 nM</text>
  <!-- Annotation -->
  <text x="325" y="375" text-anchor="middle" font-size="11" fill="#6b7280">Lower IC50 = more potent. Non-overlapping CIs suggest significant differences.</text>
</svg>
</div>

```bio
# IC50 values for three drug candidates
let drug_a = [12.3, 11.8, 13.1, 12.0, 11.5, 12.7, 13.4, 12.1]
let drug_b = [25.1, 28.3, 22.7, 26.9, 24.5, 27.1, 23.8, 25.6]
let drug_c = [8.2, 9.1, 7.5, 8.8, 10.2, 8.0, 9.5, 7.8]

let drugs = ["Drug A", "Drug B", "Drug C"]
let means = [mean(drug_a), mean(drug_b), mean(drug_c)]

# Compute 95% CIs for each
let compute_ci = |data| {
  let n = len(data)
  let se = stdev(data) / sqrt(n)
  let t_crit = qnorm(0.975)
  [mean(data) - t_crit * se, mean(data) + t_crit * se]
}

let ci_a = compute_ci(drug_a)
let ci_b = compute_ci(drug_b)
let ci_c = compute_ci(drug_c)

print("Drug A: {means[0]:.1} nM, 95% CI [{ci_a[0]:.1}, {ci_a[1]:.1}]")
print("Drug B: {means[1]:.1} nM, 95% CI [{ci_b[0]:.1}, {ci_b[1]:.1}]")
print("Drug C: {means[2]:.1} nM, 95% CI [{ci_c[0]:.1}, {ci_c[1]:.1}]")

# Bar chart with error bars
bar_chart(drugs, means, {title: "IC50 Comparison with 95% CIs", y_label: "IC50 (nM)", error_bars: [ci_a, ci_b, ci_c]})
```

### CI for Difference Between Two Means

```bio
# Compare tumor volume between treated and control mice
let treated = [180, 210, 165, 225, 195, 172, 218, 198]
let control = [485, 512, 468, 530, 495, 478, 521, 503]

let diff = mean(treated) - mean(control)
let se_diff = sqrt(variance(treated) / len(treated) + variance(control) / len(control))
let df = len(treated) + len(control) - 2
let t_crit = qnorm(0.975)  # approximate for moderate df

let ci_lower = diff - t_crit * se_diff
let ci_upper = diff + t_crit * se_diff

print("Mean difference: {diff:.1} mm^3")
print("95% CI for difference: [{ci_lower:.1}, {ci_upper:.1}] mm^3")

if ci_upper < 0 {
  print("CI excludes zero: treatment significantly reduces tumor volume")
} else {
  print("CI includes zero: cannot rule out no difference")
}
```

### Vaccine Efficacy CI (Proportion)

```bio
set_seed(42)
# Clinical trial: 15 of 200 vaccinated got infected vs 60 of 200 placebo
let p_vacc = 15 / 200
let p_plac = 60 / 200
let efficacy = 1.0 - (p_vacc / p_plac)

print("Vaccine efficacy: {efficacy * 100:.1}%")

# CI for proportion (vaccinated group infection rate)
let n = 200
let z = 1.96
let se_p = sqrt(p_vacc * (1.0 - p_vacc) / n)
let ci_lower_p = p_vacc - z * se_p
let ci_upper_p = p_vacc + z * se_p

print("Infection rate (vaccinated): {p_vacc*100:.1}%")
print("95% CI for infection rate: [{ci_lower_p*100:.1}%, {ci_upper_p*100:.1}%]")

# Bootstrap CI for vaccine efficacy itself
let vacc_outcomes = flatten([repeat(1, 15), repeat(0, 185)])
let plac_outcomes = flatten([repeat(1, 60), repeat(0, 140)])

let n_boot = 10000
let boot_eff = []
for i in range(0, n_boot) {
    let v_resample = []
    let p_resample = []
    for j in range(0, len(vacc_outcomes)) {
        v_resample = append(v_resample, vacc_outcomes[random_int(0, len(vacc_outcomes) - 1)])
        p_resample = append(p_resample, plac_outcomes[random_int(0, len(plac_outcomes) - 1)])
    }
    let pv = mean(v_resample)
    let pp = mean(p_resample)
    let eff = if pp == 0.0 then 0.0 else 1.0 - (pv / pp)
    boot_eff = append(boot_eff, eff)
}

let eff_ci = [quantile(boot_eff, 0.025), quantile(boot_eff, 0.975)]
print("Bootstrap 95% CI for efficacy: [{eff_ci[0]*100:.1}%, {eff_ci[1]*100:.1}%]")
```

**Python:**

```python
import numpy as np
from scipy import stats

ic50 = [11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, 12.5]
ci = stats.t.interval(0.95, df=len(ic50)-1,
                       loc=np.mean(ic50),
                       scale=stats.sem(ic50))
print(f"95% CI: [{ci[0]:.2f}, {ci[1]:.2f}]")

# Bootstrap
boot = [np.mean(np.random.choice(ic50, len(ic50))) for _ in range(10000)]
print(f"Bootstrap CI: [{np.percentile(boot, 2.5):.2f}, {np.percentile(boot, 97.5):.2f}]")
```

**R:**

```r
ic50 <- c(11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, 12.5)
t.test(ic50)$conf.int

# Bootstrap
library(boot)
boot_fn <- function(data, i) mean(data[i])
b <- boot(ic50, boot_fn, R = 10000)
boot.ci(b, type = "perc")
```

## Exercises

**Exercise 1: Compute a CI by Hand**

Eight mice on a high-fat diet had cholesterol levels: 215, 228, 197, 241, 209, 233, 220, 212 mg/dL. Compute the 95% CI for the mean cholesterol.

```bio
let cholesterol = [215, 228, 197, 241, 209, 233, 220, 212]

# TODO: Compute mean, SE, critical value, and the 95% CI
# Hint: df = n - 1, use qnorm(0.975) as approximate critical value
```

**Exercise 2: Bootstrap a Ratio**

Gene A has FPKM values [2.1, 3.4, 1.8, 4.2, 2.9] in tumor and [1.0, 1.2, 0.9, 1.5, 1.1] in normal. Bootstrap a 95% CI for the tumor/normal fold change of medians.

```bio
let tumor = [2.1, 3.4, 1.8, 4.2, 2.9]
let normal = [1.0, 1.2, 0.9, 1.5, 1.1]

# TODO: Bootstrap the ratio median(tumor) / median(normal)
# Use n_boot = 10000, then extract 2.5th and 97.5th percentiles with quantile()
```

**Exercise 3: Overlapping CIs**

Compute 95% CIs for Drug X (IC50: [5.2, 6.1, 4.8, 5.5, 6.3, 5.0]) and Drug Y (IC50: [5.8, 6.5, 7.2, 6.0, 5.9, 6.8]). Do the CIs overlap? What does this suggest?

```bio
let drug_x = [5.2, 6.1, 4.8, 5.5, 6.3, 5.0]
let drug_y = [5.8, 6.5, 7.2, 6.0, 5.9, 6.8]

# TODO: Compute CIs for both, then compute CI for the difference
# Note: overlapping CIs do NOT necessarily mean non-significant difference
```

**Exercise 4: Effect of Sample Size**

Starting with n = 5 replicates drawn from the IC50 data, increase to n = 10, 20, 50, and 100 (use bootstrap resampling to simulate larger samples). Plot CI width vs sample size.

```bio
let ic50 = [11.2, 13.1, 12.8, 10.9, 14.2, 12.0, 11.7, 12.5]

# TODO: For each sample size, bootstrap to simulate, compute CI width
# Plot sample size vs CI width using line_plot
```

## Key Takeaways

- A **confidence interval** gives a range of plausible values for a population parameter, not just a point estimate
- The 95% in "95% CI" refers to the long-run coverage rate of the procedure, not the probability for a specific interval
- For small samples (n < 30), always use the **t-distribution** — it accounts for extra uncertainty
- **Bootstrap CIs** work for any statistic (median, ratio, fold change) without distributional assumptions
- CI width shrinks with larger n, lower variability, and lower confidence level
- A CI for the difference that **includes zero** means the data are consistent with no difference
- CIs are more informative than p-values alone: they tell you both significance AND the plausible magnitude of an effect

## What's Next

Tomorrow we formalize the logic behind "ruling out chance" with hypothesis testing. You will learn to frame biological questions as null and alternative hypotheses, compute p-values, and understand the courtroom analogy that makes the whole framework click. Confidence intervals and hypothesis tests are two sides of the same coin — a 95% CI that excludes zero corresponds exactly to a p-value less than 0.05.
