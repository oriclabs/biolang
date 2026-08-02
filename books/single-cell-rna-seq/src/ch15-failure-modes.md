# Failure Modes and Review Checklist

## Common attractive mistakes

### "The UMAP has islands, so these are cell types"

UMAP is a visualization of a selected representation. Inspect markers,
technical covariates, graph stability, and reference evidence.

### "There are 10,000 cells, so the study is well replicated"

Count independent donors or experimental units. Cell count and replicate count
answer different power questions.

### "Batch correction mixed the samples, so integration worked"

Perfect mixing can erase biology. Check known controls, marker preservation, and
whether condition is confounded with batch.

### "The smallest cluster is a rare discovery"

It can also be low quality, doublets, ambient RNA, a sample-specific artifact,
or overclustering. Require stronger evidence for rarer claims.

### "A famous marker appeared, so the label is certain"

Use panels, negative evidence, tissue context, references, and uncertainty.

### "Scanpy and Seurat agree, so the biology is proven"

Agreement validates implementation behavior on the tested inputs. The tools can
share assumptions and biases.

## Technical review

- [ ] Input dimensions and orientation are known.
- [ ] Stable gene IDs and symbols are preserved.
- [ ] Raw counts are retained.
- [ ] Sample and donor IDs are attached before merging.
- [ ] QC distributions were inspected per sample.
- [ ] Cells and genes removed at each step are counted.
- [ ] Sparse matrices were not accidentally densified.
- [ ] PCA, `k`, and resolution choices are recorded.
- [ ] Numeric cluster IDs are not treated as biological labels.
- [ ] Outputs join by barcode, not row position alone.
- [ ] Random seeds and software versions are recorded.

## Scientific review

- [ ] The question and experimental unit are explicit.
- [ ] Replicates, pairing, and confounders are represented.
- [ ] Marker annotation uses positive and negative evidence.
- [ ] Doublets and low-quality clusters were considered.
- [ ] Condition DE uses replicate-aware inference.
- [ ] Sensitivity analyses cover reasonable QC and clustering choices.
- [ ] Claims distinguish association, mechanism, and prediction.
- [ ] Independent or orthogonal validation is identified.

## Clinical review

- [ ] The result is labeled discovery, validation, or clinical use.
- [ ] The population matches the intended use population.
- [ ] Performance and uncertainty were measured on held-out data.
- [ ] Privacy, consent, and data access are appropriate.
- [ ] A qualified human reviews any decision-affecting output.
- [ ] No exploratory cluster is presented as a diagnostic category.

## Final question

Could another team recover the same result, understand every major decision,
and identify what evidence would change the conclusion? If not, the analysis is
not finished.
