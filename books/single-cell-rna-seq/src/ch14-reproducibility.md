# Scale, Reproduce, and Report

## Know where memory goes

Real count matrices are mostly zero. BioLang keeps 10x counts and normalized
matrices sparse and computes PCA without materializing a dense centered matrix.
This is important because a dense matrix of one million cells by 30,000 genes
is not a practical laptop object.

Current exact nearest-neighbor search is quadratic in cell count. The graph
itself is sparse, but building exact neighbors becomes the practical limit for
large atlases. Use a representative subset, a validated approximate-neighbor
tool, or remote compute until approximate indexing is available in BioLang.

## Checkpoint intermediate results

Expensive stages should produce named, checksummed artifacts:

```text
results/
  01-qc-summary.tsv
  02-filtered.zarr/
  03-pca-summary.tsv
  04-cell-clusters.tsv
  05-marker-review.tsv
  figures/
  logs/
```

BioLang reads and writes sparse AnnData Zarr `X` plus observation and variable
index names. Arbitrary AnnData metadata columns and auxiliary layers are not yet
fully preserved. Direct `.h5ad` I/O requires external conversion. Verify a
roundtrip before relying on it for archival interchange.

## Record the environment

At minimum save:

- BioLang version and commit;
- package source and version;
- operating system and architecture;
- command line;
- input accession, path, size, and checksum;
- genome and annotation version;
- every non-default parameter;
- Python/R/container versions used for validation;
- random seeds where relevant;
- warnings and failed attempts.

## Separate exploration from final analysis

Exploration is allowed to be iterative. The final workflow should be rerunnable
from immutable inputs and should not depend on clicking cells or manually
editing a result table without an audit trail.

A literate `.bln` notebook can explain decisions and show compact outputs. Keep
the production pipeline in scripts when it needs batch execution, retries, and
large artifacts.

## Remote execution

Large projects may execute on a workstation, scheduler, container platform, or
remote service. Preserve the same logical contract:

```text
source + parameters + input identities -> logs + status + artifacts
```

Do not silently depend on local files that a remote worker cannot access.
Stage inputs explicitly, avoid embedding credentials in scripts, and verify
artifacts after transfer.

## Report decisions, not only pictures

A useful methods section states:

- how cells and genes were filtered;
- how counts were normalized;
- how HVGs and PCs were selected;
- graph and clustering parameters;
- how labels were assigned and reviewed;
- how sample replication entered statistical tests;
- which sensitivity analyses were performed;
- which steps used external tools.
