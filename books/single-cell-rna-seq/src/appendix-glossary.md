# Glossary

**Adjusted Rand index (ARI):** Agreement between two partitions, corrected for
chance and independent of numeric cluster names.

**Ambient RNA:** RNA released into the suspension and captured by droplets that
may not contain the source cell.

**Barcode:** Sequence used to associate molecules with a captured droplet or
cell library.

**Batch:** A group sharing a technical processing event. Batch can be
confounded with biology.

**Cell state:** A relatively temporary program such as activation, stress, or
cell cycle.

**Cell type:** A biological identity supported by lineage, function, markers,
and context. It is not identical to an algorithmic cluster.

**Cluster:** A group produced by an algorithm under selected data and
parameters.

**Count matrix:** Table of observed molecule counts for cells and genes.

**Differential expression:** A statistical comparison of expression between
defined groups under a specified design.

**Doublet:** One barcode representing two captured cells.

**Experimental unit:** The smallest independently assigned or sampled unit that
supports inference, often a donor, animal, organoid, or culture.

**Feature:** A measured item, usually a gene but potentially an antibody tag or
guide.

**Gene:** A genomic region that contributes to a functional RNA or protein
product.

**Highly variable gene (HVG):** Gene selected because its variation is useful
for describing cell structure under a particular method.

**Integration:** Construction of a representation intended to align shared
biology across samples or batches.

**Library size:** Total observed counts for one cell library.

**Marker:** Gene whose expression helps distinguish a group. A marker is
context-dependent and not necessarily unique.

**Mitochondrial fraction:** Fraction of a cell's counts assigned to
mitochondrial genes.

**Neighbor graph:** Graph connecting each cell to similar cells in a selected
representation.

**Normalization:** Transformation intended to make measurements more comparable
for a downstream purpose.

**PCA:** Principal component analysis, a linear low-dimensional representation.

**Pseudobulk:** Counts aggregated across cells within each biological sample and
cell type for replicate-aware analysis.

**Pseudoreplication:** Treating correlated observations, such as cells from one
donor, as independent experimental replicates.

**Pseudotime:** Relative ordering along an inferred cellular continuum. It is
not clock time.

**RNA:** Molecules involved in gene expression and regulation. Messenger RNA is
used as a proxy for transcriptional activity.

**Sparse matrix:** Matrix stored using nonzero entries rather than every zero.

**Transcript:** RNA molecule produced from a gene.

**UMAP:** Nonlinear low-dimensional visualization. Its visual distances are not
calibrated biological distances.

**UMI:** Unique molecular identifier used to reduce duplicate counting of the
same captured molecule.
