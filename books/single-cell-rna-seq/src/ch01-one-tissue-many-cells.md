# One Tissue, Many Cells

## The everyday analogy

Suppose a hospital asks whether patients are waiting too long. One number for
the whole hospital hides the difference between emergency, radiology, pharmacy,
and reception. Measuring each department separately reveals where the delay
occurs.

Bulk RNA-seq is the hospital-wide average. Single-cell RNA-seq is closer to
measuring each room separately. It can reveal a rare population, a changing
cell state, or a different mixture of cells that the average hides.

## What RNA tells us

DNA is a long-term instruction store. A **gene** is a named region that can
contribute to a biological product. When a gene is active, the cell can make RNA
copies called **transcripts**. Messenger RNA is not a direct measurement of
protein, behavior, or disease, but its abundance is evidence about what a cell
was doing when it was captured.

Single-cell RNA-seq asks: which RNA molecules were observed in each captured
cell or nucleus?

The usual result is a matrix:

| | Gene A | Gene B | Gene C |
|---|---:|---:|---:|
| Cell 1 | 0 | 4 | 1 |
| Cell 2 | 0 | 5 | 0 |
| Cell 3 | 7 | 0 | 2 |

Rows are cells, columns are genes, and entries are observed molecule counts.
Most entries are zero, so the matrix is **sparse**.

## Questions it can answer

Single-cell analysis is useful when the variation among cells matters:

- Which cell populations are present in a tumor?
- Does treatment change a cell type's abundance or state?
- Which immune population expresses an inflammatory program?
- What intermediate states appear during development?
- Which cells respond to infection?

It is less useful when the scientific question concerns only a well-purified,
uniform population or when sample replication is too weak to support the
comparison.

## Four viewpoints

**The biologist** asks whether the populations and markers make biological
sense. **The computational researcher** asks whether processing choices created
the pattern. **The programmer** asks whether data structures and algorithms
scale. **The clinician** asks whether the finding was replicated and whether it
changes a validated decision.

All four viewpoints are needed. A technically correct cluster can still be a
doublet. A plausible marker can still be caused by batch. A statistically
significant gene can still be clinically irrelevant.

## Checkpoint

Before continuing, explain this sentence in your own words:

> Single-cell RNA-seq measures noisy evidence about cell state, not a complete
> inventory of everything a cell contains or does.
