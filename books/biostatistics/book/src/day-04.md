# Day 4: Probability — Quantifying Uncertainty

<div class="day-meta">
<span class="badge">Day 4 of 30</span>
<span class="badge">Prerequisites: Days 1-3</span>
<span class="badge">~50 min</span>
<span class="badge">Hands-on</span>
</div>

## The Problem

Maria and David sit in a genetic counselor's office. The air is still. Maria has just learned she carries a pathogenic BRCA1 mutation — a variant that dramatically increases lifetime risk of breast and ovarian cancer. They are planning to start a family and they need answers.

What is the probability their child inherits the mutation? If the child inherits it, what is the probability she develops breast cancer by age 70? They are considering preimplantation genetic testing — if the test says the embryo is mutation-free, how confident can they be? The counselor pulls out a notepad and begins writing probabilities.

This is not an abstract exercise. These numbers will determine whether Maria and David proceed with natural conception, pursue IVF with genetic screening, or consider adoption. The difference between a 50% risk and a 5% risk changes lives. Understanding how to compute, combine, and interpret probabilities is not just statistics — in genetics, it is clinical care.

## What Is Probability?

Probability is a number between 0 and 1 that quantifies how likely an event is to occur. Zero means impossible. One means certain. Everything interesting happens in between.

Think of probability as a weather forecast. When the forecaster says "70% chance of rain," she means: in historical situations with similar atmospheric conditions, it rained about 70% of the time. She is not saying it will rain exactly 70% of the total rainfall. She is quantifying uncertainty about a future event using past data and models.

In biology, we use probability constantly:
- The probability a child inherits a specific allele from a heterozygous parent: 0.5
- The probability a random human carries at least one pathogenic BRCA1 variant: roughly 1/400
- The probability that a drug produces a response in a given cancer type: varies, but measured in clinical trials
- The probability that a sequencing read is mapped incorrectly: the mapping quality score encodes this directly

| Probability | Meaning | Example |
|---|---|---|
| 0 | Impossible | Rolling a 7 on a standard die |
| 0.001 | Very unlikely | Rare disease prevalence |
| 0.05 | Unlikely | Conventional significance threshold |
| 0.25 | Possible | Child inheriting recessive allele from two carriers |
| 0.50 | Even odds | Coin flip, heterozygous allele transmission |
| 0.95 | Very likely | Diagnostic test sensitivity |
| 1.0 | Certain | Sum of all possible outcomes |

## Basic Rules of Probability

### The Addition Rule

The probability of event A **or** event B occurring depends on whether they can both happen at the same time.

**Mutually exclusive events** (cannot co-occur):
P(A or B) = P(A) + P(B)

A person's blood type is A, B, AB, or O. These are mutually exclusive — you cannot be both type A and type B simultaneously. So P(A or B) = P(A) + P(B) = 0.40 + 0.11 = 0.51.

**Non-mutually exclusive events** (can co-occur):
P(A or B) = P(A) + P(B) - P(A and B)

A patient might have diabetes, hypertension, or both. If P(diabetes) = 0.10, P(hypertension) = 0.30, and P(both) = 0.05, then P(either) = 0.10 + 0.30 - 0.05 = 0.35. You subtract the overlap to avoid double-counting.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="300" viewBox="0 0 660 300" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Set Operations: Union and Intersection</text>
  <!-- Universe rectangle -->
  <rect x="40" y="40" width="580" height="240" rx="8" fill="#f8fafc" stroke="#d1d5db" stroke-width="1.5"/>
  <text x="600" y="60" text-anchor="end" font-size="11" fill="#9ca3af">Sample Space S</text>
  <!-- Circle A -->
  <circle cx="250" cy="160" r="100" fill="#3b82f6" fill-opacity="0.2" stroke="#2563eb" stroke-width="2.5"/>
  <!-- Circle B -->
  <circle cx="400" cy="160" r="100" fill="#ef4444" fill-opacity="0.2" stroke="#dc2626" stroke-width="2.5"/>
  <!-- Intersection highlight -->
  <clipPath id="clipA"><circle cx="250" cy="160" r="100"/></clipPath>
  <circle cx="400" cy="160" r="100" fill="#7c3aed" fill-opacity="0.3" clip-path="url(#clipA)"/>
  <!-- Labels -->
  <text x="195" y="155" text-anchor="middle" font-size="16" font-weight="bold" fill="#2563eb">A only</text>
  <text x="195" y="175" text-anchor="middle" font-size="12" fill="#2563eb">P(A) - P(A and B)</text>
  <text x="325" y="148" text-anchor="middle" font-size="14" font-weight="bold" fill="#7c3aed">A and B</text>
  <text x="325" y="168" text-anchor="middle" font-size="12" fill="#7c3aed">P(A and B)</text>
  <text x="455" y="155" text-anchor="middle" font-size="16" font-weight="bold" fill="#dc2626">B only</text>
  <text x="455" y="175" text-anchor="middle" font-size="12" fill="#dc2626">P(B) - P(A and B)</text>
  <!-- Labels outside circles -->
  <text x="120" y="90" font-size="14" font-weight="bold" fill="#2563eb">A = Diabetes</text>
  <text x="120" y="108" font-size="12" fill="#2563eb">P(A) = 0.10</text>
  <text x="420" y="72" font-size="14" font-weight="bold" fill="#dc2626">B = Hypertension</text>
  <text x="420" y="90" font-size="12" fill="#dc2626">P(B) = 0.30</text>
  <!-- Formula box -->
  <rect x="170" y="240" width="320" height="30" rx="4" fill="#faf5ff" stroke="#7c3aed" stroke-width="1"/>
  <text x="330" y="260" text-anchor="middle" font-size="12" font-weight="600" fill="#7c3aed">P(A or B) = P(A) + P(B) - P(A and B) = 0.10 + 0.30 - 0.05 = 0.35</text>
</svg>
</div>

### The Multiplication Rule

The probability of event A **and** event B both occurring depends on whether they are independent.

**Independent events** (one does not affect the other):
P(A and B) = P(A) &times; P(B)

The probability that two unrelated people both carry a BRCA1 mutation: P(carrier) &times; P(carrier) = (1/400) &times; (1/400) = 1/160,000.

**Dependent events** (one affects the other):
P(A and B) = P(A) &times; P(B|A)

where P(B|A) is the probability of B **given that** A has occurred. If a mother carries BRCA1 (event A), the probability her daughter both inherits it (B) and develops cancer by 70 (C) is:
P(B and C) = P(B|A) &times; P(C|B) = 0.5 &times; 0.72 = 0.36.

### The Complement Rule

P(not A) = 1 - P(A)

The probability of **not** inheriting the mutation is 1 - 0.5 = 0.5. The probability a diagnostic test does **not** give a false positive is 1 - (false positive rate). Simple but powerful — often easier to compute the complement and subtract.

> **Key insight:** Most probability errors in biology come from confusing independent and dependent events, or from forgetting to subtract the overlap in non-mutually exclusive events. Write out the formula before plugging in numbers.

## Conditional Probability

Conditional probability is the probability of an event **given that** another event has occurred. It is written P(A|B) and read "the probability of A given B."

This is where most people's intuition breaks down, because **P(A|B) is not the same as P(B|A).**

### The Critical Distinction

- P(cancer | BRCA1 mutation) &asymp; 0.72 — If you carry BRCA1, your lifetime breast cancer risk is about 72%.
- P(BRCA1 mutation | cancer) &asymp; 0.05 — If you have breast cancer, the probability it is due to BRCA1 is only about 5%.

These are completely different numbers answering completely different questions. Confusing them is called the **inverse probability fallacy**, and it has real consequences in medicine, law, and genetics.

### The Prosecutor's Fallacy

In forensic genetics, the prosecutor's fallacy works like this: "The probability of this DNA match occurring by chance is 1 in 10 million. Therefore, the probability the defendant is innocent is 1 in 10 million."

This is logically wrong. P(match | innocent) is not P(innocent | match). In a city of 8 million people, you would expect roughly one other person to match by chance. If the only evidence is the DNA match, the probability of innocence might be closer to 50%, not 1 in 10 million.

The same fallacy appears in genetic testing: "The test is 99% accurate" does not mean a positive result is 99% likely to be correct. The answer depends on how common the condition is — which brings us to Bayes' theorem.

> **Common pitfall:** Never interpret P(result | hypothesis) as P(hypothesis | result). This mistake is ubiquitous in clinical genetics, drug development, and forensics. Bayes' theorem is the correction.

## Bayes' Theorem

Bayes' theorem provides the mathematical machinery to flip conditional probabilities. Given P(B|A), it computes P(A|B):

**P(A|B) = P(B|A) &times; P(A) / P(B)**

In words: the probability of A given B equals the probability of B given A, times the prior probability of A, divided by the total probability of B.

### The Diagnostic Test Example

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="360" viewBox="0 0 680 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <defs>
    <marker id="arrowBayes" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#6b7280"/></marker>
  </defs>
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Bayes' Theorem: Diagnostic Test Decision Tree</text>
  <!-- Root: Population -->
  <rect x="255" y="40" width="170" height="36" rx="18" fill="#f1f5f9" stroke="#64748b" stroke-width="2"/>
  <text x="340" y="63" text-anchor="middle" font-size="12" font-weight="600" fill="#334155">100,000 People</text>
  <!-- Branch to Diseased -->
  <line x1="290" y1="76" x2="160" y2="118" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="210" y="92" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="600">P = 0.001</text>
  <!-- Branch to Healthy -->
  <line x1="390" y1="76" x2="520" y2="118" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="470" y="92" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="600">P = 0.999</text>
  <!-- Diseased box -->
  <rect x="80" y="118" width="160" height="36" rx="18" fill="#fef2f2" stroke="#dc2626" stroke-width="2"/>
  <text x="160" y="141" text-anchor="middle" font-size="11" font-weight="600" fill="#dc2626">Diseased: 100</text>
  <!-- Healthy box -->
  <rect x="440" y="118" width="160" height="36" rx="18" fill="#f0fdf4" stroke="#16a34a" stroke-width="2"/>
  <text x="520" y="141" text-anchor="middle" font-size="11" font-weight="600" fill="#16a34a">Healthy: 99,900</text>
  <!-- Diseased -> Test+ -->
  <line x1="120" y1="154" x2="80" y2="200" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="85" y="178" text-anchor="middle" font-size="9" fill="#6b7280">Sens=0.99</text>
  <!-- Diseased -> Test- -->
  <line x1="200" y1="154" x2="240" y2="200" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="235" y="178" text-anchor="middle" font-size="9" fill="#6b7280">0.01</text>
  <!-- Healthy -> Test+ -->
  <line x1="480" y1="154" x2="440" y2="200" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="445" y="178" text-anchor="middle" font-size="9" fill="#6b7280">FPR=0.05</text>
  <!-- Healthy -> Test- -->
  <line x1="560" y1="154" x2="600" y2="200" stroke="#6b7280" stroke-width="1.5" marker-end="url(#arrowBayes)"/>
  <text x="595" y="178" text-anchor="middle" font-size="9" fill="#6b7280">Spec=0.95</text>
  <!-- Leaf nodes -->
  <rect x="20" y="200" width="120" height="44" rx="6" fill="#fef2f2" stroke="#dc2626" stroke-width="1.5"/>
  <text x="80" y="218" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">TP: 99</text>
  <text x="80" y="234" text-anchor="middle" font-size="9" fill="#991b1b">True Positive</text>
  <rect x="180" y="200" width="120" height="44" rx="6" fill="#fefce8" stroke="#ca8a04" stroke-width="1.5"/>
  <text x="240" y="218" text-anchor="middle" font-size="11" font-weight="bold" fill="#ca8a04">FN: 1</text>
  <text x="240" y="234" text-anchor="middle" font-size="9" fill="#854d0e">False Negative</text>
  <rect x="380" y="200" width="120" height="44" rx="6" fill="#fefce8" stroke="#ca8a04" stroke-width="1.5"/>
  <text x="440" y="218" text-anchor="middle" font-size="11" font-weight="bold" fill="#ca8a04">FP: 4,995</text>
  <text x="440" y="234" text-anchor="middle" font-size="9" fill="#854d0e">False Positive</text>
  <rect x="540" y="200" width="120" height="44" rx="6" fill="#f0fdf4" stroke="#16a34a" stroke-width="1.5"/>
  <text x="600" y="218" text-anchor="middle" font-size="11" font-weight="bold" fill="#16a34a">TN: 94,905</text>
  <text x="600" y="234" text-anchor="middle" font-size="9" fill="#14532d">True Negative</text>
  <!-- Summary box -->
  <rect x="100" y="265" width="480" height="82" rx="8" fill="#faf5ff" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="340" y="286" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">The Surprising Result</text>
  <text x="340" y="305" text-anchor="middle" font-size="11" fill="#581c87">Total positive tests: 99 + 4,995 = 5,094</text>
  <text x="340" y="322" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">PPV = 99 / 5,094 = 1.94% (not 99%!)</text>
  <text x="340" y="340" text-anchor="middle" font-size="10" fill="#6b7280">False positives overwhelm true positives when the disease is rare</text>
</svg>
</div>

This is the single most important application of Bayes' theorem in biomedical science. Work through it carefully.

**Setup:**
- A genetic disease has a prevalence of 1 in 1,000 (P(disease) = 0.001)
- A diagnostic test has 99% sensitivity: P(positive | disease) = 0.99
- The test has 95% specificity: P(negative | no disease) = 0.95, so P(positive | no disease) = 0.05

**Question:** A patient tests positive. What is the probability they actually have the disease?

**Intuition says:** The test is 99% sensitive and 95% specific — surely a positive result means ~95-99% chance of disease?

**Bayes says:**

P(disease | positive) = P(positive | disease) &times; P(disease) / P(positive)

P(positive) = P(positive | disease) &times; P(disease) + P(positive | no disease) &times; P(no disease)
P(positive) = 0.99 &times; 0.001 + 0.05 &times; 0.999
P(positive) = 0.00099 + 0.04995 = 0.05094

P(disease | positive) = 0.00099 / 0.05094 = **0.0194**

A positive test result means only a **1.94% chance** of actually having the disease.

How can a "99% accurate" test give such a low positive predictive value? Because the disease is rare. In 100,000 people, 100 have the disease (99 test positive) and 99,900 are healthy (4,995 test positive by mistake). Of the 5,094 total positives, only 99 are true positives.

| Group | Population | Test Positive | Test Negative |
|---|---|---|---|
| Diseased | 100 | 99 (TP) | 1 (FN) |
| Healthy | 99,900 | 4,995 (FP) | 94,905 (TN) |
| **Total** | **100,000** | **5,094** | **94,906** |

PPV = 99 / 5,094 = 1.94%. The false positives overwhelm the true positives.

> **Clinical relevance:** This is why population-wide genetic screening for rare conditions produces mostly false positives. It is also why a confirmatory test with a different methodology is always required. Understanding PPV is essential for anyone interpreting genetic test results.

### Bayes in BioLang

```bio
# Diagnostic test calculator using Bayes' theorem
let prevalence = 0.001      # P(disease) = 1 in 1,000
let sensitivity = 0.99      # P(positive | disease)
let specificity = 0.95      # P(negative | no disease)
let fpr = 1.0 - specificity # P(positive | no disease) = 0.05

# Total probability of testing positive
let p_positive = sensitivity * prevalence + fpr * (1.0 - prevalence)

# Positive Predictive Value (PPV)
let ppv = (sensitivity * prevalence) / p_positive

# Negative Predictive Value (NPV)
let p_negative = (1.0 - sensitivity) * prevalence + specificity * (1.0 - prevalence)
let npv = (specificity * (1.0 - prevalence)) / p_negative

print("=== Diagnostic Test Analysis ===")
print("Prevalence:    {prevalence}")
print("Sensitivity:   {sensitivity}")
print("Specificity:   {specificity}")
print("P(positive):   {p_positive:.4}")
print("PPV:           {ppv:.4} ({ppv * 100:.1}%)")
print("NPV:           {npv:.6} ({npv * 100:.4}%)")
print("")
print("Interpretation: A positive result means only a {ppv * 100:.1}% chance of disease.")
print("A negative result means a {npv * 100:.4}% chance of being disease-free.")
```

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="250" viewBox="0 0 660 250" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Diagnostic Test Flow: From Prevalence to Predictive Values</text>
  <defs>
    <marker id="arrowFlow" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#2563eb"/></marker>
  </defs>
  <!-- Step 1: Prevalence -->
  <rect x="30" y="55" width="120" height="60" rx="8" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <text x="90" y="78" text-anchor="middle" font-size="11" font-weight="bold" fill="#1e40af">PREVALENCE</text>
  <text x="90" y="95" text-anchor="middle" font-size="10" fill="#1e40af">P(Disease)</text>
  <text x="90" y="108" text-anchor="middle" font-size="9" fill="#3b82f6">Prior probability</text>
  <!-- Arrow -->
  <line x1="150" y1="85" x2="185" y2="85" stroke="#2563eb" stroke-width="2" marker-end="url(#arrowFlow)"/>
  <!-- Step 2: Test characteristics -->
  <rect x="190" y="40" width="140" height="90" rx="8" fill="#f0fdf4" stroke="#16a34a" stroke-width="2"/>
  <text x="260" y="60" text-anchor="middle" font-size="11" font-weight="bold" fill="#15803d">TEST PROPERTIES</text>
  <text x="260" y="80" text-anchor="middle" font-size="10" fill="#15803d">Sensitivity</text>
  <text x="260" y="94" text-anchor="middle" font-size="9" fill="#16a34a">P(+|Disease)</text>
  <text x="260" y="112" text-anchor="middle" font-size="10" fill="#15803d">Specificity</text>
  <text x="260" y="124" text-anchor="middle" font-size="9" fill="#16a34a">P(-|Healthy)</text>
  <!-- Arrow -->
  <line x1="330" y1="85" x2="365" y2="85" stroke="#2563eb" stroke-width="2" marker-end="url(#arrowFlow)"/>
  <!-- Bayes box -->
  <rect x="370" y="55" width="115" height="60" rx="30" fill="#faf5ff" stroke="#7c3aed" stroke-width="2.5"/>
  <text x="428" y="80" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">BAYES'</text>
  <text x="428" y="96" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">THEOREM</text>
  <!-- Arrow -->
  <line x1="485" y1="85" x2="515" y2="85" stroke="#2563eb" stroke-width="2" marker-end="url(#arrowFlow)"/>
  <!-- Step 3: Predictive Values -->
  <rect x="520" y="40" width="120" height="90" rx="8" fill="#fef2f2" stroke="#dc2626" stroke-width="2"/>
  <text x="580" y="60" text-anchor="middle" font-size="11" font-weight="bold" fill="#991b1b">PREDICTIVE</text>
  <text x="580" y="76" text-anchor="middle" font-size="11" font-weight="bold" fill="#991b1b">VALUES</text>
  <text x="580" y="96" text-anchor="middle" font-size="10" fill="#dc2626">PPV: P(Dis|+)</text>
  <text x="580" y="112" text-anchor="middle" font-size="10" fill="#dc2626">NPV: P(Heal|-)</text>
  <text x="580" y="124" text-anchor="middle" font-size="9" fill="#b91c1c">What the patient needs</text>
  <!-- Bottom key insight -->
  <rect x="60" y="155" width="540" height="80" rx="6" fill="#fffbeb" stroke="#f59e0b" stroke-width="1.5"/>
  <text x="330" y="176" text-anchor="middle" font-size="12" font-weight="bold" fill="#92400e">Key Insight: Prevalence Dominates PPV</text>
  <text x="330" y="196" text-anchor="middle" font-size="11" fill="#78350f">Rare disease (0.1%) + 99% sensitive test + 95% specific test = PPV of only 1.9%</text>
  <text x="330" y="214" text-anchor="middle" font-size="11" fill="#78350f">Common disease (50%) + same test = PPV of 95.2%</text>
  <text x="330" y="230" text-anchor="middle" font-size="10" fill="#92400e" font-style="italic">The same test accuracy means completely different things depending on prevalence</text>
</svg>
</div>

### How Prevalence Changes Everything

```bio
# Show how PPV changes with disease prevalence
let sensitivity = 0.99
let specificity = 0.95

let prevalences = [0.0001, 0.001, 0.01, 0.05, 0.10, 0.20, 0.50]

print("Prevalence | PPV")
print("-----------|--------")
for prev in prevalences {
    let fpr = 1.0 - specificity
    let p_pos = sensitivity * prev + fpr * (1.0 - prev)
    let ppv = (sensitivity * prev) / p_pos
    print("  {prev:.4}   | {ppv * 100:.1}%")
}
# At 0.01% prevalence: PPV = 0.2% (nearly all positives are false)
# At 50% prevalence: PPV = 95.2% (most positives are true)
```

This table is one of the most important results in clinical statistics. It explains why screening tests work well for common conditions but poorly for rare ones.

## Probability Distributions Revisited

On Day 3, we met distributions as shapes. Now we can understand them as probability functions.

### Discrete Distributions

For discrete random variables (counts, genotypes), the **probability mass function** (PMF) gives P(X = k) — the probability of observing exactly the value k.

```bio
# Binomial PMF: probability of exactly k successes in n trials
# Scenario: 4 children, mother is BRCA1 carrier (p = 0.5)
let n_children = 4
let p_inherit = 0.5

print("Number of children inheriting BRCA1:")
for k in 0..5 {
    let prob = dbinom(k, n_children, p_inherit)
    print("  {k} children: {prob:.4} ({prob * 100:.1}%)")
}
# 0: 6.25%, 1: 25.0%, 2: 37.5%, 3: 25.0%, 4: 6.25%
```

### Cumulative Distribution

The **cumulative distribution function** (CDF) gives P(X &le; k) — the probability of observing k or fewer.

```bio
# What's the probability that at most 1 of 4 children inherits the mutation?
let p_at_most_1 = pbinom(1, 4, 0.5)
print("P(0 or 1 child inherits): {p_at_most_1:.4}")  # 0.3125

# What's the probability at least 1 inherits?
let p_at_least_1 = 1.0 - pbinom(0, 4, 0.5)
print("P(at least 1 inherits): {p_at_least_1:.4}")  # 0.9375
```

### Continuous Distributions

For continuous random variables (gene expression, blood pressure), the **probability density function** (PDF) does not give P(X = k) (which is always zero for continuous variables). Instead, probabilities are computed over intervals using the CDF.

```bio
# Blood pressure: normal with mean 120, SD 15
let mu = 120.0
let sigma = 15.0

# P(BP > 140) — hypertension threshold
let p_hypertension = 1.0 - pnorm(140, mu, sigma)
print("P(BP > 140): {p_hypertension:.4}")  # ~0.0912

# P(100 < BP < 130)
let p_normal_range = pnorm(130, mu, sigma) - pnorm(100, mu, sigma)
print("P(100 < BP < 130): {p_normal_range:.4}")

# What BP value has only 5% of people above it?
let bp_95th = qnorm(0.95, mu, sigma)
print("95th percentile BP: {bp_95th:.1}")  # ~144.7
```

## Hardy-Weinberg as a Probability Model

Hardy-Weinberg equilibrium (HWE) is one of the most elegant applications of probability in genetics. For a biallelic locus with allele frequencies p (allele A) and q = 1-p (allele a), random mating produces genotypes with probabilities:

| Genotype | Frequency | If p = 0.3 |
|---|---|---|
| AA | p&sup2; | 0.09 |
| Aa | 2pq | 0.42 |
| aa | q&sup2; | 0.49 |

This model treats each allele transmission as an independent random event — like drawing two alleles from a bag with replacement.

```bio
# Test for Hardy-Weinberg equilibrium
# Observed genotype counts from a population study
let observed_AA = 45
let observed_Aa = 210
let observed_aa = 245
let total = observed_AA + observed_Aa + observed_aa  # 500

# Estimate allele frequencies
let p_A = (2.0 * observed_AA + observed_Aa) / (2.0 * total)
let p_a = 1.0 - p_A
print("Estimated allele frequencies: p(A) = {p_A:.3}, p(a) = {p_a:.3}")

# Expected counts under HWE
let expected_AA = p_A * p_A * total
let expected_Aa = 2.0 * p_A * p_a * total
let expected_aa = p_a * p_a * total
print("Expected: AA={expected_AA:.1}, Aa={expected_Aa:.1}, aa={expected_aa:.1}")
print("Observed: AA={observed_AA}, Aa={observed_Aa}, aa={observed_aa}")

# Chi-square test for HWE (we'll cover this formally on Day 13)
let chi2 = (observed_AA - expected_AA) ** 2 / expected_AA +
           (observed_Aa - expected_Aa) ** 2 / expected_Aa +
           (observed_aa - expected_aa) ** 2 / expected_aa
print("Chi-square statistic: {chi2:.3}")
print("Deviation from HWE: {if chi2 > 3.84 then 'Significant' else 'Not significant'}")
```

## Carrier Probability Calculations

Returning to Maria and David's consultation:

```bio
# Genetic counseling probability calculator

# Maria is a BRCA1 carrier (heterozygous)
# David is not a carrier (assumed)

# Autosomal dominant: 50% chance each child inherits
let p_inherit = 0.5

# They want 3 children
let n_children = 3

print("=== Genetic Counseling: BRCA1 Inheritance ===")
print("")

# Probability none of 3 children inherit
let p_none = dbinom(0, n_children, p_inherit)
print("P(no children inherit): {p_none:.4} ({p_none * 100:.1}%)")

# Probability exactly 1 inherits
let p_one = dbinom(1, n_children, p_inherit)
print("P(exactly 1 inherits):  {p_one:.4} ({p_one * 100:.1}%)")

# Probability at least 1 inherits
let p_at_least_one = 1.0 - p_none
print("P(at least 1 inherits): {p_at_least_one:.4} ({p_at_least_one * 100:.1}%)")

print("")
print("=== Conditional Cancer Risk ===")
# If a daughter inherits BRCA1, lifetime breast cancer risk ~ 72%
let p_cancer_given_brca = 0.72

# P(inherits AND develops cancer)
let p_inherit_and_cancer = p_inherit * p_cancer_given_brca
print("P(daughter inherits AND gets cancer): {p_inherit_and_cancer:.3} ({p_inherit_and_cancer * 100:.1}%)")

# P(daughter gets cancer by 70 | she is female)
# Must account for 50% chance of being female
let p_affected_daughter = 0.5 * p_inherit * p_cancer_given_brca
print("P(random child is affected daughter): {p_affected_daughter:.3} ({p_affected_daughter * 100:.1}%)")
```

## Python and R Equivalents

**Python:**
```python
from scipy import stats
import numpy as np

# Binomial probabilities
stats.binom.pmf(k=2, n=4, p=0.5)     # P(X = 2)
stats.binom.cdf(k=1, n=4, p=0.5)     # P(X <= 1)

# Normal probabilities
stats.norm.cdf(140, loc=120, scale=15) # P(X <= 140)
stats.norm.ppf(0.95, loc=120, scale=15) # 95th percentile

# Poisson
stats.poisson.pmf(k=5, mu=3.5)        # P(X = 5)
stats.poisson.cdf(k=9, mu=3.5)        # P(X <= 9)

# Bayes calculation
prevalence = 0.001
sensitivity = 0.99
fpr = 0.05
p_pos = sensitivity * prevalence + fpr * (1 - prevalence)
ppv = (sensitivity * prevalence) / p_pos
```

**R:**
```r
# Binomial
dbinom(2, size = 4, prob = 0.5)    # P(X = 2)
pbinom(1, size = 4, prob = 0.5)    # P(X <= 1)

# Normal
pnorm(140, mean = 120, sd = 15)    # P(X <= 140)
qnorm(0.95, mean = 120, sd = 15)   # 95th percentile

# Poisson
dpois(5, lambda = 3.5)             # P(X = 5)
ppois(9, lambda = 3.5)             # P(X <= 9)

# Bayes
prevalence <- 0.001
sensitivity <- 0.99
fpr <- 0.05
p_pos <- sensitivity * prevalence + fpr * (1 - prevalence)
ppv <- (sensitivity * prevalence) / p_pos
```

## Exercises

### Exercise 1: The Prenatal Test

A prenatal screening test for Down syndrome has sensitivity 95% and specificity 97%. The prevalence of Down syndrome is approximately 1 in 700 live births for a 30-year-old mother.

```bio
# TODO: Compute the PPV — if the test is positive, what is the true probability?
# TODO: Compute the NPV — if the test is negative, how reassuring is it?
# TODO: How does the PPV change for a 40-year-old mother (prevalence ~1 in 100)?
let prevalence_30 = 1.0 / 700.0
let prevalence_40 = 1.0 / 100.0
let sensitivity = 0.95
let specificity = 0.97
```

### Exercise 2: Multiple Independent Events

A patient takes three independent diagnostic tests for a condition. Each test has 90% sensitivity.

```bio
# TODO: What is P(all three tests are positive | disease)?
# TODO: What is P(at least one test is negative | disease)?
# TODO: What is P(all three tests are negative | disease)?
# TODO: If the patient tests positive on all three, and prevalence is 1%,
#       what is the updated probability they have the disease?
let sensitivity = 0.90
let specificity = 0.95
let prevalence = 0.01
```

### Exercise 3: Carrier Frequency

Cystic fibrosis is autosomal recessive with a carrier frequency of approximately 1 in 25 among Europeans.

```bio
let carrier_freq = 1.0 / 25.0

# TODO: What is P(both parents are carriers)?
# TODO: If both are carriers, P(affected child)?
# TODO: P(random European couple has an affected child)?
# TODO: If one parent is a confirmed carrier and the other is untested,
#       what is P(their child is affected)?
```

### Exercise 4: Sequencing Error

A variant caller reports a SNV at a position covered by 50 reads. 5 reads support the variant allele.

```bio
# Is this a real variant or sequencing error?
# Assume sequencing error rate = 0.01 per base per read

let n_reads = 50
let n_alt = 5
let error_rate = 0.01

# TODO: Under the null (all errors), what is P(5+ alt reads)?
# Use the binomial: P(X >= 5 | n=50, p=0.01) = 1 - pbinom(4, 50, 0.01)
# TODO: Is this consistent with error alone, or likely a real variant?
```

## Key Takeaways

- **Probability** quantifies uncertainty on a 0-to-1 scale. It is the mathematical language of statistics.
- The **addition rule** handles "or" questions; the **multiplication rule** handles "and" questions. Whether events are mutually exclusive or independent changes the formula.
- **Conditional probability** P(A|B) is NOT the same as P(B|A). Confusing them is the source of the prosecutor's fallacy and misinterpretation of diagnostic tests.
- **Bayes' theorem** is the bridge from P(B|A) to P(A|B). It shows that a positive test for a rare disease usually means the patient is healthy — the positive predictive value depends critically on prevalence.
- In genetics, probability models like **Hardy-Weinberg equilibrium** and **Mendelian inheritance** translate directly into binomial probability calculations.
- Always compute both **PPV and NPV** when interpreting diagnostic or screening tests. Sensitivity and specificity alone are insufficient.

## What's Next

Today you learned to compute probabilities for individual events. But in real experiments, you do not observe single events — you observe samples drawn from populations. Tomorrow, on Day 5, we tackle the crucial question of sampling: Why does sample size matter so much? You will see the Central Limit Theorem in action — watching the distribution of sample means magically approach normality even when the underlying data is wildly skewed. You will understand what "statistical power" means and why a study with 20 patients per arm is almost guaranteed to fail. Day 5 is the bridge between probability theory and the practical reality of experimental design.
