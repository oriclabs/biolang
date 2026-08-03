# Design the Study Before the Analysis

## The experimental unit

If three patients contribute 5,000 cells each, the treatment comparison usually
has three biological replicates per group, not 15,000 independent replicates.
Cells from the same person share genetics, environment, collection, and
processing. Treating them as independent is **pseudoreplication**.

This distinction changes the question:

- "Which genes distinguish these two clusters in this dataset?" is exploratory.
- "Which genes respond to treatment across patients?" requires patient-level
  replication and a model that accounts for it.

## Start with a question table

Write this before opening the matrix:

| Field | Example |
|---|---|
| Population | Adults with treatment-resistant disease |
| Comparison | Drug vs matched baseline |
| Biological unit | Patient |
| Tissue | Peripheral blood |
| Primary outcome | State change within CD8 T cells |
| Main confounders | Patient, processing day, sex, disease severity |
| Exclusions | Predefined low viability or failed library |
| Validation | Independent cohort and orthogonal assay |

The table prevents a visually interesting result from silently replacing the
original question.

## Balance biology and processing

If all controls are processed Monday and all treated samples Tuesday, treatment
and processing day are **confounded**. No algorithm can prove which caused the
difference. Randomize or balance samples across processing batches where
possible. Record donor, condition, collection time, chemistry, lane, operator,
and reference version.

## Power is not only cell count

More cells improve the description of within-sample diversity and help detect
rare populations. More independent samples improve inference about a
population. Ten thousand cells from one donor do not substitute for additional
donors when the claim concerns people.

## Clinical and ethical context

Single-cell data can contain germline variants, disease information, and rare
cell states that increase re-identification risk. Keep identifiers separate,
apply appropriate access controls, record consent limitations, and avoid
publishing unrestricted cell-level metadata that can expose a participant.

For clinical questions, define in advance whether the result is discovery,
validation, or decision support. Do not move between those roles without new
evidence and governance.

## Analysis contract

Create a short analysis contract:

```text
Question: Does treatment alter the inflammatory state of monocytes?
Unit: patient
Samples: 8 baseline, 8 treated, paired where available
Cell annotation: blinded marker review plus reference mapping
Primary comparison: monocyte pseudobulk counts, paired design
Sensitivity: alternate QC thresholds and cluster resolutions
Validation: held-out cohort plus flow cytometry panel
```

This plain-language contract is more valuable than a long script with no stated
claim.
