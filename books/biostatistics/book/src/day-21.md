# Day 21: Dimensionality Reduction — PCA and Friends

<div class="day-meta">
<span class="badge">Day 21 of 30</span>
<span class="badge">Prerequisites: Days 2-3, 13, 20</span>
<span class="badge">~60 min reading</span>
<span class="badge">Unsupervised Learning</span>
</div>

## The Problem

A single-cell RNA sequencing experiment has just finished. Your collaborator drops a matrix on your desk: expression levels for 20,000 genes measured across 5,000 individual cells. She wants to know whether the cells form distinct populations — immune subtypes, perhaps, or tumor cells versus stroma.

You stare at the matrix. Twenty thousand dimensions. You cannot visualize it. You cannot eyeball it. You cannot plot 20,000 axes on a screen. If you pick two genes at random and make a scatter plot, you might miss the structure entirely — those two genes might be irrelevant. Pick different genes and you get a completely different picture.

What you need is a camera angle. A way to look at 20,000-dimensional data from the direction that reveals the most structure. A method that compresses the information into a handful of dimensions you can actually see and reason about — without throwing away the patterns that matter.

That method is Principal Component Analysis. It is the single most widely used technique in genomics for exploring high-dimensional data, and by the end of today, you will understand exactly how it works, when it fails, and how to use it effectively.

## What Is Dimensionality Reduction?

Dimensionality reduction takes data with many variables (dimensions) and represents it using fewer variables, while preserving as much of the important structure as possible.

Think of it this way. You are standing on a hilltop overlooking a city. The city exists in three dimensions, but you are looking at it from one particular angle. Your view — a photograph — is a two-dimensional representation of a three-dimensional scene. A good photograph, taken from the right angle, captures the layout of the streets, the relative positions of buildings, and the overall structure. A bad photograph, taken facing a blank wall, tells you nothing.

Dimensionality reduction is the art of finding the best camera angle for your data. In a 20,000-dimensional gene expression dataset, the "best angle" is the one that shows you the most variation — the direction along which cells differ the most.

> **Key insight:** Dimensionality reduction does not create information. It finds the most informative low-dimensional summary of high-dimensional data. If the real structure is inherently high-dimensional, no reduction will capture it perfectly.

## The Curse of Dimensionality

Before we solve the problem, let us understand why high dimensions are problematic.

In one dimension, 10 evenly spaced points cover a line segment well. In two dimensions, you need 100 points (10 x 10) to cover a square with the same density. In three dimensions, 1,000 points (10 x 10 x 10). In 20,000 dimensions, you would need 10^20,000 points — a number so large it dwarfs the number of atoms in the observable universe.

This means that in high-dimensional space, data is always sparse. Your 5,000 cells are scattered across 20,000-dimensional space like five thousand grains of sand in the Sahara. Most of the space is empty. Distances between points become unreliable — in very high dimensions, the nearest neighbor and the farthest neighbor are almost the same distance away.

| Dimensions | Points needed for even coverage | Nearest-neighbor reliability |
|---|---|---|
| 2 | 100 | Excellent |
| 10 | 10 billion | Good |
| 100 | 10^100 | Poor |
| 20,000 | 10^20,000 | Meaningless |

This is the curse of dimensionality. It makes direct analysis of raw high-dimensional data unreliable. Fortunately, biological data has a saving grace: most of the 20,000 gene dimensions are redundant. Genes in the same pathway are correlated. Housekeeping genes barely vary. The "true" dimensionality of gene expression data is typically much lower than the number of genes measured — perhaps tens to hundreds of effective dimensions. PCA exploits this redundancy.

## PCA Mechanics: Finding the Best Camera Angle

PCA proceeds in a simple sequence of steps.

### Step 1: Center the Data

Subtract the mean of each gene across all cells. This ensures we are looking at variation, not absolute levels. A gene expressed at 10,000 in every cell has zero variation and should contribute nothing.

### Step 2: Find the Direction of Maximum Variance

Imagine all 5,000 cells as points in 20,000-dimensional space. PCA finds the single direction (a line through the origin) along which the spread of points is greatest. This is Principal Component 1 (PC1). It is the camera angle that shows you the most variation.

Mathematically, PC1 is the eigenvector of the covariance matrix with the largest eigenvalue. But you do not need to understand eigenvectors to use PCA — think of it as the axis along which the data is most stretched.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="420" viewBox="0 0 650 420" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <defs>
    <marker id="arrowPC1" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"><polygon points="0 0, 10 3.5, 0 7" fill="#2563eb"/></marker>
    <marker id="arrowPC2" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"><polygon points="0 0, 10 3.5, 0 7" fill="#dc2626"/></marker>
  </defs>
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">PCA: Finding the Axes of Maximum Variance</text>
  <!-- Data cloud (elliptical scatter) -->
  <g opacity="0.55">
    <circle cx="280" cy="250" r="5" fill="#93c5fd"/><circle cx="310" cy="230" r="5" fill="#93c5fd"/>
    <circle cx="350" cy="210" r="5" fill="#93c5fd"/><circle cx="260" cy="260" r="5" fill="#93c5fd"/>
    <circle cx="330" cy="220" r="5" fill="#93c5fd"/><circle cx="370" cy="195" r="5" fill="#93c5fd"/>
    <circle cx="290" cy="245" r="5" fill="#93c5fd"/><circle cx="400" cy="180" r="5" fill="#93c5fd"/>
    <circle cx="240" cy="275" r="5" fill="#93c5fd"/><circle cx="320" cy="235" r="5" fill="#93c5fd"/>
    <circle cx="360" cy="205" r="5" fill="#93c5fd"/><circle cx="250" cy="268" r="5" fill="#93c5fd"/>
    <circle cx="380" cy="188" r="5" fill="#93c5fd"/><circle cx="270" cy="258" r="5" fill="#93c5fd"/>
    <circle cx="340" cy="215" r="5" fill="#93c5fd"/><circle cx="305" cy="240" r="5" fill="#93c5fd"/>
    <circle cx="390" cy="185" r="5" fill="#93c5fd"/><circle cx="335" cy="225" r="5" fill="#93c5fd"/>
    <circle cx="295" cy="248" r="5" fill="#93c5fd"/><circle cx="315" cy="232" r="5" fill="#93c5fd"/>
    <circle cx="355" cy="200" r="5" fill="#93c5fd"/><circle cx="275" cy="255" r="5" fill="#93c5fd"/>
    <circle cx="345" cy="218" r="5" fill="#93c5fd"/><circle cx="230" cy="280" r="5" fill="#93c5fd"/>
    <circle cx="410" cy="175" r="5" fill="#93c5fd"/><circle cx="265" cy="263" r="5" fill="#93c5fd"/>
    <circle cx="385" cy="190" r="5" fill="#93c5fd"/><circle cx="300" cy="242" r="5" fill="#93c5fd"/>
    <circle cx="325" cy="228" r="5" fill="#93c5fd"/><circle cx="375" cy="198" r="5" fill="#93c5fd"/>
    <circle cx="255" cy="270" r="5" fill="#93c5fd"/><circle cx="420" cy="170" r="5" fill="#93c5fd"/>
    <circle cx="220" cy="288" r="5" fill="#93c5fd"/><circle cx="365" cy="202" r="5" fill="#93c5fd"/>
  </g>
  <!-- PC1 axis (direction of max variance) -->
  <line x1="180" y1="305" x2="460" y2="155" stroke="#2563eb" stroke-width="3" marker-end="url(#arrowPC1)"/>
  <text x="470" y="152" font-size="14" font-weight="bold" fill="#2563eb">PC1</text>
  <text x="468" y="168" font-size="11" fill="#2563eb">(max variance)</text>
  <!-- PC2 axis (perpendicular) -->
  <line x1="255" y1="160" x2="395" y2="310" stroke="#dc2626" stroke-width="3" marker-end="url(#arrowPC2)"/>
  <text x="400" y="320" font-size="14" font-weight="bold" fill="#dc2626">PC2</text>
  <text x="398" y="336" font-size="11" fill="#dc2626">(perpendicular)</text>
  <!-- Center point -->
  <circle cx="320" cy="228" r="6" fill="#1e293b" stroke="white" stroke-width="2"/>
  <text x="328" y="224" font-size="11" fill="#1e293b">center</text>
  <!-- Spread annotation for PC1 -->
  <line x1="200" y1="350" x2="440" y2="350" stroke="#2563eb" stroke-width="2"/>
  <line x1="200" y1="343" x2="200" y2="357" stroke="#2563eb" stroke-width="2"/>
  <line x1="440" y1="343" x2="440" y2="357" stroke="#2563eb" stroke-width="2"/>
  <text x="320" y="370" text-anchor="middle" font-size="12" fill="#2563eb">Large spread along PC1</text>
  <!-- Spread annotation for PC2 -->
  <line x1="140" y1="195" x2="140" y2="275" stroke="#dc2626" stroke-width="2"/>
  <line x1="133" y1="195" x2="147" y2="195" stroke="#dc2626" stroke-width="2"/>
  <line x1="133" y1="275" x2="147" y2="275" stroke="#dc2626" stroke-width="2"/>
  <text x="125" y="240" text-anchor="middle" font-size="11" fill="#dc2626" transform="rotate(-90 125 240)">Small spread</text>
  <!-- Legend -->
  <rect x="20" y="385" width="610" height="25" rx="4" fill="#f1f5f9"/>
  <circle cx="40" cy="397" r="5" fill="#93c5fd" opacity="0.55"/>
  <text x="52" y="401" font-size="11" fill="#6b7280">Data points</text>
  <line x1="130" y1="397" x2="160" y2="397" stroke="#2563eb" stroke-width="2"/>
  <text x="168" y="401" font-size="11" fill="#6b7280">PC1 (most variance)</text>
  <line x1="310" y1="397" x2="340" y2="397" stroke="#dc2626" stroke-width="2"/>
  <text x="348" y="401" font-size="11" fill="#6b7280">PC2 (perpendicular, next most)</text>
</svg>
</div>

### Step 3: Find the Next Perpendicular Direction

PC2 is the direction of maximum remaining variance, with the constraint that it must be perpendicular (orthogonal) to PC1. This ensures PC2 captures new information, not a rehash of PC1.

### Step 4: Repeat

PC3 is perpendicular to both PC1 and PC2, and captures the next most variance. Continue for as many components as you want (up to the number of samples or genes, whichever is smaller).

Each successive PC captures less variance. The first few PCs often capture the majority of the total variation, and the rest is noise.

| Component | Captures | Constraint |
|---|---|---|
| PC1 | Maximum variance in the data | None (first direction) |
| PC2 | Maximum remaining variance | Perpendicular to PC1 |
| PC3 | Maximum remaining variance | Perpendicular to PC1 and PC2 |
| ... | Decreasing variance | Perpendicular to all previous |

### The Camera Analogy

If your data were a 3D object:
- PC1 is like looking at the object from the angle where its shadow is largest
- PC2 is the perpendicular angle that shows the next most detail
- PC3 fills in the remaining depth

For gene expression: PC1 might separate tumor from normal. PC2 might separate tissue subtypes. PC3 might reflect a batch effect. Each component tells a different biological or technical story.

## Eigenvalues and Variance Explained

Each principal component has an associated eigenvalue. The eigenvalue tells you how much variance that component captures. Dividing each eigenvalue by the total gives the proportion of variance explained.

| Component | Eigenvalue | Variance Explained | Cumulative |
|---|---|---|---|
| PC1 | 85.3 | 28.4% | 28.4% |
| PC2 | 42.1 | 14.0% | 42.4% |
| PC3 | 18.7 | 6.2% | 48.7% |
| PC4 | 12.4 | 4.1% | 52.8% |
| ... | ... | ... | ... |
| PC20 | 2.1 | 0.7% | 75.2% |

In a typical scRNA-seq dataset, the first 20-50 PCs might capture 70-80% of the total variance. The remaining thousands of PCs together account for the other 20-30% — mostly noise.

> **Key insight:** If the first 2 PCs capture 60%+ of the variance, a 2D PCA plot is a good representation. If they capture only 15%, the data has complex structure that two dimensions cannot convey.

## The Scree Plot: Finding the Elbow

A scree plot displays the eigenvalue (or variance explained) for each successive PC. It is named after the geological term for rubble at the base of a cliff — because the plot typically shows a steep drop followed by a long, flat tail.

The "elbow" — the point where the curve transitions from steep to flat — suggests how many PCs contain real signal. Components before the elbow capture structured variation; components after the elbow capture noise.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="340" viewBox="0 0 650 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Scree Plot: Eigenvalues by Principal Component</text>
  <!-- Axes -->
  <line x1="80" y1="280" x2="600" y2="280" stroke="#9ca3af" stroke-width="1.5"/>
  <line x1="80" y1="280" x2="80" y2="50" stroke="#9ca3af" stroke-width="1.5"/>
  <text x="340" y="315" text-anchor="middle" font-size="13" fill="#6b7280">Principal Component</text>
  <text x="25" y="165" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90 25 165)">Eigenvalue</text>
  <!-- Y-axis ticks -->
  <text x="72" y="284" text-anchor="end" font-size="11" fill="#9ca3af">0</text>
  <text x="72" y="234" text-anchor="end" font-size="11" fill="#9ca3af">20</text>
  <text x="72" y="184" text-anchor="end" font-size="11" fill="#9ca3af">40</text>
  <text x="72" y="134" text-anchor="end" font-size="11" fill="#9ca3af">60</text>
  <text x="72" y="84" text-anchor="end" font-size="11" fill="#9ca3af">80</text>
  <line x1="77" y1="230" x2="83" y2="230" stroke="#9ca3af"/>
  <line x1="77" y1="180" x2="83" y2="180" stroke="#9ca3af"/>
  <line x1="77" y1="130" x2="83" y2="130" stroke="#9ca3af"/>
  <line x1="77" y1="80" x2="83" y2="80" stroke="#9ca3af"/>
  <!-- Bars: eigenvalues dropping sharply then flattening -->
  <rect x="95" y="68" width="32" height="212" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="140" y="130" width="32" height="150" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="185" y="185" width="32" height="95" rx="2" fill="#3b82f6" opacity="0.8"/>
  <rect x="230" y="218" width="32" height="62" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="275" y="238" width="32" height="42" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="320" y="250" width="32" height="30" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="365" y="257" width="32" height="23" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="410" y="262" width="32" height="18" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="455" y="265" width="32" height="15" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="500" y="268" width="32" height="12" rx="2" fill="#93c5fd" opacity="0.8"/>
  <rect x="545" y="270" width="32" height="10" rx="2" fill="#93c5fd" opacity="0.8"/>
  <!-- X-axis labels -->
  <text x="111" y="296" text-anchor="middle" font-size="11" fill="#6b7280">1</text>
  <text x="156" y="296" text-anchor="middle" font-size="11" fill="#6b7280">2</text>
  <text x="201" y="296" text-anchor="middle" font-size="11" fill="#6b7280">3</text>
  <text x="246" y="296" text-anchor="middle" font-size="11" fill="#6b7280">4</text>
  <text x="291" y="296" text-anchor="middle" font-size="11" fill="#6b7280">5</text>
  <text x="336" y="296" text-anchor="middle" font-size="11" fill="#6b7280">6</text>
  <text x="381" y="296" text-anchor="middle" font-size="11" fill="#6b7280">7</text>
  <text x="426" y="296" text-anchor="middle" font-size="11" fill="#6b7280">8</text>
  <text x="471" y="296" text-anchor="middle" font-size="11" fill="#6b7280">9</text>
  <text x="516" y="296" text-anchor="middle" font-size="11" fill="#6b7280">10</text>
  <text x="561" y="296" text-anchor="middle" font-size="11" fill="#6b7280">11</text>
  <!-- Elbow annotation -->
  <line x1="228" y1="50" x2="228" y2="215" stroke="#dc2626" stroke-width="2" stroke-dasharray="6,4"/>
  <text x="228" y="44" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Elbow</text>
  <!-- Signal vs noise regions -->
  <rect x="85" y="38" width="140" height="18" rx="3" fill="#2563eb" opacity="0.12"/>
  <text x="155" y="51" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">Signal</text>
  <rect x="228" y="38" width="360" height="18" rx="3" fill="#9ca3af" opacity="0.12"/>
  <text x="408" y="51" text-anchor="middle" font-size="11" fill="#9ca3af">Noise floor</text>
</svg>
</div>

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="320" viewBox="0 0 650 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Cumulative Variance Explained</text>
  <!-- Axes -->
  <line x1="80" y1="270" x2="600" y2="270" stroke="#9ca3af" stroke-width="1.5"/>
  <line x1="80" y1="270" x2="80" y2="50" stroke="#9ca3af" stroke-width="1.5"/>
  <text x="340" y="300" text-anchor="middle" font-size="13" fill="#6b7280">Number of PCs retained</text>
  <text x="25" y="160" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90 25 160)">Cumulative Variance (%)</text>
  <!-- Y-axis labels -->
  <text x="72" y="274" text-anchor="end" font-size="11" fill="#9ca3af">0%</text>
  <text x="72" y="219" text-anchor="end" font-size="11" fill="#9ca3af">25%</text>
  <text x="72" y="164" text-anchor="end" font-size="11" fill="#9ca3af">50%</text>
  <text x="72" y="109" text-anchor="end" font-size="11" fill="#9ca3af">75%</text>
  <text x="72" y="54" text-anchor="end" font-size="11" fill="#9ca3af">100%</text>
  <!-- Grid lines -->
  <line x1="80" y1="215" x2="600" y2="215" stroke="#e5e7eb" stroke-width="0.5"/>
  <line x1="80" y1="160" x2="600" y2="160" stroke="#e5e7eb" stroke-width="0.5"/>
  <line x1="80" y1="105" x2="600" y2="105" stroke="#e5e7eb" stroke-width="0.5"/>
  <!-- Cumulative line: rises steeply then flattens -->
  <!-- Values: 28%, 42%, 49%, 53%, 57%, 60%, 63%, 65%, 67%, 69%, 71%, 73%, 75%, 77%, 79%, 81%, 83%, 85%, 87%, 89% -->
  <polyline points="111,208 156,178 201,162 246,154 291,145 336,138 381,132 426,127 471,123 516,119 561,115" fill="none" stroke="#2563eb" stroke-width="2.5"/>
  <circle cx="111" cy="208" r="4" fill="#2563eb"/>
  <circle cx="156" cy="178" r="4" fill="#2563eb"/>
  <circle cx="201" cy="162" r="4" fill="#2563eb"/>
  <circle cx="246" cy="154" r="4" fill="#2563eb"/>
  <circle cx="291" cy="145" r="4" fill="#2563eb"/>
  <circle cx="336" cy="138" r="4" fill="#2563eb"/>
  <circle cx="381" cy="132" r="4" fill="#2563eb"/>
  <circle cx="426" cy="127" r="4" fill="#2563eb"/>
  <circle cx="471" cy="123" r="4" fill="#2563eb"/>
  <circle cx="516" cy="119" r="4" fill="#2563eb"/>
  <circle cx="561" cy="115" r="4" fill="#2563eb"/>
  <!-- X-axis labels -->
  <text x="111" y="286" text-anchor="middle" font-size="11" fill="#6b7280">1</text>
  <text x="156" y="286" text-anchor="middle" font-size="11" fill="#6b7280">2</text>
  <text x="201" y="286" text-anchor="middle" font-size="11" fill="#6b7280">3</text>
  <text x="246" y="286" text-anchor="middle" font-size="11" fill="#6b7280">4</text>
  <text x="291" y="286" text-anchor="middle" font-size="11" fill="#6b7280">5</text>
  <text x="336" y="286" text-anchor="middle" font-size="11" fill="#6b7280">6</text>
  <text x="381" y="286" text-anchor="middle" font-size="11" fill="#6b7280">7</text>
  <text x="426" y="286" text-anchor="middle" font-size="11" fill="#6b7280">8</text>
  <text x="471" y="286" text-anchor="middle" font-size="11" fill="#6b7280">9</text>
  <text x="516" y="286" text-anchor="middle" font-size="11" fill="#6b7280">10</text>
  <text x="561" y="286" text-anchor="middle" font-size="11" fill="#6b7280">11</text>
  <!-- 80% threshold line -->
  <line x1="80" y1="94" x2="600" y2="94" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="8,4"/>
  <text x="608" y="98" font-size="12" fill="#dc2626">80%</text>
  <!-- Drop line from threshold to x-axis -->
  <line x1="471" y1="94" x2="471" y2="270" stroke="#16a34a" stroke-width="1.5" stroke-dasharray="4,4"/>
  <circle cx="471" cy="123" r="6" fill="none" stroke="#16a34a" stroke-width="2"/>
  <text x="480" y="88" font-size="12" font-weight="bold" fill="#16a34a">9 PCs needed</text>
  <!-- Shaded area under curve -->
  <polygon points="80,270 111,208 156,178 201,162 246,154 291,145 336,138 381,132 426,127 471,123 471,270" fill="#2563eb" opacity="0.08"/>
</svg>
</div>

```bio
set_seed(42)
# Generate scree plot for gene expression data
let pca_result = pca(expression_matrix)
let eigenvalues = pca_result.variance_explained
  |> map_index(|i, v| {pc: i + 1, variance: v})
  |> to_table()
plot(eigenvalues, {type: "line", x: "pc", y: "variance",
  title: "Scree Plot — Gene Expression PCA"})
```

Rules of thumb for the elbow:
- If there is a clear elbow at PC 3, use 3 components
- If the decline is gradual, no clean cutoff exists — try cumulative variance (e.g., keep PCs until 80% variance)
- In scRNA-seq, 20-50 PCs are commonly retained for downstream clustering

## PCA Biplots: Samples and Loadings Together

A PCA biplot shows two things simultaneously:

1. **Scores** (points): Each sample projected onto PC1 and PC2. Samples that cluster together are similar.
2. **Loadings** (arrows): Each variable's contribution to PC1 and PC2. Long arrows indicate influential variables. The direction shows which PC they load on.

In genomics, the scores are your cells (or samples) and the loadings are your genes. Genes with long arrows pointing toward a cluster of cells are the genes that define that cluster's identity.

```bio
# PCA biplot with top gene loadings
let result = pca(expression_matrix)
pca_plot(result, {title: "PCA Biplot — Top 10 Gene Loadings"})
```

### Interpreting Loadings

| Loading Direction | Interpretation |
|---|---|
| Strong positive on PC1 | Gene is highly expressed in cells on the right side of the plot |
| Strong negative on PC1 | Gene is highly expressed in cells on the left side |
| Strong on PC2, weak on PC1 | Gene distinguishes top vs bottom but not left vs right |
| Near the origin | Gene contributes little — low variance or uncorrelated with PCs |

> **Common pitfall:** PCA loadings tell you which genes drive each component, but the sign is arbitrary. PC1 could point in either direction — what matters is the relative positioning, not the absolute sign.

## Which Genes Drive PC1?

After running PCA, a common next step is to extract the genes with the largest (absolute) loadings on PC1 and PC2. These are the genes responsible for the dominant patterns of variation.

```bio
set_seed(42)
# Run PCA on 500-gene by 2000-cell expression matrix
let col_names = seq(1, 500) |> map(|i| "Gene_" + str(i))
let expr = table(2000, col_names, "rnorm")

# Inject structure: first 1000 cells get higher expression of genes 1-50
for i in 0..1000 {
  for j in 0..50 {
    expr[i][j] = expr[i][j] + 3.0
  }
}

let result = pca(expr)

# Top 10 genes driving PC1
let pc1_loadings = result.loadings[0]
  |> sort_by(|x| -abs(x.value))
  |> take(10)

print("Top genes on PC1:")
print(pc1_loadings)
```

## When PCA Fails

PCA assumes that the most important structure lies along directions of maximum variance. This fails in several situations:

### Non-linear Structure

If cells lie along a curved trajectory (like a differentiation path), PCA will smear the curve into a blob. The maximum-variance direction might cut across the curve rather than follow it. Methods like t-SNE and UMAP handle non-linear structure better, but they do not preserve distances — only local neighborhoods.

### Outliers Dominate

A single extreme outlier can hijack PC1, making it the "outlier direction" rather than the biologically interesting direction. Always check for outliers before interpreting PCA.

### Batch Effects Are Stronger Than Biology

If batch effects (Day 20) contribute more variance than biological signal, PC1 will separate batches, not biology. This is actually useful — PCA is one of the best tools for detecting batch effects. But you must correct them before interpreting the biology.

### The Variance Is Uninteresting

PCA finds the direction of maximum variance, not maximum biological interest. If the biggest source of variation is sequencing depth (a technical factor), PC1 will capture that, and you will need to look at PC2 or PC3 for biology.

> **Clinical relevance:** In clinical genomics, PCA on genotype data reveals population structure. The first two PCs of human genetic variation separate continental ancestry groups. Failing to account for this in a GWAS leads to spurious associations — a gene might appear associated with disease simply because both the gene variant and the disease are more common in one ancestry group.

## PCA in BioLang

```bio
set_seed(42)
# Complete PCA analysis pipeline

# 1. Load expression matrix (500 genes x 2000 cells)
let n_cells = 2000
let n_genes = 500

# Simulate two cell populations
let group_a = table(1000, n_genes, "rnorm")
let group_b = table(1000, n_genes, "rnorm")

# Group B has elevated expression in genes 1-80
for i in 0..1000 {
  for j in 0..80 {
    group_b[i][j] = group_b[i][j] + 2.5
  }
}

let expr = rbind(group_a, group_b)
let labels = repeat("A", 1000) + repeat("B", 1000)

# 2. Run PCA
let result = pca(expr)

# 3. Scree plot — find the elbow
let eigenvalues = result.variance_explained
  |> take(20)
  |> map_index(|i, v| {pc: i + 1, variance: v})
  |> to_table()
plot(eigenvalues, {type: "line", x: "pc", y: "variance",
  title: "Scree Plot"})

# 4. Variance explained
print("PC1 variance explained: " + str(result.variance_explained[0]))
print("PC2 variance explained: " + str(result.variance_explained[1]))
print("Cumulative (PC1+PC2): " + str(
  result.variance_explained[0] + result.variance_explained[1]
))

# 5. PCA scatter plot colored by group
scatter(result.scores[0], result.scores[1])

# 6. PCA plot with gene loadings
pca_plot(result, {title: "PCA Biplot — Top 15 Genes"})

# 7. Extract top genes per PC
let top_pc1 = result.loadings[0]
  |> sort_by(|x| -abs(x.value))
  |> take(10)

let top_pc2 = result.loadings[1]
  |> sort_by(|x| -abs(x.value))
  |> take(10)

print("Top 10 genes on PC1:")
for gene in top_pc1 {
  print("  " + gene.name + ": " + str(round(gene.value, 4)))
}

# 8. Transform new data into PC space
let new_cells = table(50, n_genes, "rnorm")
let projected = pca_transform(result, new_cells)
print("Projected new cells: " + str(nrow(projected)) + " x " + str(ncol(projected)))
```

**Python:**

```python
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler
import matplotlib.pyplot as plt
import numpy as np

scaler = StandardScaler()
X_scaled = scaler.fit_transform(expr)
pca = PCA(n_components=20)
scores = pca.fit_transform(X_scaled)

# Scree plot
plt.plot(range(1, 21), pca.explained_variance_ratio_, 'bo-')
plt.xlabel('Component')
plt.ylabel('Variance Explained')
plt.show()

# Scatter
plt.scatter(scores[:, 0], scores[:, 1], c=labels, cmap='Set1', alpha=0.5)
plt.xlabel(f'PC1 ({pca.explained_variance_ratio_[0]*100:.1f}%)')
plt.ylabel(f'PC2 ({pca.explained_variance_ratio_[1]*100:.1f}%)')
plt.show()

# Top loadings
loadings = pca.components_[0]
top_idx = np.argsort(np.abs(loadings))[::-1][:10]
```

**R:**

```r
pca_result <- prcomp(expr, scale. = TRUE)
summary(pca_result)

# Scree plot
screeplot(pca_result, npcs = 20, type = "lines")

# Scatter
plot(pca_result$x[,1], pca_result$x[,2], col = labels, pch = 19,
     xlab = paste0("PC1 (", round(summary(pca_result)$importance[2,1]*100, 1), "%)"),
     ylab = paste0("PC2 (", round(summary(pca_result)$importance[2,2]*100, 1), "%)"))

# Biplot
biplot(pca_result)

# Top loadings on PC1
sort(abs(pca_result$rotation[,1]), decreasing = TRUE)[1:10]
```

## Exercises

1. **Scree plot interpretation.** Generate a 100-gene x 500-sample matrix with three hidden groups (shift different gene blocks for each group). Run PCA and create a scree plot. How many PCs have eigenvalues clearly above the noise floor? Does this match the number of groups you created?

```bio
# Create three groups with different gene signatures
# Your code here: build the matrix, run pca(), plot variance explained
```

2. **Loading detective.** Run PCA on the three-group data from Exercise 1. Extract the top 10 genes on PC1 and PC2. Do the gene indices match the blocks you shifted? Create a biplot to visualize.

```bio
# Your code here: extract loadings, identify driving genes
# pca_plot(result, {title: "Biplot"})
```

3. **Outlier hijacking.** Take the matrix from Exercise 1 and add a single extreme outlier cell (all genes set to 100). Re-run PCA. What happens to PC1? Remove the outlier and compare.

```bio
# Your code here: add outlier, run pca(), compare scree plots
```

4. **Batch effect detection.** Create a matrix with two biological groups AND a batch effect (add 2.0 to all genes in half the samples, crossing the biological groups). Run PCA. Which PC captures the batch effect? Which captures biology?

```bio
# Your code here: simulate batch + biology, examine PC1 vs PC2
```

5. **Variance threshold.** How many PCs do you need to capture 80% of the variance in the three-group dataset? Write code to find this number automatically.

```bio
# Your code here: cumulative variance explained, find threshold
```

## Key Takeaways

- PCA finds the directions of maximum variance in high-dimensional data, producing a low-dimensional summary that preserves the most important structure.
- Each principal component captures progressively less variance and is perpendicular to all previous components.
- The scree plot shows how much variance each PC captures; the "elbow" suggests how many PCs contain real signal.
- PCA biplots display both samples (as points) and variable loadings (as arrows), revealing which genes drive the observed patterns.
- PCA assumes linear structure — it fails on curved trajectories, is sensitive to outliers, and will capture the largest source of variance whether it is biological or technical.
- In genomics, PCA is essential for quality control (detecting batch effects and outliers), population structure analysis (GWAS), and dimensionality reduction before clustering (scRNA-seq).
- Always examine what PC1 actually represents before interpreting it as biology — it might be a technical artifact.

## What's Next

Tomorrow we take the reduced data from PCA and ask: are there natural groups? Clustering methods — k-means, hierarchical, and DBSCAN — will find structure in your omics data, identify tumor subtypes, and reveal cellular populations. But beware: clustering always finds clusters, even in pure noise. You will learn how to tell real structure from statistical ghosts.
