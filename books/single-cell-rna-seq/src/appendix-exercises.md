# Exercises and Answers

## Exercises

### 1. Experimental units

Four mice per condition each contribute 3,000 cells. What is the sample size
for a condition comparison? What does the large cell count improve?

### 2. Matrix orientation

A 10x Matrix Market header reports 20,000 rows and 5,000 columns. What do those
usually represent? What orientation does `sc.load()` expose?

### 3. QC sensitivity

Run the teaching workflow with maximum mitochondrial percentages of `3.0`,
`5.0`, and `10.0`. Record retained cells, clusters, and ARI.

### 4. Resolution sensitivity

Use `sc.sweep()` across `[0.2, 0.5, 0.8, 1.2]`. Which result has the most
clusters? Does more detail automatically mean more truth?

### 5. Annotation

For one real cluster, prepare a table of positive markers, negative markers,
state programs, reference evidence, and uncertainty. Which label is the
narrowest one the evidence supports?

### 6. Validation

Why can BioLang cluster `0` correspond to Scanpy cluster `2` while ARI remains
1.0?

### 7. Differential expression

Write a pseudobulk table design for six patients measured before and after
treatment. What are the rows, columns, pairing variable, and primary contrast?

### 8. Clinical claim

Rewrite "Cluster 4 proves this patient will respond to therapy" as an accurate
exploratory statement.

## Answers

### 1

The independent sample size is four mice per condition. More cells improve the
description of within-mouse heterogeneity and rare populations, not the number
of independently treated mice.

### 2

Rows usually represent genes/features and columns represent barcodes/cells.
`sc.load()` exposes a cells-by-genes matrix.

### 3

The exact numbers are empirical. The important answer is the recorded change
and whether removed cells concentrate in a population. A threshold is not
justified because it gives the preferred ARI.

### 4

Higher resolution generally creates more clusters. More clusters can split
states or technical variation and does not automatically improve biological
truth.

### 5

The answer must use multiple genes and conflicting evidence. `unknown` or a
broad lineage is acceptable when evidence is insufficient.

### 6

Cluster IDs are arbitrary labels. ARI compares co-membership: which cells are
grouped together.

### 7

Rows are genes and columns are patient-by-timepoint pseudobulk profiles within a
specified cell type. Metadata contains patient and timepoint; the model uses
patient pairing and tests the post-vs-pre contrast.

### 8

"In this exploratory dataset, a cell population labeled cluster 4 is associated
with response and requires validation in independent patients before predictive
or clinical interpretation."
